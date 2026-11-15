use super::router::router;
use crate::env::App;
use axum::Router;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use topcoat::asset::AssetBundle;
use tower::ServiceExt;
use tower_sessions::MemoryStore;
use tower_sessions::SessionManagerLayer;

fn test_app() -> Router {
    test_app_with(
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
    )
}

fn test_app_with(db: PgPool) -> Router {
    let assets = AssetBundle::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/assets"))
        .expect("missing asset bundle: run `topcoat asset bundle` before `cargo test`");

    router(App::for_tests(db), assets).layer(SessionManagerLayer::new(MemoryStore::default()))
}

async fn get_status(path: &str) -> StatusCode {
    test_app()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[sqlx::test]
async fn health_checks_answer_without_a_session_or_redirect(db: PgPool) {
    let app = test_app_with(db);
    for path in ["/healthz", "/readyz"] {
        let request = Request::get(path)
            .header("host", "load-balancer.internal")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert!(response.headers().contains_key("x-request-id"));
    }
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

async fn login_attempt_from(app: &Router, ip: [u8; 4], attempt: usize) -> StatusCode {
    let request = Request::post("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .extension(axum::extract::ConnectInfo(SocketAddr::from((ip, 50_000))))
        .body(Body::from(format!(
            "email=someone{attempt}%40example.test&password=guess"
        )))
        .unwrap();
    app.clone().oneshot(request).await.unwrap().status()
}

#[sqlx::test]
async fn too_many_login_attempts_from_one_ip_get_a_429(db: PgPool) {
    let app = test_app_with(db);
    let attacker = [203, 0, 113, 1];
    let someone_else = [203, 0, 113, 2];

    for attempt in 0..10 {
        assert_ne!(
            login_attempt_from(&app, attacker, attempt).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    assert_eq!(
        login_attempt_from(&app, attacker, 10).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_ne!(
        login_attempt_from(&app, someone_else, 11).await,
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

#[sqlx::test]
async fn too_many_sign_ups_from_one_ip_get_a_429(db: PgPool) {
    let app = test_app_with(db);
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

async fn response_for(
    app: &Router,
    method: &str,
    path: &str,
    host: &str,
) -> axum::response::Response {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", host)
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn other_hosts_are_sent_to_the_canonical_address() {
    let app = test_app();

    let response = response_for(&app, "GET", "/about?x=1", "127.0.0.1").await;
    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(response.headers()["location"], "http://localhost/about?x=1");

    let canonical = response_for(&app, "GET", "/about", "localhost").await;
    assert_eq!(canonical.status(), StatusCode::OK);

    let form_post = response_for(&app, "POST", "/theme", "127.0.0.1").await;
    assert_ne!(form_post.status(), StatusCode::PERMANENT_REDIRECT);
}

#[sqlx::test]
async fn every_documented_endpoint_exists_and_needs_a_token(db: PgPool) {
    let app = test_app_with(db);

    for endpoint in crate::api::catalog::endpoints() {
        let request = Request::builder()
            .method(endpoint.method.as_str())
            .uri(endpoint.path)
            .header("host", "localhost")
            .body(Body::empty())
            .unwrap();
        let status = app.clone().oneshot(request).await.unwrap().status();

        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{} {}",
            endpoint.method.as_str(),
            endpoint.path
        );
    }
}

#[sqlx::test]
async fn every_emailed_link_points_at_a_real_page(db: PgPool) {
    let app = test_app_with(db);
    for purpose in crate::auth::links::TokenPurpose::ALL {
        let request = Request::get(purpose.path("not-a-real-token"))
            .header("host", "localhost")
            .body(Body::empty())
            .unwrap();
        let status = app.clone().oneshot(request).await.unwrap().status();

        assert_eq!(status, StatusCode::BAD_REQUEST, "{}", purpose.as_str());
    }
}

#[tokio::test]
async fn the_openapi_document_docs_page_and_llms_txt_are_public() {
    assert_eq!(get_status("/llms.txt").await, StatusCode::OK);
    assert_eq!(get_status("/api/v1/openapi.json").await, StatusCode::OK);
    assert_eq!(get_status("/docs/api").await, StatusCode::OK);
}

async fn status_of(app: &Router, path: &str) -> StatusCode {
    let request = Request::get(path)
        .header("host", "localhost")
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(request).await.unwrap().status()
}

#[sqlx::test]
async fn every_named_path_is_a_real_page(db: PgPool) {
    let app = test_app_with(db);
    let account_paths = crate::account::tabs::AccountSection::ALL
        .into_iter()
        .map(|section| section.path());
    let paths = crate::routes::PAGE_PATHS
        .into_iter()
        .map(str::to_string)
        .chain(account_paths);

    for path in paths {
        let status = status_of(&app, &path).await;
        assert_ne!(status, StatusCode::NOT_FOUND, "{path}");
    }

    let request = Request::post(crate::routes::LOGOUT_PATH)
        .header("host", "localhost")
        .body(Body::empty())
        .unwrap();
    let status = app.clone().oneshot(request).await.unwrap().status();
    assert!(
        ![StatusCode::NOT_FOUND, StatusCode::METHOD_NOT_ALLOWED].contains(&status),
        "POST {}: {status}",
        crate::routes::LOGOUT_PATH
    );
}
