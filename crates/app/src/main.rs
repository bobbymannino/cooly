use std::sync::Arc;

use cooly_store::Store;
use cooly_ui::Quit;
use gpui_kit::KeyBinding;

fn main() {
    gpui_kit::application().with_assets(cooly_ui::Assets).run(|cx| {
        gpui_kit::init(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let store = match Store::open() {
            Ok(store) => Arc::new(store),
            Err(error) => {
                eprintln!("failed to open data store: {error}");
                cx.quit();
                return;
            }
        };
        if let Err(error) = cooly_ui::launch(store, cx) {
            eprintln!("failed to open window: {error:?}");
            cx.quit();
        }
        cx.activate(true);
    });
}
