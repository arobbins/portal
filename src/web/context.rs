use std::sync::Arc;

use topcoat::{
    Error, Result,
    context::{Cx, context},
    router::request::{self, client_ip},
};
use tower_sessions::Session;

use crate::config::Config;
use crate::env::{App, Env};
use crate::theme::Theme;
use crate::user::model::User;

#[derive(Debug, Clone, Default)]
pub struct GlobalState {
    pub year: i32,
    pub current_user: Option<Arc<User>>,
    pub theme: Theme,
}

pub fn globals(cx: &Cx) -> GlobalState {
    request::extensions(cx)
        .get::<GlobalState>()
        .cloned()
        .unwrap_or_default()
}

pub fn session(cx: &Cx) -> Result<Session> {
    request::extensions(cx)
        .get::<Session>()
        .cloned()
        .ok_or_else(|| Error::msg("session layer is not installed"))
}

pub fn app(cx: &Cx) -> &App {
    context::<App>(cx)
}

pub fn env(cx: &Cx) -> Env<'_> {
    app(cx).env(client_ip(cx))
}

pub fn config(cx: &Cx) -> &Config {
    &app(cx).config
}

pub fn redisplay<T: Send + Sync + 'static>(path: impl Into<String>, failure: T) -> Error {
    topcoat::router::error::rewrite(path.into(), topcoat::router::Body::empty())
        .method(topcoat::router::Method::GET)
        .with(failure)
        .into()
}
