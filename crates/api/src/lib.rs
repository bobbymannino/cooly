//! Types and client for talking to a Coolify instance.

mod client;
mod credentials;

pub use client::{ApiError, Client, Team};
pub use credentials::{Credentials, CredentialsError};
