mod app;
mod cleanup;
mod components;
mod flash;
mod layout;
mod mail;
mod pages;
mod rate_limit;
mod seed;
mod theme;
mod user;
mod utils;
mod web;

use crate::mail::Mailer;
use crate::rate_limit::RateLimits;
use crate::web::{attach_remote_addr, inject_globals};

use axum::{Router, ServiceExt, extract::Request, middleware};
use listenfd::ListenFd;
use sqlx::PgPool;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use topcoat::asset::AssetBundle;
use topcoat::router::tower::TowerService;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_sessions::cookie::{SameSite, time::Duration};
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    if env::args().nth(1).as_deref() == Some("seed") {
        seed::run(&database_url).await;
        return;
    }

    let db = PgPool::connect(&database_url).await.unwrap();

    let session_store = PostgresStore::new(db.clone());
    session_store.migrate().await.unwrap();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)));

    let assets = AssetBundle::load()
        .expect("missing Topcoat asset bundle: run `topcoat dev` or `topcoat asset bundle`");

    let rate_limits = Arc::new(RateLimits::from_env());
    tokio::spawn({
        let rate_limits = rate_limits.clone();
        async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                rate_limits.forget_idle_keys();
            }
        }
    });

    cleanup::spawn(db.clone());

    let app = app(db, Mailer::from_env(), rate_limits, assets).layer(session_layer);
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

    let _ = axum::serve(
        listener,
        ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app),
    )
    .await;
}

fn bind_address() -> String {
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    format!("{host}:{port}")
}

fn app(db: PgPool, mailer: Mailer, rate_limits: Arc<RateLimits>, assets: AssetBundle) -> Router {
    let pages = app::router(db.clone(), mailer, rate_limits, assets);

    Router::new()
        .fallback_service(TowerService::new(pages))
        .layer(middleware::from_fn_with_state(db, inject_globals))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(middleware::from_fn(attach_remote_addr))
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limit::RateLimitMode;
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
        let assets =
            AssetBundle::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/assets"))
                .expect("missing asset bundle: run `topcoat asset bundle` before `cargo test`");

        let rate_limits = Arc::new(RateLimits::new(RateLimitMode::Strict));

        app(db, Mailer::Log, rate_limits, assets)
            .layer(SessionManagerLayer::new(MemoryStore::default()))
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

    async fn login_attempt_from(app: &Router, ip: [u8; 4]) -> StatusCode {
        let request = Request::post("/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((ip, 50_000))))
            .body(Body::from("email=someone%40example.test&password=guess"))
            .unwrap();
        app.clone().oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn too_many_login_attempts_from_one_ip_get_a_429() {
        let app = test_app();
        let attacker = [203, 0, 113, 1];
        let someone_else = [203, 0, 113, 2];

        for _ in 0..10 {
            assert_ne!(
                login_attempt_from(&app, attacker).await,
                StatusCode::TOO_MANY_REQUESTS
            );
        }

        assert_eq!(
            login_attempt_from(&app, attacker).await,
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_ne!(
            login_attempt_from(&app, someone_else).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    async fn sign_up_from(app: &Router, ip: [u8; 4]) -> StatusCode {
        let request = Request::post("/register")
            .header("content-type", "application/x-www-form-urlencoded")
            .extension(axum::extract::ConnectInfo(SocketAddr::from((ip, 50_000))))
            .body(Body::from("name=Someone&email=someone%40example.test"))
            .unwrap();
        app.clone().oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn too_many_sign_ups_from_one_ip_get_a_429() {
        let app = test_app();
        let spammer = [203, 0, 113, 3];

        for _ in 0..5 {
            assert_ne!(
                sign_up_from(&app, spammer).await,
                StatusCode::TOO_MANY_REQUESTS
            );
        }

        assert_eq!(
            sign_up_from(&app, spammer).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
}
