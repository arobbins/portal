use crate::utils::create_body_page_class;

use askama::Template;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use axum::{
    http::{StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    extract::Request,
    middleware::Next,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct GlobalState {
    pub year: i32,
    pub current_user: Option<CurrentUser>,
    pub current_slug: String,
}

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}

pub async fn inject_globals(session: Session, uri: Uri, mut req: Request, next: Next) -> Response {
    let current_user = session
        .get::<CurrentUser>("current_user")
        .await
        .unwrap_or(None);

    let current_path = uri.path().trim_start_matches('/').to_string();

    req.extensions_mut().insert(GlobalState {
        year: chrono::Local::now().year(),
        current_user,
        current_slug: create_body_page_class(current_path),
    });

    next.run(req).await
}