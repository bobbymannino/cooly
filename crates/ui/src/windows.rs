//! Opening and swapping the app's windows.

use cooly_api::{Client, Team};
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Bounds, Size, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};

use crate::views::{DashboardEvent, DashboardView, LoginEvent, LoginView};

const LOGIN_SIZE: Size<gpui_kit::Pixels> = size(px(380.0), px(460.0));
const MAIN_SIZE: Size<gpui_kit::Pixels> = size(px(960.0), px(640.0));
const MAIN_MIN_SIZE: Size<gpui_kit::Pixels> = size(px(480.0), px(480.0));

/// Opens the fixed-size, chromeless login window. Signing in replaces it with the main window.
///
/// # Errors
///
/// Returns an error if the platform fails to open the window.
pub fn open_login_window(cx: &mut App) -> gpui_kit::Result<()> {
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
    let (handle, login) = gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| LoginView::new(window, cx)))?;

    cx.subscribe(&login, move |_, event, cx| match event {
        LoginEvent::SignedIn { client, team } => {
            replace_window(handle, cx, |cx| open_main_window(client.clone(), team.clone(), cx));
        }
    })
    .detach();
    Ok(())
}

fn open_main_window(client: Client, team: Team, cx: &mut App) -> gpui_kit::Result<()> {
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
        DashboardEvent::SignedOut => replace_window(handle, cx, open_login_window),
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
