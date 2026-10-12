use std::fmt;

use url::Url;

/// What is needed to authenticate against a Coolify instance's API.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    instance: Url,
    token: String,
}

impl Credentials {
    /// Builds credentials from raw user input.
    ///
    /// The instance URL defaults to `https://` when no scheme is given.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL is missing or invalid, or the token is empty.
    pub fn parse(instance: &str, token: &str) -> Result<Self, CredentialsError> {
        let instance = instance.trim();
        if instance.is_empty() {
            return Err(CredentialsError::MissingInstance);
        }
        let instance = if instance.contains("://") {
            Url::parse(instance)
        } else {
            Url::parse(&format!("https://{instance}"))
        }
        .map_err(|_| CredentialsError::InvalidInstance)?;
        if !matches!(instance.scheme(), "http" | "https") || instance.host().is_none() {
            return Err(CredentialsError::InvalidInstance);
        }

        let token = token.trim();
        if token.is_empty() {
            return Err(CredentialsError::MissingToken);
        }

        Ok(Self {
            instance,
            token: token.to_owned(),
        })
    }

    /// The base URL of the Coolify instance.
    #[must_use]
    pub const fn instance(&self) -> &Url {
        &self.instance
    }

    /// The API token, sent as a bearer token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("instance", &self.instance.as_str())
            .field("token", &"<redacted>")
            .finish()
    }
}

/// Why user-supplied credentials were rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialsError {
    MissingInstance,
    InvalidInstance,
    MissingToken,
}

impl fmt::Display for CredentialsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingInstance => "Enter the URL of your Coolify instance.",
            Self::InvalidInstance => "Enter a valid http(s) URL, e.g. https://coolify.example.com.",
            Self::MissingToken => "Enter an API token.",
        })
    }
}

impl std::error::Error for CredentialsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_https() {
        let credentials = Credentials::parse("coolify.example.com", "abc").unwrap();
        assert_eq!(credentials.instance().as_str(), "https://coolify.example.com/");
    }

    #[test]
    fn keeps_explicit_scheme_and_trims_input() {
        let credentials = Credentials::parse("  http://10.0.0.2:8000  ", " abc ").unwrap();
        assert_eq!(credentials.instance().as_str(), "http://10.0.0.2:8000/");
        assert_eq!(credentials.token(), "abc");
    }

    #[test]
    fn rejects_bad_input() {
        assert_eq!(Credentials::parse("", "abc"), Err(CredentialsError::MissingInstance));
        assert_eq!(
            Credentials::parse("ftp://example.com", "abc"),
            Err(CredentialsError::InvalidInstance)
        );
        assert_eq!(
            Credentials::parse("https://", "abc"),
            Err(CredentialsError::InvalidInstance)
        );
        assert_eq!(
            Credentials::parse("example.com", "  "),
            Err(CredentialsError::MissingToken)
        );
    }

    #[test]
    fn debug_redacts_token() {
        let credentials = Credentials::parse("example.com", "secret").unwrap();
        assert!(!format!("{credentials:?}").contains("secret"));
    }
}
