mod user;
mod utils;
mod web;
mod pages;

use crate::pages::*;
use crate::user::controller::*;
use crate::user::view::*;
use crate::web::inject_globals;

use std::env;
use sqlx::PgPool;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use axum_extra::routing::RouterExt;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions::cookie::{SameSite, time::Duration};
use tower_sessions_sqlx_store::PostgresStore;
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

    let app = app(db).layer(session_layer);

    let mut listenfd = ListenFd::from_env();

    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        Some(listener) => {
            listener.set_nonblocking(true).unwrap();
            TcpListener::from_std(listener).unwrap()
        }
        None => TcpListener::bind("127.0.0.1:3000").await.unwrap(),
    };

    tracing::debug!("listening on {}", listener.local_addr().unwrap());

    let _ = axum::serve(listener, app).await;
}

fn app(db: PgPool) -> Router {

    Router::new()
        .route("/", get(home_template))
        .route_with_tsr("/about", get(about_template))
        .route_with_tsr("/register", get(register_template))
        .route_with_tsr("/login", get(login_template))
        .route_with_tsr("/account", get(account_template))
        .route_with_tsr("/webtransport", get(webtransport_template))
        .route_with_tsr("/styleguide", get(styleguide_template))
        .route("/account/{page_name}", get(account_page_template))
        .route("/user/register", post(register_user_controller))
        .route("/user/login", post(login_user_controller))
        .route("/user/logout", post(logout_user_controller))
        .route("/user/verify/{token}", get(verify_user_controller))
        .with_state(db)
        .nest_service("/assets", ServeDir::new("assets"))
        .fallback(handler_404)
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

        app(db).layer(SessionManagerLayer::new(MemoryStore::default()))
    }

    #[tokio::test]
    async fn home_page_renders() {
        let response = test_app()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let response = test_app()
            .oneshot(Request::get("/does-not-exist").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
