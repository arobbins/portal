use crate::mail::Mailer;
use crate::rate_limit::RateLimits;
use crate::theme::Theme;
use crate::utils::create_body_page_class;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request},
    http::Uri,
    middleware::Next,
    response::Response,
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use topcoat::{
    Error, Result,
    context::{Cx, app_context},
    router::{RemoteAddr, request},
};
use tower_sessions::Session;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentUser {
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalState {
    pub year: i32,
    pub current_user: Option<CurrentUser>,
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
    app_context::<PgPool>(cx)
}

pub fn mailer(cx: &Cx) -> &Mailer {
    app_context::<Mailer>(cx)
}

pub fn app_url(path: &str) -> Result<String> {
    let base_url = std::env::var("APP_BASE_URL")
        .map_err(|e| Error::from(e).context("APP_BASE_URL is not set"))?;
    Ok(format!("{base_url}{path}"))
}

pub fn rate_limits(cx: &Cx) -> &RateLimits {
    app_context::<Arc<RateLimits>>(cx)
}

pub async fn attach_remote_addr(mut req: Request, next: Next) -> Response {
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>().copied() {
        req.extensions_mut().insert(RemoteAddr(addr));
    }

    next.run(req).await
}

pub async fn inject_globals(session: Session, uri: Uri, mut req: Request, next: Next) -> Response {
    let current_user = session
        .get::<CurrentUser>("current_user")
        .await
        .unwrap_or(None);

    let current_path = uri.path().trim_start_matches('/').to_string();

    let theme = Theme::from_headers(req.headers());

    req.extensions_mut().insert(GlobalState {
        year: chrono::Local::now().year(),
        current_user,
        current_slug: create_body_page_class(current_path),
        theme,
    });

    next.run(req).await
}
