use serde::Deserialize;

use crate::api::scope::ApiScope;
use crate::env::Env;
use crate::oauth::client::find_client;
use crate::oauth::queries::{
    NewRequest, RequestRow, RequestStatus, query_get_request, query_insert_request,
    query_touch_request,
};
use crate::oauth::redirect::with_params;
use crate::oauth::secrets::REQUEST_IDLE_LIMIT;
use crate::routes::{MCP_PATH, OAUTH_AUTHORIZE_PATH};
use crate::secret::PublicIdKind;

const MAX_STATE_LENGTH: usize = 1024;
const CHALLENGE_LENGTH: usize = 43;
pub const REQUEST_PARAM: &str = "request";

pub fn request_path(request: &str) -> String {
    format!("{OAUTH_AUTHORIZE_PATH}?{REQUEST_PARAM}={request}")
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthorizeParams {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CantConnect {
    UnknownClient,
    UnregisteredRedirect,
    Expired,
    TooManyRequests,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Started {
    Request(String),
    Refused(CantConnect),
    ReturnError(String),
}

fn requested_scopes(scope: &str) -> Vec<ApiScope> {
    let mut scopes: Vec<ApiScope> = Vec::new();
    for scope in scope.split_whitespace().filter_map(ApiScope::from_str) {
        if scope.offered_by_sign_in() && !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }
    scopes
}

pub fn mcp_resource(env: &Env<'_>) -> String {
    env.app.url(MCP_PATH)
}

pub fn issuer(env: &Env<'_>) -> String {
    env.app.config.origin().trim_end_matches('/').to_string()
}

pub async fn start_authorization(
    env: &Env<'_>,
    params: &AuthorizeParams,
) -> topcoat::Result<Started> {
    if !env.limits().allow_authorization_start(env.ip).await? {
        return Ok(Started::Refused(CantConnect::TooManyRequests));
    }
    let client_id = params.client_id.as_deref().unwrap_or_default();
    let Some(client) = find_client(env, client_id).await? else {
        return Ok(Started::Refused(CantConnect::UnknownClient));
    };
    let Some(redirect_uri) = registered_redirect(&client.redirect_uris, params) else {
        return Ok(Started::Refused(CantConnect::UnregisteredRedirect));
    };
    let state = params.state.as_deref();
    if let Some(error) = request_error(env, params) {
        return Ok(Started::ReturnError(error_redirect(
            env,
            redirect_uri,
            error,
            state,
        )));
    }
    let public_id = store_request(env, client.id, redirect_uri, params).await?;
    Ok(Started::Request(public_id))
}

async fn store_request(
    env: &Env<'_>,
    client_id: i32,
    redirect_uri: &str,
    params: &AuthorizeParams,
) -> Result<String, sqlx::Error> {
    let scopes = requested_scopes(params.scope.as_deref().unwrap_or_default());
    let public_id = PublicIdKind::AuthorizationRequest.generate();
    let request = NewRequest {
        public_id: &public_id,
        client_id,
        redirect_uri,
        scopes: &scopes,
        state: params.state.as_deref(),
        code_challenge: params.code_challenge.as_deref().unwrap_or_default(),
        now: env.now(),
    };
    query_insert_request(&request, env.db()).await?;
    Ok(public_id)
}

fn registered_redirect<'a>(registered: &'a [String], params: &AuthorizeParams) -> Option<&'a str> {
    let wanted = match params.redirect_uri.as_deref() {
        Some(uri) => uri,
        None if registered.len() == 1 => registered[0].as_str(),
        None => return None,
    };
    registered
        .iter()
        .find(|known| known.as_str() == wanted)
        .map(String::as_str)
}

fn request_error(env: &Env<'_>, params: &AuthorizeParams) -> Option<&'static str> {
    if params.response_type.as_deref() != Some("code") {
        return Some("unsupported_response_type");
    }
    let challenge = params.code_challenge.as_deref().unwrap_or_default();
    let pkce = params.code_challenge_method.as_deref() == Some("S256")
        && challenge.len() == CHALLENGE_LENGTH
        && challenge
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    let state_fits = params
        .state
        .as_deref()
        .is_none_or(|state| state.len() <= MAX_STATE_LENGTH);
    if !pkce || !state_fits {
        return Some("invalid_request");
    }
    if params.resource.as_deref() != Some(mcp_resource(env).as_str()) {
        return Some("invalid_target");
    }
    None
}

pub(super) fn error_redirect(
    env: &Env<'_>,
    redirect_uri: &str,
    error: &str,
    state: Option<&str>,
) -> String {
    let issuer = issuer(env);
    let mut params = vec![("error", error), ("iss", issuer.as_str())];
    if let Some(state) = state {
        params.push(("state", state));
    }
    with_params(redirect_uri, &params)
}

pub async fn live_request(env: &Env<'_>, public_id: &str) -> topcoat::Result<Option<RequestRow>> {
    if !PublicIdKind::AuthorizationRequest.is_valid(public_id) {
        return Ok(None);
    }
    let request = query_get_request(public_id, env.db()).await?;
    Ok(request.filter(|request| {
        request.status == RequestStatus::Pending
            && env.now() - request.acted_at < REQUEST_IDLE_LIMIT
    }))
}

pub async fn keep_request_alive(env: &Env<'_>, public_id: &str) -> topcoat::Result<()> {
    if let Some(request) = live_request(env, public_id).await? {
        let cutoff = env.now() - REQUEST_IDLE_LIMIT;
        query_touch_request(request.id, env.now(), cutoff, env.db()).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
