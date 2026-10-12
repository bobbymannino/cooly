use std::{fmt, io};

use cooly_api::CredentialsError;

/// Why reading or writing the store failed.
#[derive(Debug)]
pub enum StoreError {
    /// The platform has no home or data directory.
    NoDataDir,
    Io(io::Error),
    /// A file's contents aren't the JSON we expect.
    Format(serde_json::Error),
    /// The OS credential store couldn't be read or written.
    Keychain(keyring::Error),
    /// The OS couldn't provide secure random bytes.
    Random,
    /// The encryption key is missing or wrong, or the encrypted file was tampered with.
    Undecryptable,
    /// Decrypted credentials no longer pass validation.
    InvalidCredentials(CredentialsError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDataDir => f.write_str("no data directory is available on this system"),
            Self::Io(error) => write!(f, "file error: {error}"),
            Self::Format(error) => write!(f, "malformed data: {error}"),
            Self::Keychain(error) => write!(f, "keychain error: {error}"),
            Self::Random => f.write_str("the system random number generator failed"),
            Self::Undecryptable => f.write_str("saved credentials can't be decrypted"),
            Self::InvalidCredentials(error) => write!(f, "saved credentials are invalid: {error}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Format(error) => Some(error),
            Self::Keychain(error) => Some(error),
            Self::InvalidCredentials(error) => Some(error),
            Self::NoDataDir | Self::Random | Self::Undecryptable => None,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Format(error)
    }
}

impl From<keyring::Error> for StoreError {
    fn from(error: keyring::Error) -> Self {
        Self::Keychain(error)
    }
}

impl From<CredentialsError> for StoreError {
    fn from(error: CredentialsError) -> Self {
        Self::InvalidCredentials(error)
    }
}
