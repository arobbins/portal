use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, BodyLimit, StatusCode, href,
        request::{Bytes, FromRequest},
        route,
    },
};

use crate::api::body::{MAX_JSON_BODY, read_json};
use crate::api::problem::{ApiResponse, ok};
use crate::api::scope::ApiScope;
use crate::app::SITE_NAME;
use crate::mcp::mcp_endpoint;
use crate::oauth::authorize::issuer;
use crate::oauth::client::{ClientMetadata, register_client};
use crate::oauth::grant::{TokenProblem, TokenRequest, issue_tokens, revoke_token};
use crate::oauth::pages::authorize_page::authorize_page;
use crate::oauth::response::{oauth_error, oauth_json};
use crate::web::{config, env};

const MAX_FORM_BODY: usize = 16 * 1024;

fn scopes_supported() -> Vec<&'static str> {
    ApiScope::ALL
        .into_iter()
        .filter(|scope| scope.offered_by_sign_in())
        .map(ApiScope::as_str)
        .collect()
}

fn protected_resource(cx: &Cx) -> ApiResponse {
    let config = config(cx);
    ok(&json!({
        "resource": config.url(&href!(mcp_endpoint).resolve(cx)),
        "authorization_servers": [issuer(&env(cx))],
        "scopes_supported": scopes_supported(),
        "bearer_methods_supported": ["header"],
        "resource_name": SITE_NAME,
    }))
}

#[route(GET "/.well-known/oauth-protected-resource")]
pub async fn protected_resource_metadata(cx: &Cx) -> Result<ApiResponse> {
    Ok(protected_resource(cx))
}

#[route(GET "/.well-known/oauth-protected-resource/mcp")]
pub async fn mcp_resource_metadata(cx: &Cx) -> Result<ApiResponse> {
    Ok(protected_resource(cx))
}

#[route(GET "/.well-known/oauth-authorization-server")]
pub async fn authorization_server_metadata(cx: &Cx) -> Result<ApiResponse> {
    let url = |path: String| config(cx).url(&path);
    Ok(ok(&json!({
        "issuer": issuer(&env(cx)),
        "authorization_endpoint": url(href!(authorize_page).resolve(cx)),
        "token_endpoint": url(href!(token_endpoint).resolve(cx)),
        "registration_endpoint": url(href!(registration_endpoint).resolve(cx)),
        "revocation_endpoint": url(href!(revocation_endpoint).resolve(cx)),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "revocation_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": scopes_supported(),
        "client_id_metadata_document_supported": true,
        "authorization_response_iss_parameter_supported": true,
    })))
}

#[route(POST "/oauth/register", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn registration_endpoint(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let metadata: ClientMetadata = match read_json(cx, body).await? {
        Ok(metadata) => metadata,
        Err(response) => return Ok(response),
    };
    Ok(match register_client(&env(cx), &metadata).await? {
        Ok(client) => oauth_json(StatusCode::CREATED, serde_json::to_value(client)?),
        Err(problem) => oauth_error(problem),
    })
}

async fn read_form<T: DeserializeOwned + Default>(cx: &Cx, body: Body) -> T {
    let bytes = Bytes::from_request(cx, body).await.unwrap_or_default();
    let fields: Map<String, Value> = url::form_urlencoded::parse(&bytes)
        .into_owned()
        .map(|(key, value)| (key, Value::String(value)))
        .collect();
    serde_json::from_value(Value::Object(fields)).unwrap_or_default()
}

#[route(POST "/oauth/token", with(BodyLimit::max(MAX_FORM_BODY)))]
pub async fn token_endpoint(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let input: TokenRequest = read_form(cx, body).await;
    Ok(match issue_tokens(&env(cx), &input).await? {
        Ok(tokens) => oauth_json(StatusCode::OK, serde_json::to_value(tokens)?),
        Err(problem) => oauth_error(problem),
    })
}

#[derive(Debug, Default, serde::Deserialize)]
struct RevokeRequest {
    token: Option<String>,
    client_id: Option<String>,
}

#[route(POST "/oauth/revoke", with(BodyLimit::max(MAX_FORM_BODY)))]
pub async fn revocation_endpoint(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let input: RevokeRequest = read_form(cx, body).await;
    let (Some(token), Some(client_id)) = (input.token, input.client_id) else {
        return Ok(oauth_error(TokenProblem::InvalidRequest));
    };
    revoke_token(&env(cx), &token, &client_id).await?;
    Ok(oauth_json(StatusCode::OK, json!({})))
}
