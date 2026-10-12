//! The app's menu bar.
//!
//! Items run the same actions as the shortcuts, and the platform disables any item the focused window doesn't handle,
//! so the dashboard's items are greyed out while signing in.

use cooly_store::ProjectsLayout;
use gpui_kit::{App, Menu, MenuItem, actions};

use crate::views::{Refresh, ShowGrid, ShowList, SignOut};

actions!(
    cooly,
    [
        #[derive(Eq)]
        Quit
    ]
);

/// Replaces the menu bar, checking the item for the current `layout`.
///
/// Shortcuts are read from the keymap now, so bind keys first. Call again whenever the layout changes, since menu
/// items don't update on their own.
pub fn set_menus(layout: ProjectsLayout, cx: &App) {
    cx.set_menus([
        Menu::new("Cooly").items([
            MenuItem::action("Sign Out", SignOut),
            MenuItem::separator(),
            MenuItem::action("Quit Cooly", Quit),
        ]),
        Menu::new("View").items([
            MenuItem::action("Refresh", Refresh),
            MenuItem::separator(),
            MenuItem::action("as Grid", ShowGrid).checked(layout == ProjectsLayout::Grid),
            MenuItem::action("as List", ShowList).checked(layout == ProjectsLayout::List),
        ]),
    ]);
}
