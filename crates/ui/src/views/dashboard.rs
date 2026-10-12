use std::sync::Arc;

use cooly_api::{ApiError, Client, Team};
use cooly_store::Store;
use gpui_kit::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, IntoElement, KeyBinding, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Task, Window, actions,
    assets::IconName,
    component::{
        ActiveTheme as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex, v_flex,
    },
    div,
    prelude::{FluentBuilder as _, InteractiveElement as _},
};

use crate::{
    api_task::spawn_api_call,
    views::{ProjectsEvent, ProjectsView},
};

actions!(
    dashboard,
    [
        /// Reloads the team's projects.
        #[derive(Eq)]
        Refresh
    ]
);

const KEY_CONTEXT: &str = "Dashboard";

/// Binds the dashboard's shortcuts. `secondary` is cmd on macOS and ctrl elsewhere.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("secondary-r", Refresh, Some(KEY_CONTEXT))]);
}

pub enum DashboardEvent {
    SignedOut,
    /// The instance rejected the token, e.g. because it was revoked since it was saved.
    SessionExpired(SharedString),
}

/// The main window: a header for the signed-in team, with the team's projects below.
pub struct DashboardView {
    client: Client,
    team: Option<Team>,
    error: Option<SharedString>,
    /// The in-flight team fetch. Dropping it cancels the fetch.
    loading: Option<Task<()>>,
    projects: Entity<ProjectsView>,
    /// Focused on open so the dashboard's shortcuts work without clicking into the window first.
    focus_handle: FocusHandle,
    _subscription: Subscription,
}

impl EventEmitter<DashboardEvent> for DashboardView {}

impl DashboardView {
    /// Without a `team` (e.g. when restoring a saved session) it's fetched, which also re-checks the token.
    pub fn new(
        store: Arc<Store>,
        client: Client,
        team: Option<Team>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let projects = cx.new(|cx| ProjectsView::new(client.clone(), store, cx));
        let subscription = cx.subscribe(&projects, |_, _, event, cx| match event {
            ProjectsEvent::SessionExpired(reason) => cx.emit(DashboardEvent::SessionExpired(reason.clone())),
        });

        let focus_handle = cx.focus_handle();
        focus_handle.focus(window, cx);

        let mut this = Self {
            client,
            team,
            error: None,
            loading: None,
            projects,
            focus_handle,
            _subscription: subscription,
        };
        if this.team.is_none() {
            this.load_team(cx);
        }
        this
    }

    fn load_team(&mut self, cx: &Context<Self>) {
        let client = self.client.clone();
        self.loading = Some(spawn_api_call(
            cx,
            move || client.current_team(),
            |this, result, cx| {
                this.loading = None;
                match result {
                    Ok(team) => this.team = Some(team),
                    Err(error @ ApiError::InvalidToken) => {
                        cx.emit(DashboardEvent::SessionExpired(error.to_string().into()));
                    }
                    Err(error) => this.error = Some(error.to_string().into()),
                }
                cx.notify();
            },
        ));
    }

    fn retry(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.load_team(cx);
        cx.notify();
    }

    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let title = self
            .team
            .as_ref()
            .map_or_else(|| SharedString::from("Connecting…"), |team| team.name.clone().into());

        h_flex()
            .justify_between()
            .gap_4()
            .px_6()
            .py_3()
            .border_b_1()
            .border_color(theme.border)
            .child(
                v_flex()
                    .min_w_0()
                    .child(div().font_semibold().text_ellipsis().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .text_ellipsis()
                            .child(self.client.credentials().instance().to_string()),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .flex_none()
                    .when_some(self.error.clone(), |this, error| {
                        this.child(div().text_sm().text_color(theme.danger).child(error)).child(
                            Button::new("retry-team")
                                .label("Retry")
                                .on_click(cx.listener(|this, _, _, cx| this.retry(cx))),
                        )
                    })
                    .child(
                        Button::new("sign-out")
                            .ghost()
                            .icon(IconName::LogOut)
                            .label("Sign out")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(DashboardEvent::SignedOut))),
                    ),
            )
    }
}

impl Render for DashboardView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .track_focus(&self.focus_handle)
            .key_context(KEY_CONTEXT)
            .on_action(cx.listener(|this, _: &Refresh, _, cx| {
                this.projects.update(cx, ProjectsView::refresh);
            }))
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(self.render_header(cx))
            .child(div().flex_1().min_h_0().child(self.projects.clone()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use gpui_kit::{TestAppContext, WindowOptions, test::TestWindowExt as _};

    use super::*;
    use crate::test_support::{serve, store};

    #[gpui_kit::test]
    fn refresh_shortcut_reloads_projects(cx: &mut TestAppContext) {
        let (_dir, store) = store();
        let (client, requests) = serve("200 OK", "[]");
        let team = Team {
            id: 0,
            name: "Team".into(),
        };
        let (window, _) = cx.update(|cx| {
            gpui_kit::init(cx);
            bind_keys(cx);
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| DashboardView::new(store, client, Some(team), window, cx))
            })
            .unwrap()
        });

        cx.run_until_parked();
        assert_eq!(requests.load(Ordering::SeqCst), 1);

        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.press("secondary-r", cx);
        })
        .unwrap();

        cx.run_until_parked();
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }
}
