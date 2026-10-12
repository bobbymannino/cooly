//! Views and windows for the Cooly desktop app.

mod api_task;
mod assets;
pub mod views;
#[cfg(test)]
mod test_support;
mod windows;

pub use assets::Assets;
pub use windows::launch;
