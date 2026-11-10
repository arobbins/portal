use crate::config::Config;
use crate::mail::Mailer;
use crate::rate_limit::RateLimits;
use crate::theme::Theme;
use crate::user::auth::load_signed_in_user;
use crate::user::model::User;
use crate::utils::create_body_page_class;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode, Uri, header::CACHE_CONTROL},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Datelike;
use sqlx::PgPool;
use topcoat::{
    Error, Result,
    context::{Cx, context},
    router::{RemoteAddr, request},
};
use tower_sessions::Session;

#[derive(Debug, Clone, Default)]
pub struct GlobalState {
    pub year: i32,
    pub current_user: Option<Arc<User>>,
    pub current_slug: String,
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

pub fn db(cx: &Cx) -> &PgPool {
    context::<PgPool>(cx)
}

pub fn config(cx: &Cx) -> &Config {
    context::<Config>(cx)
}

pub fn mailer(cx: &Cx) -> &Mailer {
    &config(cx).mailer
}

pub fn app_url(cx: &Cx, path: &str) -> String {
    config(cx).url(path)
}

pub fn rate_limits(cx: &Cx) -> &RateLimits {
    context::<Arc<RateLimits>>(cx)
}

pub async fn attach_remote_addr(mut req: Request, next: Next) -> Response {
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>().copied() {
        req.extensions_mut().insert(RemoteAddr(addr));
    }

    next.run(req).await
}

pub async fn inject_globals(
    State(db): State<PgPool>,
    session: Session,
    uri: Uri,
    mut req: Request,
    next: Next,
) -> Response {
    let current_user = match load_signed_in_user(&session, &db).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Failed to load signed-in user: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let current_path = uri.path().trim_start_matches('/').to_string();

    let theme = Theme::from_headers(req.headers());
    let is_signed_in = current_user.is_some();

    req.extensions_mut().insert(GlobalState {
        year: chrono::Local::now().year(),
        current_user,
        current_slug: create_body_page_class(current_path),
        theme,
    });

    let mut response = next.run(req).await;
    if is_signed_in {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}
