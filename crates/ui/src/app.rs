use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _, Subscription, Window,
    component::ActiveTheme as _, div,
};

use crate::views::{DashboardEvent, DashboardView, LoginEvent, LoginView};

enum Screen {
    Login(Entity<LoginView>),
    Dashboard(Entity<DashboardView>),
}

/// The root view of the main window. Decides which screen is shown.
pub struct CoolyApp {
    screen: Screen,
    _subscription: Subscription,
}

impl CoolyApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (screen, subscription) = Self::login(window, cx);
        Self {
            screen,
            _subscription: subscription,
        }
    }

    fn login(window: &mut Window, cx: &mut Context<Self>) -> (Screen, Subscription) {
        let login = cx.new(|cx| LoginView::new(window, cx));
        let subscription = cx.subscribe_in(&login, window, |this, _, event, window, cx| match event {
            LoginEvent::Submitted(credentials) => {
                let (screen, subscription) = Self::dashboard(credentials.clone(), window, cx);
                this.show(screen, subscription, cx);
            }
        });
        (Screen::Login(login), subscription)
    }

    fn dashboard(
        credentials: cooly_api::Credentials,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> (Screen, Subscription) {
        let dashboard = cx.new(|_| DashboardView::new(credentials));
        let subscription = cx.subscribe_in(&dashboard, window, |this, _, event, window, cx| match event {
            DashboardEvent::SignedOut => {
                let (screen, subscription) = Self::login(window, cx);
                this.show(screen, subscription, cx);
            }
        });
        (Screen::Dashboard(dashboard), subscription)
    }

    fn show(&mut self, screen: Screen, subscription: Subscription, cx: &mut Context<Self>) {
        *self = Self {
            screen,
            _subscription: subscription,
        };
        cx.notify();
    }
}

impl Render for CoolyApp {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let screen = match &self.screen {
            Screen::Login(view) => view.clone().into_any_element(),
            Screen::Dashboard(view) => view.clone().into_any_element(),
        };
        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(screen)
    }
}
