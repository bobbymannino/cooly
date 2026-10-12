//! Resources returned by the Coolify API. Fields Cooly doesn't use are ignored, so additions to the API don't break it.

use serde::Deserialize;

/// The team an API token is bound to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Team {
    pub id: u64,
    pub name: String,
}

/// A project: a group of environments holding applications, databases, and services.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Project {
    pub uuid: String,
    pub name: String,
    /// `null` or absent when the project has no description.
    #[serde(default)]
    pub description: Option<String>,
}
