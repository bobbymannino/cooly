use cooly_ui::CoolyApp;
use gpui_kit::{
    App, AppContext as _, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, actions, px, size,
};

actions!(
    cooly,
    [
        #[derive(Eq)]
        Quit
    ]
);

fn main() {
    gpui_kit::application().run(|cx| {
        gpui_kit::init(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        if let Err(error) = open_main_window(cx) {
            eprintln!("failed to open window: {error:?}");
            cx.quit();
        }
        cx.activate(true);
    });
}

fn open_main_window(cx: &mut App) -> gpui_kit::Result<()> {
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Cooly".into()),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(960.), px(640.)),
            cx,
        ))),
        window_min_size: Some(size(px(480.), px(480.))),
        ..Default::default()
    };
    gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| CoolyApp::new(window, cx)))?;
    Ok(())
}
