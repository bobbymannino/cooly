//! Local, offline storage for Cooly.
//!
//! Everything lives in one directory on this machine and nothing is sent over the network:
//!
//! - `preferences.json`: plain JSON, safe to inspect or edit by hand.
//! - `credentials.enc`: encrypted with XChaCha20-Poly1305. The 256-bit key is generated on first use and kept only in
//!   the OS credential store (Keychain on macOS), never on disk.

mod error;
mod fs;
mod key;
mod preferences;
mod vault;

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use cooly_api::Credentials;
use directories::ProjectDirs;
pub use error::StoreError;
pub use key::{KEY_LEN, KeySource, Keychain};
pub use preferences::{Appearance, Preferences};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const PREFERENCES_FILE: &str = "preferences.json";
const CREDENTIALS_FILE: &str = "credentials.enc";

/// Reads and writes Cooly's data on disk.
pub struct Store {
    dir: PathBuf,
    keys: Box<dyn KeySource>,
}

impl Store {
    /// Opens the store in the platform's local (non-roaming) data directory, with the key in the OS keychain.
    ///
    /// Nothing is read or created until it's needed, so this doesn't touch the keychain.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NoDataDir`] if the platform has no home directory.
    pub fn open() -> Result<Self, StoreError> {
        let dirs = ProjectDirs::from("", "", "Cooly").ok_or(StoreError::NoDataDir)?;
        Ok(Self::new(dirs.data_local_dir(), Keychain))
    }

    /// Opens a store in `dir` that gets its encryption key from `keys`.
    pub fn new(dir: impl Into<PathBuf>, keys: impl KeySource + 'static) -> Self {
        Self {
            dir: dir.into(),
            keys: Box::new(keys),
        }
    }

    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Loads preferences, or the defaults if none have been saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but can't be read or parsed.
    pub fn preferences(&self) -> Result<Preferences, StoreError> {
        match fs::read(&self.dir.join(PREFERENCES_FILE))? {
            Some(bytes) => Ok(serde_json::from_slice(&bytes)?),
            None => Ok(Preferences::default()),
        }
    }

    /// # Errors
    ///
    /// Returns an error if the file can't be written.
    pub fn save_preferences(&self, preferences: &Preferences) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec_pretty(preferences)?;
        fs::write_private(&self.dir, PREFERENCES_FILE, &bytes)
    }

    /// Loads and decrypts saved credentials, if any. The keychain is only consulted when a credentials file exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the file can't be read, the key is missing or wrong, or the contents were tampered with.
    pub fn credentials(&self) -> Result<Option<Credentials>, StoreError> {
        let Some(sealed) = fs::read(&self.dir.join(CREDENTIALS_FILE))? else {
            return Ok(None);
        };
        let key = self.keys.load()?.ok_or(StoreError::Undecryptable)?;
        let plaintext = vault::open(&key, CREDENTIALS_FILE, &sealed)?;
        let saved: SavedCredentials<'_> = serde_json::from_slice(&plaintext)?;
        // Re-validate rather than trusting the file, so stored data can't bypass `Credentials` invariants.
        Ok(Some(Credentials::parse(&saved.instance, &saved.token)?))
    }

    /// Encrypts and saves credentials, creating the encryption key on first use.
    ///
    /// # Errors
    ///
    /// Returns an error if the key can't be loaded or created, or the file can't be written.
    pub fn save_credentials(&self, credentials: &Credentials) -> Result<(), StoreError> {
        let key = self.keys.load_or_create()?;
        let plaintext = Zeroizing::new(serde_json::to_vec(&SavedCredentials {
            instance: credentials.instance().as_str().into(),
            token: credentials.token().into(),
        })?);
        let sealed = vault::seal(&key, CREDENTIALS_FILE, &plaintext)?;
        fs::write_private(&self.dir, CREDENTIALS_FILE, &sealed)
    }

    /// Deletes saved credentials. The encryption key is kept for the next sign-in.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but can't be removed.
    pub fn clear_credentials(&self) -> Result<(), StoreError> {
        fs::remove(&self.dir.join(CREDENTIALS_FILE))
    }
}

