use chacha20poly1305::{Key, aead::Generate as _};
use zeroize::Zeroizing;

use crate::StoreError;

/// Length in bytes of the store's encryption key.
pub const KEY_LEN: usize = 32;

const KEYCHAIN_SERVICE: &str = "Cooly";
const KEYCHAIN_ACCOUNT: &str = "store-encryption-key";

/// Where the store's encryption key lives.
pub trait KeySource: Send + Sync {
    /// Returns the key, or `None` if it hasn't been created yet.
    ///
    /// # Errors
    ///
    /// Returns an error if the key exists but can't be read.
    fn load(&self) -> Result<Option<Zeroizing<[u8; KEY_LEN]>>, StoreError>;

    /// Returns the key, generating and saving a new random one if none exists.
    ///
    /// # Errors
    ///
    /// Returns an error if the key can't be read, generated, or saved.
    fn load_or_create(&self) -> Result<Zeroizing<[u8; KEY_LEN]>, StoreError>;
}

/// Keeps the key in the OS credential store: Keychain on macOS, Credential Manager on Windows, Secret Service on Linux.
pub struct Keychain;

impl Keychain {
    fn entry() -> Result<keyring::Entry, StoreError> {
        Ok(keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)?)
    }
}

impl KeySource for Keychain {
    fn load(&self) -> Result<Option<Zeroizing<[u8; KEY_LEN]>>, StoreError> {
        match Self::entry()?.get_secret() {
            Ok(secret) => {
                let secret = Zeroizing::new(secret);
                let key = <[u8; KEY_LEN]>::try_from(secret.as_slice()).map_err(|_| StoreError::Undecryptable)?;
                Ok(Some(Zeroizing::new(key)))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn load_or_create(&self) -> Result<Zeroizing<[u8; KEY_LEN]>, StoreError> {
        if let Some(key) = self.load()? {
            return Ok(key);
        }
        let key = generate()?;
        Self::entry()?.set_secret(key.as_slice())?;
        Ok(key)
    }
}

/// Generates a new random key from the OS's secure random number generator.
pub fn generate() -> Result<Zeroizing<[u8; KEY_LEN]>, StoreError> {
    let key = Key::try_generate().map_err(|_| StoreError::Random)?;
    Ok(Zeroizing::new(key.into()))
}
