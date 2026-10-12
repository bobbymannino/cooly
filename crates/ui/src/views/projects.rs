use std::sync::Arc;

use cooly_api::{ApiError, Client, Project};
use cooly_store::{ProjectsLayout, Store};
use gpui_kit::{
    AnyElement, Context, EventEmitter, Hsla, IntoElement, ParentElement as _, Pixels, Render, SharedString,
    Styled as _, Task, Window,
    assets::IconName,
    component::{
        ActiveTheme as _, Icon, Selectable as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        h_flex,
        scroll::ScrollableElement as _,
        v_flex,
    },
    div,
    prelude::{FluentBuilder as _, InteractiveElement as _},
    px,
};

use crate::api_task::spawn_api_call;

pub enum ProjectsEvent {
    /// The instance rejected the token while loading projects.
    SessionExpired(SharedString),
}

/// The team's projects as a grid of cards or a list of rows. The chosen layout is saved in preferences.
pub struct ProjectsView {
    client: Client,
    store: Arc<Store>,
    layout: ProjectsLayout,
    /// `None` until the first load succeeds. Kept across failed refreshes so the list doesn't vanish.
    projects: Option<Vec<Project>>,
    error: Option<SharedString>,
    /// The in-flight fetch. Dropping it cancels the fetch.
    loading: Option<Task<()>>,
}

impl EventEmitter<ProjectsEvent> for ProjectsView {}

impl ProjectsView {
    pub fn new(client: Client, store: Arc<Store>, cx: &mut Context<Self>) -> Self {
        let layout = store
            .preferences()
            .map(|preferences| preferences.projects_layout)
            .unwrap_or_default();
        let mut this = Self {
            client,
            store,
            layout,
            projects: None,
            error: None,
            loading: None,
        };
        this.refresh(cx);
        this
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.error = None;
        self.loading = Some(spawn_api_call(
            cx,
            move || client.projects(),
            |this, result, cx| {
                this.loading = None;
                match result {
                    Ok(mut projects) => {
                        projects.sort_by_cached_key(|project| project.name.to_lowercase());
                        this.projects = Some(projects);
                    }
                    Err(error @ ApiError::InvalidToken) => {
                        cx.emit(ProjectsEvent::SessionExpired(error.to_string().into()));
                    }
                    Err(error) => this.error = Some(error.to_string().into()),
                }
                cx.notify();
            },
        ));
        cx.notify();
    }

    fn set_layout(&mut self, layout: ProjectsLayout, cx: &mut Context<Self>) {
        if self.layout == layout {
            return;
        }
        self.layout = layout;
        cx.notify();

        // Read-modify-write so other preferences are kept.
        let mut preferences = self.store.preferences().unwrap_or_default();
        preferences.projects_layout = layout;
        if let Err(error) = self.store.save_preferences(&preferences) {
            eprintln!("failed to save preferences: {error}");
        }
    }

    fn render_toolbar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let count = self.projects.as_ref().map(Vec::len);

        h_flex()
            .justify_between()
            .px_6()
            .pt_5()
            .pb_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_baseline()
                    .child(div().text_lg().font_semibold().child("Projects"))
                    .when_some(count, |this, count| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(count.to_string()),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("refresh-projects")
                            .ghost()
                            .icon(IconName::RefreshCw)
                            .tooltip("Refresh")
                            .loading(self.loading.is_some())
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                    )
                    .child(
                        Button::new("projects-grid")
                            .ghost()
                            .icon(IconName::LayoutGrid)
                            .tooltip("Grid")
                            .selected(self.layout == ProjectsLayout::Grid)
                            .on_click(cx.listener(|this, _, _, cx| this.set_layout(ProjectsLayout::Grid, cx))),
                    )
                    .child(
                        Button::new("projects-list")
                            .ghost()
                            .icon(IconName::LayoutList)
                            .tooltip("List")
                            .selected(self.layout == ProjectsLayout::List)
                            .on_click(cx.listener(|this, _, _, cx| this.set_layout(ProjectsLayout::List, cx))),
                    ),
            )
    }

    fn render_body(&self, window: &Window, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();
        match &self.projects {
            None if self.error.is_none() => centered()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("Loading projects…")
                .into_any_element(),
            None => centered()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.danger)
                        .child(self.error.clone().unwrap_or_default()),
                )
                .child(
                    Button::new("retry-projects")
                        .label("Retry")
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                )
                .into_any_element(),
            Some(projects) if projects.is_empty() => centered()
                .gap_2()
                .child(Icon::new(IconName::Inbox).size_8().text_color(theme.muted_foreground))
                .child(div().font_semibold().child("No projects yet"))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child("Projects you create in Coolify will show up here."),
                )
                .into_any_element(),
            Some(projects) => match self.layout {
                ProjectsLayout::Grid => project_grid(projects, window.viewport_size().width, cx).into_any_element(),
                ProjectsLayout::List => project_list(projects, cx).into_any_element(),
            },
        }
    }
}

impl Render for ProjectsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        // A failed refresh keeps showing the last projects, with the error above them.
        let refresh_error = self.projects.as_ref().and_then(|_| self.error.clone());

        v_flex()
            .size_full()
            .child(self.render_toolbar(cx))
            .when_some(refresh_error, |this, error| {
                this.child(div().px_6().pb_3().text_sm().text_color(theme.danger).child(error))
            })
            .child(
                div()
                    .id("projects-scroll")
                    .flex_1()
                    .min_h_0()
                    .px_6()
                    .pb_6()
                    .overflow_y_scrollbar()
                    .child(self.render_body(window, cx)),
            )
    }
}