/// The on-disk (pre-encryption) shape of [`Credentials`].
#[derive(Serialize, Deserialize)]
/// Borrows from the input where possible, so usually the only plaintext copy is the zeroized buffer.
struct SavedCredentials<'a> {
    #[serde(borrow)]
    instance: Cow<'a, str>,
    #[serde(borrow)]
    token: Cow<'a, str>,
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tempfile::TempDir;
    use zeroize::Zeroizing;

    use super::*;

    /// An in-memory stand-in for the keychain. Clones share the same key.
    #[derive(Clone, Default)]
    struct MemoryKey(Arc<Mutex<Option<[u8; KEY_LEN]>>>);

    impl KeySource for MemoryKey {
        fn load(&self) -> Result<Option<Zeroizing<[u8; KEY_LEN]>>, StoreError> {
            Ok(self.0.lock().unwrap().map(Zeroizing::new))
        }

        fn load_or_create(&self) -> Result<Zeroizing<[u8; KEY_LEN]>, StoreError> {
            let mut key = self.0.lock().unwrap();
            Ok(Zeroizing::new(*key.get_or_insert_with(|| *key::generate().unwrap())))
        }
    }

    fn store() -> (TempDir, Store, MemoryKey) {
        let dir = TempDir::new().unwrap();
        let keys = MemoryKey::default();
        let store = Store::new(dir.path(), keys.clone());
        (dir, store, keys)
    }

    fn credentials() -> Credentials {
        Credentials::parse("https://coolify.example.com", "1|super-secret-token").unwrap()
    }

    #[test]
    fn preferences_default_then_round_trip() {
        let (_dir, store, _) = store();
        assert_eq!(store.preferences().unwrap(), Preferences::default());

        let preferences = Preferences {
            appearance: Appearance::Dark,
        };
        store.save_preferences(&preferences).unwrap();
        assert_eq!(store.preferences().unwrap(), preferences);
    }

    #[test]
    fn preferences_tolerate_missing_and_unknown_fields() {
        let (dir, store, _) = store();
        std::fs::write(dir.path().join(PREFERENCES_FILE), r#"{"from_the_future":1}"#).unwrap();
        assert_eq!(store.preferences().unwrap(), Preferences::default());
    }

    #[test]
    fn credentials_round_trip_and_clear() {
        let (_dir, store, keys) = store();
        assert_eq!(store.credentials().unwrap(), None);
        assert!(keys.load().unwrap().is_none(), "reading nothing shouldn't create a key");

        store.save_credentials(&credentials()).unwrap();
        assert_eq!(store.credentials().unwrap(), Some(credentials()));

        store.clear_credentials().unwrap();
        assert_eq!(store.credentials().unwrap(), None);
        store.clear_credentials().unwrap();
    }

    #[test]
    fn credentials_are_encrypted_on_disk() {
        let (dir, store, _) = store();
        store.save_credentials(&credentials()).unwrap();

        let on_disk = std::fs::read(dir.path().join(CREDENTIALS_FILE)).unwrap();
        let on_disk = String::from_utf8_lossy(&on_disk);
        assert!(!on_disk.contains("super-secret-token"));
        assert!(!on_disk.contains("coolify.example.com"));
    }

    #[test]
    fn saving_twice_uses_fresh_nonces() {
        let (dir, store, _) = store();
        store.save_credentials(&credentials()).unwrap();
        let first = std::fs::read(dir.path().join(CREDENTIALS_FILE)).unwrap();
        store.save_credentials(&credentials()).unwrap();
        let second = std::fs::read(dir.path().join(CREDENTIALS_FILE)).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn wrong_or_missing_key_is_undecryptable() {
        let (dir, store, _) = store();
        store.save_credentials(&credentials()).unwrap();

        let missing = Store::new(dir.path(), MemoryKey::default());
        assert!(matches!(missing.credentials(), Err(StoreError::Undecryptable)));

        let different = MemoryKey::default();
        different.load_or_create().unwrap();
        assert!(matches!(
            Store::new(dir.path(), different).credentials(),
            Err(StoreError::Undecryptable)
        ));
    }

    #[test]
    fn tampering_is_detected() {
        let (dir, store, _) = store();
        store.save_credentials(&credentials()).unwrap();

        let path = dir.path().join(CREDENTIALS_FILE);
        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last ^= 1;
        std::fs::write(&path, bytes).unwrap();

        assert!(matches!(store.credentials(), Err(StoreError::Undecryptable)));
    }

    #[cfg(unix)]
    #[test]
    fn files_are_private() {
        use std::os::unix::fs::PermissionsExt as _;

        let (dir, store, _) = store();
        store.save_credentials(&credentials()).unwrap();
        store.save_preferences(&Preferences::default()).unwrap();

        for file in [CREDENTIALS_FILE, PREFERENCES_FILE] {
            let mode = std::fs::metadata(dir.path().join(file)).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "{file}");
        }
    }
}
