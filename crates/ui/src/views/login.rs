use cooly_api::Credentials;
use gpui_kit::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, ParentElement as _, Render, SharedString, Styled as _,
    Subscription, Window,
    component::{
        ActiveTheme as _, StyledExt as _,
        button::{Button, ButtonVariants as _},
        form::{Field, Form},
        input::{Input, InputEvent, InputState},
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
};

/// Emitted once the user submits credentials that pass local validation.
pub enum LoginEvent {
    Submitted(Credentials),
}

/// Asks for the Coolify instance URL and an API token.
pub struct LoginView {
    instance: Entity<InputState>,
    token: Entity<InputState>,
    error: Option<SharedString>,
    _subscriptions: [Subscription; 2],
}

impl EventEmitter<LoginEvent> for LoginView {}

impl LoginView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let instance = cx.new(|cx| InputState::new(window, cx).placeholder("https://coolify.example.com"));
        let token = cx.new(|cx| InputState::new(window, cx).placeholder("API token").masked(true));
        let subscriptions = [
            cx.subscribe_in(&instance, window, Self::on_input_event),
            cx.subscribe_in(&token, window, Self::on_input_event),
        ];
        instance.update(cx, |input, cx| input.focus(window, cx));

        Self {
            instance,
            token,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn on_input_event(&mut self, _: &Entity<InputState>, event: &InputEvent, _: &mut Window, cx: &mut Context<Self>) {
        match event {
            InputEvent::PressEnter { .. } => self.submit(cx),
            InputEvent::Change if self.error.is_some() => {
                self.error = None;
                cx.notify();
            }
            _ => {}
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let instance = self.instance.read(cx).value();
        let token = self.token.read(cx).value();
        match Credentials::parse(&instance, &token) {
            Ok(credentials) => {
                self.error = None;
                cx.emit(LoginEvent::Submitted(credentials));
            }
            Err(error) => self.error = Some(error.to_string().into()),
        }
        cx.notify();
    }
}

impl Render for LoginView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        // The titlebar is transparent, so the extra top padding keeps content clear of the traffic lights.
        v_flex()
            .size_full()
            .gap_6()
            .px_8()
            .pt_10()
            .pb_8()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_2xl().font_semibold().child("Sign in to Coolify"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Create an API token under Keys & Tokens in your Coolify dashboard."),
                    ),
            )
            .child(
                Form::new()
                    .child(
                        Field::new()
                            .label("Instance URL")
                            .child(Input::new(&self.instance).id("instance").aria_label("Instance URL")),
                    )
                    .child(
                        Field::new().label("API token").child(
                            Input::new(&self.token)
                                .id("token")
                                .aria_label("API token")
                                .mask_toggle(),
                        ),
                    ),
            )
            .child(
                Button::new("sign-in")
                    .primary()
                    .w_full()
                    .label("Sign in")
                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
            )
            .when_some(self.error.clone(), |this, error| {
                this.child(div().text_sm().text_color(theme.danger).child(error))
            })
    }
}
