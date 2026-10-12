use serde::{Deserialize, Serialize};

/// User preferences. Unknown fields are ignored and missing ones take their defaults, so older and newer versions of
/// the app can share a file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub appearance: Appearance,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    /// Match the OS light/dark setting.
    #[default]
    System,
    Light,
    Dark,
}
