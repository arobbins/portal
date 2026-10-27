use crate::web::{GlobalState, HtmlTemplate};

use askama::Template;
use axum::{
    extract::{Extension},
    http::StatusCode,
    response::IntoResponse,
};

pub async fn handler_404(uri: axum::http::Uri, Extension(globals): Extension<GlobalState>) -> impl IntoResponse {
    let template = NotFoundTemplate {
        path: uri.path().to_string(),
        globals,
    };
    tracing::debug!("Huh ... 404 {}", uri.path());
    (StatusCode::NOT_FOUND, HtmlTemplate(template))
}

pub async fn styleguide_template(Extension(globals): Extension<GlobalState>) -> impl IntoResponse {
    let template = StyleguideTemplate { globals };
    HtmlTemplate(template)
}

pub async fn about_template(Extension(globals): Extension<GlobalState>) -> impl IntoResponse {
    let template = AboutTemplate { globals };
    HtmlTemplate(template)
}

pub async fn webtransport_template(Extension(globals): Extension<GlobalState>) -> impl IntoResponse {
    let template = WebTransportTemplate { globals };
    HtmlTemplate(template)
}

pub async fn home_template(Extension(globals): Extension<GlobalState>) -> impl IntoResponse {
    let template = HomeTemplate { globals };
    HtmlTemplate(template)
}

#[derive(Template)]
#[template(path = "pages/styleguide.html")]
pub struct StyleguideTemplate {
    globals: GlobalState
}

#[derive(Template)]
#[template(path = "pages/about.html")]
pub struct AboutTemplate {
    globals: GlobalState
}

#[derive(Template)]
#[template(path = "pages/home.html")]
pub struct HomeTemplate {
    globals: GlobalState
}

#[derive(Template)]
#[template(path = "pages/webtransport.html")]
pub struct WebTransportTemplate {
    globals: GlobalState
}

#[derive(Template)]
#[template(path = "pages/404.html")]
pub struct NotFoundTemplate {
    path: String,
    globals: GlobalState,
}