mod app;
mod components;
mod layout;
mod pages;
mod theme;
mod user;
mod utils;
mod web;

use crate::user::controller::verify_user_controller;
use crate::web::inject_globals;

use std::env;
use sqlx::PgPool;
use axum::{
    extract::Request,
    middleware,
    routing::get,
    Router,
    ServiceExt,
};
use topcoat::asset::AssetBundle;
use topcoat::router::tower::TowerService;
use tower::Layer;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions::cookie::{SameSite, time::Duration};
use tower_sessions_sqlx_store::PostgresStore;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use listenfd::ListenFd;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().unwrap();

    let db = PgPool::connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let session_store = PostgresStore::new(db.clone());
    session_store.migrate().await.unwrap();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)));

    let assets = AssetBundle::load()
        .expect("missing Topcoat asset bundle: run `topcoat dev` or `topcoat asset bundle`");

    let app = app(db, assets).layer(session_layer);
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    let mut listenfd = ListenFd::from_env();

    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        Some(listener) => {
            listener.set_nonblocking(true).unwrap();
            TcpListener::from_std(listener).unwrap()
        }
        None => TcpListener::bind(bind_address()).await.unwrap(),
    };

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    topcoat::dev::notify_ready(listener.local_addr().ok()).await;

    let _ = axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await;
}

fn bind_address() -> String {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    format!("{host}:{port}")
}

fn app(db: PgPool, assets: AssetBundle) -> Router {
    let pages = app::router(db.clone(), assets);

    Router::new()
        .route("/user/verify/{token}", get(verify_user_controller))
        .nest_service("/assets", ServeDir::new("assets"))
        .fallback_service(TowerService::new(pages))
        .with_state(db)
        .layer(middleware::from_fn(inject_globals))
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;
    use tower_sessions::MemoryStore;

    fn test_app() -> Router {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap();
        let assets = AssetBundle::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/assets"))
            .expect("missing asset bundle: run `topcoat asset bundle` before `cargo test`");

        app(db, assets).layer(SessionManagerLayer::new(MemoryStore::default()))
    }

    async fn get_status(path: &str) -> StatusCode {
        test_app()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn home_page_renders() {
        assert_eq!(get_status("/").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn styleguide_renders() {
        assert_eq!(get_status("/styleguide").await, StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        assert_eq!(get_status("/does-not-exist").await, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn account_redirects_signed_out_users() {
        let status = get_status("/account").await;
        assert!(status.is_redirection(), "expected redirect, got {status}");
    }
}
