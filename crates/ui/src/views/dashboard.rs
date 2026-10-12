use cooly_api::Credentials;
use gpui_kit::{
    Context, EventEmitter, IntoElement, ParentElement as _, Render, Styled as _, Window,
    component::{ActiveTheme as _, StyledExt as _, button::Button, v_flex},
    div,
};

pub enum DashboardEvent {
    SignedOut,
}

/// Placeholder for the signed-in experience.
pub struct DashboardView {
    credentials: Credentials,
}

impl EventEmitter<DashboardEvent> for DashboardView {}

impl DashboardView {
    #[must_use]
    pub const fn new(credentials: Credentials) -> Self {
        Self { credentials }
    }
}

impl Render for DashboardView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(div().text_xl().font_semibold().child("Dashboard"))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.credentials.instance().to_string()),
            )
            .child(
                Button::new("sign-out")
                    .label("Sign out")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(DashboardEvent::SignedOut))),
            )
    }
}
