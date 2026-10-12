//! Types and client for talking to a Coolify instance.

mod client;
mod credentials;
mod models;

pub use client::{ApiError, Client};
pub use credentials::{Credentials, CredentialsError};
pub use models::{Project, Team};
