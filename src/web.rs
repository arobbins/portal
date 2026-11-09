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
    http::{StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Datelike;
use sqlx::PgPool;
use topcoat::{
    Error, Result,
    context::{Cx, app_context},
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

    req.extensions_mut().insert(GlobalState {
        year: chrono::Local::now().year(),
        current_user,
        current_slug: create_body_page_class(current_path),
        theme,
    });

    next.run(req).await
}
