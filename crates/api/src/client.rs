use std::{fmt, time::Duration};

use serde::de::DeserializeOwned;
use ureq::{
    Agent,
    http::StatusCode,
    tls::{RootCerts, TlsConfig},
};

use crate::{Credentials, Project, Team};

const TIMEOUT: Duration = Duration::from_secs(15);

/// A blocking client for a Coolify instance's API. Run its calls off the UI thread.
#[derive(Clone)]
pub struct Client {
    agent: Agent,
    credentials: Credentials,
}

impl Client {
    #[must_use]
    pub fn new(credentials: Credentials) -> Self {
        let agent = Agent::config_builder()
            // Statuses are mapped to `ApiError` by hand so 401 and 403 can be told apart.
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .user_agent(concat!("cooly/", env!("CARGO_PKG_VERSION")))
            // Trust the OS certificate store so self-hosted instances behind a private CA work.
            .tls_config(TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build())
            .build()
            .into();
        Self { agent, credentials }
    }

    #[must_use]
    pub const fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    /// Fetches the team the API token belongs to: `GET /api/v1/teams/current`.
    ///
    /// Coolify recommends this endpoint for checking that a token is valid for an instance.
    ///
    /// # Errors
    ///
    /// See [`ApiError`].
    pub fn current_team(&self) -> Result<Team, ApiError> {
        self.get("teams/current")
    }

    /// Lists the projects of the token's team: `GET /api/v1/projects`.
    ///
    /// # Errors
    ///
    /// See [`ApiError`].
    pub fn projects(&self) -> Result<Vec<Project>, ApiError> {
        self.get("projects")
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let url = format!("{}api/v1/{path}", self.credentials.instance());
        let mut response = self
            .agent
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.credentials.token()))
            // Without this Laravel answers unauthenticated requests with a redirect to the login page.
            .header("Accept", "application/json")
            .call()?;

        match response.status() {
            status if status.is_success() => Ok(response.body_mut().read_json()?),
            StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED => Err(ApiError::InvalidToken),
            StatusCode::FORBIDDEN => Err(ApiError::Forbidden),
            StatusCode::NOT_FOUND => Err(ApiError::NotCoolify),
            status => Err(ApiError::UnexpectedStatus(status.as_u16())),
        }
    }
}

/// Why a request to the Coolify API failed.
#[derive(Debug)]
pub enum ApiError {
    /// The token is missing, invalid, expired, or revoked (HTTP 400 or 401).
    InvalidToken,
    /// API access is disabled, the caller's IP isn't allowed, or the token lacks permission (HTTP 403).
    Forbidden,
    /// The URL answered, but not like a Coolify API: 404 or a body that isn't the expected JSON.
    NotCoolify,
    UnexpectedStatus(u16),
    /// DNS, connection, TLS, or timeout failure.
    Network(ureq::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken => f.write_str("The API token is invalid, expired, or revoked."),
            Self::Forbidden => f.write_str(
                "Access denied. Check that API access is enabled, your IP is allowed, and the token can read.",
            ),
            Self::NotCoolify => f.write_str("That URL doesn't look like a Coolify instance."),
            Self::UnexpectedStatus(status) => write!(f, "The instance responded with HTTP {status}."),
            Self::Network(error) => write!(f, "Couldn't reach the instance: {error}"),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Network(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ureq::Error> for ApiError {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::Json(_) => Self::NotCoolify,
            error => Self::Network(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read as _, Write as _},
        net::TcpListener,
        thread::{self, JoinHandle},
    };

    use super::*;

    /// Serves a single HTTP response on localhost and returns the raw request it received.
    fn serve(status: &str, body: &str) -> (Client, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: \
             close\r\n\r\n{body}",
            body.len()
        );
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8(request).unwrap()
        });
        let credentials = Credentials::parse(&format!("http://{address}"), "1|secret").unwrap();
        (Client::new(credentials), server)
    }

    #[test]
    fn current_team_sends_bearer_token() {
        let (client, server) = serve("200 OK", r#"{"id":0,"name":"Root Team","personal_team":false}"#);

        let team = client.current_team().unwrap();

        assert_eq!(
            team,
            Team {
                id: 0,
                name: "Root Team".to_owned()
            }
        );
        let request = server.join().unwrap();
        assert!(request.starts_with("GET /api/v1/teams/current HTTP/1.1\r\n"));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer 1|secret\r\n")
        );
    }

    #[test]
    fn projects_parses_list() {
        let (client, server) = serve(
            "200 OK",
            r#"[
                {"id":1,"uuid":"abc","description":"Main site","name":"Website"},
                {"id":2,"uuid":"def","description":null,"name":"Internal"},
                {"uuid":"ghi","name":"Bare"}
            ]"#,
        );

        let projects = client.projects().unwrap();

        assert_eq!(
            projects,
            [
                Project {
                    uuid: "abc".to_owned(),
                    name: "Website".to_owned(),
                    description: Some("Main site".to_owned()),
                },
                Project {
                    uuid: "def".to_owned(),
                    name: "Internal".to_owned(),
                    description: None,
                },
                Project {
                    uuid: "ghi".to_owned(),
                    name: "Bare".to_owned(),
                    description: None,
                },
            ]
        );
        assert!(server.join().unwrap().starts_with("GET /api/v1/projects HTTP/1.1\r\n"));
    }

    #[test]
    fn maps_error_statuses() {
        let cases = [
            ("400 Bad Request", r#"{"message":"Invalid token."}"#),
            ("401 Unauthorized", r#"{"message":"Unauthenticated."}"#),
            (
                "403 Forbidden",
                r#"{"message":"You are not allowed to access the API."}"#,
            ),
            ("404 Not Found", ""),
            ("500 Internal Server Error", ""),
        ];
        let errors = cases.map(|(status, body)| {
            let (client, server) = serve(status, body);
            let error = client.current_team().unwrap_err();
            server.join().unwrap();
            error
        });

        assert!(matches!(
            errors,
            [
                ApiError::InvalidToken,
                ApiError::InvalidToken,
                ApiError::Forbidden,
                ApiError::NotCoolify,
                ApiError::UnexpectedStatus(500),
            ]
        ));
    }

    #[test]
    fn rejects_non_json_success() {
        let (client, server) = serve("200 OK", "<html>not coolify</html>");
        assert!(matches!(client.current_team(), Err(ApiError::NotCoolify)));
        server.join().unwrap();
    }

    #[test]
    fn reports_unreachable_instance() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let client = Client::new(Credentials::parse(&format!("http://{address}"), "token").unwrap());

        assert!(matches!(client.current_team(), Err(ApiError::Network(_))));
    }
}
