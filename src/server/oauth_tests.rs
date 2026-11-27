use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

use super::tests::test_app_with;

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, axum::http::HeaderMap, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, body)
}

fn get(path: &str) -> Request<Body> {
    Request::get(path)
        .header("host", "localhost")
        .body(Body::empty())
        .unwrap()
}

fn rpc(body: Value) -> Request<Body> {
    Request::post("/mcp")
        .header("host", "localhost")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test]
async fn the_approval_screen_is_never_cached_or_framed(db: PgPool) {
    let app = test_app_with(db);

    let (status, headers, _) = send(&app, get("/oauth/authorize?client_id=cli_unknown")).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(headers["x-frame-options"], "DENY");
    assert_eq!(headers["content-security-policy"], "frame-ancestors 'none'");
}

#[sqlx::test]
async fn starting_authorization_too_often_shows_a_429_page(db: PgPool) {
    let app = test_app_with(db);
    let path = "/oauth/authorize?client_id=cli_unknown";
    for _ in 0..crate::rate_limit::AUTHORIZATION_STARTS_PER_IP_PER_HOUR {
        let (status, _, _) = send(&app, get(path)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    let response = app.clone().oneshot(get(path)).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let page = String::from_utf8_lossy(&bytes);
    assert!(page.contains("Too many sign-in requests"), "{page}");
    assert!(page.contains("Try again later."), "{page}");
    assert!(!page.contains("Can&#39;t connect this app") && !page.contains("Can't connect"));
}

#[sqlx::test]
async fn both_metadata_documents_use_the_standard_fields(db: PgPool) {
    let app = test_app_with(db);

    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
    ] {
        let (status, _, resource) = send(&app, get(path)).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(resource["resource"], "http://localhost/mcp");
        assert_eq!(
            resource["authorization_servers"],
            json!(["http://localhost"])
        );
    }
    let (_, _, server) = send(&app, get("/.well-known/oauth-authorization-server")).await;
    assert_eq!(server["issuer"], "http://localhost");
    assert_eq!(
        server["authorization_endpoint"],
        "http://localhost/oauth/authorize"
    );
    assert_eq!(server["token_endpoint"], "http://localhost/oauth/token");
    assert_eq!(
        server["registration_endpoint"],
        "http://localhost/oauth/register"
    );
    assert_eq!(
        server["revocation_endpoint"],
        "http://localhost/oauth/revoke"
    );
    assert_eq!(server["code_challenge_methods_supported"], json!(["S256"]));
    assert_eq!(server["client_id_metadata_document_supported"], true);
}

#[sqlx::test]
async fn without_a_token_only_public_tools_work(db: PgPool) {
    let app = test_app_with(db);

    let init = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} });
    let (status, _, body) = send(&app, rpc(init)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["protocolVersion"].is_string());

    let list = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" });
    let (_, _, body) = send(&app, rpc(list)).await;
    let names: Vec<&str> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["list_plans"]);

    let call = json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "get_profile" } });
    let (status, headers, _) = send(&app, rpc(call)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        headers["www-authenticate"],
        "Bearer resource_metadata=\"http://localhost/.well-known/oauth-protected-resource/mcp\""
    );
}

#[sqlx::test]
async fn a_bad_token_on_mcp_points_to_the_metadata_too(db: PgPool) {
    let app = test_app_with(db);
    let mut request = rpc(json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }));
    request
        .headers_mut()
        .insert("authorization", "Bearer wsa_nonsense".parse().unwrap());

    let (status, headers, _) = send(&app, request).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let challenge = headers["www-authenticate"].to_str().unwrap();
    assert!(challenge.contains(
        "resource_metadata=\"http://localhost/.well-known/oauth-protected-resource/mcp\""
    ));
    assert!(challenge.contains("error=\"invalid_token\""));
}

#[sqlx::test]
async fn clients_register_with_json_and_get_a_client_id(db: PgPool) {
    let app = test_app_with(db);
    let register = |body: Value| {
        Request::post("/oauth/register")
            .header("host", "localhost")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };

    let good = json!({
        "client_name": "Claude",
        "redirect_uris": ["https://claude.ai/api/mcp/auth_callback"],
        "token_endpoint_auth_method": "none",
        "software_id": "ignored-unknown-metadata",
    });
    let (status, _, body) = send(&app, register(good)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(body["client_id"].as_str().unwrap().starts_with("cli_"));

    let bad = json!({ "client_name": "Evil", "redirect_uris": ["http://evil.test/cb"] });
    let (status, _, body) = send(&app, register(bad)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_redirect_uri");
}
