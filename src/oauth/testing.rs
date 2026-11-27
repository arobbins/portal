use std::sync::Arc;

use sqlx::PgPool;

use crate::action::Caller;
use crate::api::scope::ApiScope;
use crate::env::{App, Env};
use crate::oauth::actions::{ConnectApp, connect_app};
use crate::oauth::authorize::{AuthorizeParams, Started, mcp_resource, start_authorization};
use crate::oauth::grant::{TokenRequest, TokenResponse, challenge_of, issue_tokens};
use crate::oauth::queries::{NewClient, query_insert_client};
use crate::secret::PublicIdKind;
use crate::user::model::User;
use crate::user::testing::TEST_PASSWORD;

pub const REDIRECT_URI: &str = "https://claude.ai/api/mcp/auth_callback";
pub const VERIFIER: &str = "a-test-code-verifier-that-is-long-enough-for-pkce-0123";

pub async fn test_client(db: &PgPool) -> String {
    named_client("Claude", db).await
}

pub async fn named_client(name: &str, db: &PgPool) -> String {
    let client_id = PublicIdKind::OAuthClient.generate();
    let client = NewClient {
        client_id: &client_id,
        name,
        redirect_uris: &[REDIRECT_URI.to_string()],
        fetched_at: None,
    };
    query_insert_client(&client, db).await.unwrap();
    client_id
}

pub fn authorize_params(env: &Env<'_>, client_id: &str, scope: &str) -> AuthorizeParams {
    AuthorizeParams {
        response_type: Some("code".to_string()),
        client_id: Some(client_id.to_string()),
        redirect_uri: Some(REDIRECT_URI.to_string()),
        scope: Some(scope.to_string()),
        state: Some("state-1".to_string()),
        code_challenge: Some(challenge_of(VERIFIER)),
        code_challenge_method: Some("S256".to_string()),
        resource: Some(mcp_resource(env)),
    }
}

pub async fn start_request(env: &Env<'_>, client_id: &str, scope: &str) -> String {
    match start_authorization(env, &authorize_params(env, client_id, scope))
        .await
        .unwrap()
    {
        Started::Request(id) => id,
        other => panic!("authorization didn't start: {other:?}"),
    }
}

pub async fn approve(
    env: &Env<'_>,
    user: &Arc<User>,
    request: &str,
    scopes: &[ApiScope],
) -> String {
    let input = ConnectApp {
        request: request.to_string(),
        scopes: scopes.to_vec(),
        current_password: TEST_PASSWORD.to_string(),
    };
    let redirect = connect_app(env, &Caller::browser(user.clone()), &input, None)
        .await
        .unwrap()
        .unwrap();
    let url = url::Url::parse(&redirect).unwrap();
    url.query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, code)| code.into_owned())
        .unwrap()
}

pub fn code_exchange(env: &Env<'_>, client_id: &str, code: &str) -> TokenRequest {
    TokenRequest {
        grant_type: Some("authorization_code".to_string()),
        code: Some(code.to_string()),
        redirect_uri: Some(REDIRECT_URI.to_string()),
        client_id: Some(client_id.to_string()),
        code_verifier: Some(VERIFIER.to_string()),
        refresh_token: None,
        resource: Some(mcp_resource(env)),
    }
}

pub fn refresh_request(env: &Env<'_>, client_id: &str, refresh_token: &str) -> TokenRequest {
    TokenRequest {
        grant_type: Some("refresh_token".to_string()),
        client_id: Some(client_id.to_string()),
        refresh_token: Some(refresh_token.to_string()),
        resource: Some(mcp_resource(env)),
        ..TokenRequest::default()
    }
}

pub async fn connect_test_app(
    app: &App,
    user: &Arc<User>,
    scopes: &[ApiScope],
) -> (String, TokenResponse) {
    connect_named_app(app, user, "Claude", scopes).await
}

pub async fn connect_named_app(
    app: &App,
    user: &Arc<User>,
    name: &str,
    scopes: &[ApiScope],
) -> (String, TokenResponse) {
    let env = app.env(None);
    let client_id = named_client(name, &app.db).await;
    let scope: Vec<&str> = scopes.iter().map(|scope| scope.as_str()).collect();
    let request = start_request(&env, &client_id, &scope.join(" ")).await;
    let code = approve(&env, user, &request, scopes).await;
    let tokens = issue_tokens(&env, &code_exchange(&env, &client_id, &code))
        .await
        .unwrap()
        .unwrap();
    (client_id, tokens)
}

pub async fn app_still_works(app: &App, tokens: &TokenResponse) -> bool {
    crate::oauth::lifecycle::app_caller(&tokens.access_token, app.clock.now(), &app.db)
        .await
        .unwrap()
        .is_some()
}
