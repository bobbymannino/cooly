use cooly_api::{ApiError, Client, Team};
use gpui_kit::{
    Context, EventEmitter, IntoElement, ParentElement as _, Render, SharedString, Styled as _, Task, Window,
    component::{ActiveTheme as _, StyledExt as _, button::Button, h_flex, v_flex},
    div,
    prelude::FluentBuilder as _,
};

pub enum DashboardEvent {
    SignedOut,
    /// The instance rejected the token, e.g. because it was revoked since it was saved.
    SessionExpired(SharedString),
}

/// Placeholder for the signed-in experience.
pub struct DashboardView {
    client: Client,
    team: Option<Team>,
    error: Option<SharedString>,
    /// The in-flight team fetch. Dropping it cancels the fetch.
    loading: Option<Task<()>>,
}

impl EventEmitter<DashboardEvent> for DashboardView {}

impl DashboardView {
    /// Without a `team` (e.g. when restoring a saved session) it's fetched, which also re-checks the token.
    #[must_use]
    pub fn new(client: Client, team: Option<Team>, cx: &Context<Self>) -> Self {
        let mut this = Self {
            client,
            team,
            error: None,
            loading: None,
        };
        if this.team.is_none() {
            this.load_team(cx);
        }
        this
    }

    fn load_team(&mut self, cx: &Context<Self>) {
        let client = self.client.clone();
        let fetch = cx.background_executor().spawn(async move { client.current_team() });
        self.loading = Some(cx.spawn(async move |this, cx| {
            let result = fetch.await;
            // Fails only if the view was closed meanwhile, in which case there's nothing left to update.
            this.update(cx, |this, cx| {
                this.loading = None;
                match result {
                    Ok(team) => this.team = Some(team),
                    Err(error @ ApiError::InvalidToken) => {
                        cx.emit(DashboardEvent::SessionExpired(error.to_string().into()));
                    }
                    Err(error) => this.error = Some(error.to_string().into()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn retry(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.load_team(cx);
        cx.notify();
    }
}

impl Render for DashboardView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let title = self
            .team
            .as_ref()
            .map_or_else(|| SharedString::from("Connecting…"), |team| team.name.clone().into());

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().text_xl().font_semibold().child(title))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(self.client.credentials().instance().to_string()),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(div().text_sm().text_color(theme.danger).child(error))
            })
            .child(
                h_flex()
                    .gap_2()
                    .when(self.error.is_some(), |this| {
                        this.child(
                            Button::new("retry")
                                .label("Retry")
                                .on_click(cx.listener(|this, _, _, cx| this.retry(cx))),
                        )
                    })
                    .child(
                        Button::new("sign-out")
                            .label("Sign out")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(DashboardEvent::SignedOut))),
                    ),
            )
    }
}