fn centered() -> gpui_kit::Div {
    v_flex().size_full().items_center().justify_center().py_16()
}

/// Picks a column count that keeps cards a comfortable width as the window resizes.
fn grid_columns(width: Pixels) -> u16 {
    if width >= px(1200.0) {
        4
    } else if width >= px(880.0) {
        3
    } else if width >= px(560.0) {
        2
    } else {
        1
    }
}

fn project_grid(projects: &[Project], width: Pixels, cx: &gpui_kit::App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .grid()
        .grid_cols(grid_columns(width))
        .gap_4()
        .children(projects.iter().map(|project| {
            v_flex()
                .gap_2()
                .p_4()
                .min_w_0()
                .rounded(theme.radius_lg)
                .border_1()
                .border_color(theme.border)
                .child(project_name(project, theme.muted_foreground))
                .child(
                    description(project, theme.muted_foreground)
                        .text_sm()
                        .line_clamp(2)
                        .h(px(40.0)),
                )
        }))
}

fn project_list(projects: &[Project], cx: &gpui_kit::App) -> impl IntoElement {
    let theme = cx.theme();
    let last = projects.len().saturating_sub(1);
    v_flex()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(theme.border)
        .children(projects.iter().enumerate().map(|(index, project)| {
            h_flex()
                .gap_4()
                .px_4()
                .py_3()
                .when(index != last, |this| this.border_b_1().border_color(theme.border))
                .child(
                    div()
                        .w(px(240.0))
                        .flex_none()
                        .child(project_name(project, theme.muted_foreground)),
                )
                .child(
                    description(project, theme.muted_foreground)
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .text_ellipsis(),
                )
        }))
}

fn project_name(project: &Project, icon_color: Hsla) -> impl IntoElement {
    h_flex()
        .gap_2()
        .min_w_0()
        .child(Icon::new(IconName::FolderOpen).text_color(icon_color).flex_none())
        .child(
            div()
                .min_w_0()
                .font_semibold()
                .text_ellipsis()
                .child(project.name.clone()),
        )
}

fn description(project: &Project, muted: Hsla) -> gpui_kit::Div {
    let text = project
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty());
    div()
        .text_color(muted)
        .when(text.is_none(), gpui_kit::Styled::italic)
        .child(SharedString::from(text.unwrap_or("No description").to_owned()))
}

#[cfg(test)]
mod tests {
    use std::{
        cell::Cell,
        io::{Read as _, Write as _},
        net::TcpListener,
        rc::Rc,
        thread,
    };

    use cooly_api::Credentials;
    use cooly_store::Keychain;
    use gpui_kit::{AnyWindowHandle, AppContext as _, Entity, TestAppContext, WindowOptions, test::TestWindowExt as _};
    use tempfile::TempDir;

    use super::*;

    /// A client for a local server that answers one request with `status` and `body`.
    fn serve(status: &'static str, body: &'static str) -> Client {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: \
                 close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        Client::new(Credentials::parse(&format!("http://{address}"), "1|token").unwrap())
    }

    /// The keychain is never touched: only preferences are read and written, and those aren't encrypted.
    fn store() -> (TempDir, Arc<Store>) {
        let dir = TempDir::new().unwrap();
        let store = Arc::new(Store::new(dir.path(), Keychain));
        (dir, store)
    }

    fn open(cx: &TestAppContext, client: Client, store: Arc<Store>) -> (AnyWindowHandle, Entity<ProjectsView>) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|cx| ProjectsView::new(client, store, cx))
            })
            .unwrap()
        })
    }

    #[gpui_kit::test]
    fn loads_sorted_projects_and_saves_layout(cx: &mut TestAppContext) {
        let (_dir, store) = store();
        let client = serve(
            "200 OK",
            r#"[{"uuid":"b","name":"beta","description":null},{"uuid":"a","name":"Alpha","description":"Site"}]"#,
        );
        let (window, view) = open(cx, client, store.clone());

        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            let names: Vec<_> = view
                .projects
                .iter()
                .flatten()
                .map(|project| project.name.as_str())
                .collect();
            assert_eq!(names, ["Alpha", "beta"]);
            assert_eq!(view.layout, ProjectsLayout::Grid);
            assert!(view.error.is_none());
        });

        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.click("projects-list", cx);
        })
        .unwrap();

        view.read_with(cx, |view, _| assert_eq!(view.layout, ProjectsLayout::List));
        assert_eq!(store.preferences().unwrap().projects_layout, ProjectsLayout::List);
    }

    #[gpui_kit::test]
    fn rejected_token_expires_session(cx: &TestAppContext) {
        let (_dir, store) = store();
        let client = serve("401 Unauthorized", r#"{"message":"Unauthenticated."}"#);
        let (_, view) = open(cx, client, store);

        let expired = Rc::new(Cell::new(false));
        let seen = expired.clone();
        cx.update(|cx| {
            cx.subscribe(&view, move |_, event, _| match event {
                ProjectsEvent::SessionExpired(_) => seen.set(true),
            })
            .detach();
        });

        cx.run_until_parked();
        assert!(expired.get());
        view.read_with(cx, |view, _| assert!(view.projects.is_none()));
    }
}
