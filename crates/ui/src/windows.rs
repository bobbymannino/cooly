//! Opening and swapping the app's windows.

use std::sync::Arc;

use cooly_api::{Client, Credentials, Team};
use cooly_store::{Appearance, Store};
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Bounds, Size, TitlebarOptions, WindowBounds, WindowOptions,
    component::{Theme, ThemeMode},
    px, size,
};

use crate::views::{DashboardEvent, DashboardView, LoginEvent, LoginView};

const LOGIN_SIZE: Size<gpui_kit::Pixels> = size(px(380.0), px(460.0));
const MAIN_SIZE: Size<gpui_kit::Pixels> = size(px(960.0), px(640.0));
const MAIN_MIN_SIZE: Size<gpui_kit::Pixels> = size(px(480.0), px(480.0));

/// Applies saved preferences and opens the login window, which signs in automatically with saved credentials.
///
/// Unreadable preferences or credentials are logged and ignored rather than blocking startup.
///
/// # Errors
///
/// Returns an error if the platform fails to open the window.
pub fn launch(store: Arc<Store>, cx: &mut App) -> gpui_kit::Result<()> {
    let preferences = store.preferences().unwrap_or_else(|error| {
        eprintln!("ignoring preferences: {error}");
        cooly_store::Preferences::default()
    });
    apply_appearance(preferences.appearance, cx);

    let saved = store.credentials().unwrap_or_else(|error| {
        eprintln!("ignoring saved credentials: {error}");
        None
    });
    open_login_window(store, saved, cx)
}

fn apply_appearance(appearance: Appearance, cx: &mut App) {
    match appearance {
        Appearance::System => Theme::sync_system_appearance(None, cx),
        Appearance::Light => Theme::change(ThemeMode::Light, None, cx),
        Appearance::Dark => Theme::change(ThemeMode::Dark, None, cx),
    }
}

/// Opens the fixed-size, chromeless login window. Signing in replaces it with the main window.
fn open_login_window(store: Arc<Store>, saved: Option<Credentials>, cx: &mut App) -> gpui_kit::Result<()> {
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Cooly".into()),
            appears_transparent: true,
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, LOGIN_SIZE, cx))),
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };
    let (handle, login) = gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| LoginView::new(store.clone(), saved, window, cx))
    })?;

    cx.subscribe(&login, move |_, event, cx| match event {
        LoginEvent::SignedIn { client, team } => {
            replace_window(handle, cx, |cx| {
                open_main_window(store.clone(), client.clone(), team.clone(), cx)
            });
        }
    })
    .detach();
    Ok(())
}

fn open_main_window(store: Arc<Store>, client: Client, team: Team, cx: &mut App) -> gpui_kit::Result<()> {
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Cooly".into()),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, MAIN_SIZE, cx))),
        window_min_size: Some(MAIN_MIN_SIZE),
        ..Default::default()
    };
    let (handle, dashboard) = gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| DashboardView::new(client, team)))?;

    cx.subscribe(&dashboard, move |_, event, cx| match event {
        DashboardEvent::SignedOut => {
            if let Err(error) = store.clear_credentials() {
                eprintln!("failed to forget credentials: {error}");
            }
            replace_window(handle, cx, |cx| open_login_window(store.clone(), None, cx));
        }
    })
    .detach();
    Ok(())
}

/// Opens a new window, then closes `old`. The new window opens first so the app never has zero windows and quits.
fn replace_window(old: AnyWindowHandle, cx: &mut App, open: impl FnOnce(&mut App) -> gpui_kit::Result<()>) {
    if let Err(error) = open(cx) {
        eprintln!("failed to open window: {error:?}");
        return;
    }
    if let Err(error) = old.update(cx, |_, window, _| window.remove_window()) {
        eprintln!("failed to close window: {error:?}");
    }
}
