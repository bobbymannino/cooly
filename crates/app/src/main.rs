use gpui_kit::{KeyBinding, actions};

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

        if let Err(error) = cooly_ui::open_login_window(cx) {
            eprintln!("failed to open window: {error:?}");
            cx.quit();
        }
        cx.activate(true);
    });
}
