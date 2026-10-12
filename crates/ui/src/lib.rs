//! Views and windows for the Cooly desktop app.

mod api_task;
mod assets;
mod menus;
#[cfg(test)]
mod test_support;
pub mod views;
mod windows;

pub use assets::Assets;
pub use menus::Quit;
pub use windows::launch;
