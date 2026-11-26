use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, Outcome, ProblemSpec, fail};
use crate::env::Env;
use crate::oauth::authorize::mcp_resource;
use crate::oauth::lifecycle::record_disconnected;
use crate::oauth::queries::{
    NewToken, query_consume_code, query_delete_access_tokens, query_delete_connection_by_id,
    query_get_connection_scopes, query_get_refresh_token, query_get_token_owner,
    query_insert_token, query_replace_refresh_token,
};
use crate::oauth::secrets::{ACCESS_TOKEN_LIFETIME, CODE_LIFETIME, SecretKind};
use crate::user::events::{EventChannel, EventSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProblem {
    InvalidRequest,
    InvalidGrant,
    UnsupportedGrantType,
    TooManyRequests,
}

impl ActionProblem for TokenProblem {
    fn spec(self) -> ProblemSpec {
        let bad = StatusCode::BAD_REQUEST;
        match self {
            Self::InvalidRequest => ProblemSpec::new(
                "invalid_request",
                "Send client_id, resource and either code, redirect_uri and code_verifier or refresh_token.",
            )
            .with_status(bad),
            Self::InvalidGrant => ProblemSpec::new(
                "invalid_grant",
                "The code or refresh token is invalid, expired, already used or for another client or resource.",
            )
            .with_status(bad),
            Self::UnsupportedGrantType => ProblemSpec::new(
                "unsupported_grant_type",
                "grant_type must be authorization_code or refresh_token.",
            )
            .with_status(bad),
            Self::TooManyRequests => ProblemSpec::new(
                "too_many_requests",
                "This client is asking for tokens too often. Wait a minute and try again.",
            )
            .with_status(StatusCode::TOO_MANY_REQUESTS),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TokenRequest {
    pub grant_type: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub scope: String,
}

pub async fn issue_tokens(
    env: &Env<'_>,
    input: &TokenRequest,
) -> Outcome<TokenResponse, TokenProblem> {
    let (Some(client_id), Some(resource)) = (input.client_id.as_deref(), input.resource.as_deref())
    else {
        return fail(TokenProblem::InvalidRequest);
    };
    if !env.limits().allow_token_request(client_id).await? {
        return fail(TokenProblem::TooManyRequests);
    }
    if resource != mcp_resource(env) {
        return fail(TokenProblem::InvalidGrant);
    }
    let connection = match input.grant_type.as_deref() {
        Some("authorization_code") => exchange_code(env, client_id, input).await?,
        Some("refresh_token") => refresh(env, client_id, input).await?,
        _ => return fail(TokenProblem::UnsupportedGrantType),
    };
    let connection_id = match connection {
        Ok(id) => id,
        Err(problem) => return fail(problem),
    };
    let mut tx = env.db().begin().await?;
    let response = store_tokens(env, connection_id, &mut tx).await?;
    tx.commit().await?;
    Ok(response.ok_or(TokenProblem::InvalidGrant))
}

async fn exchange_code(
    env: &Env<'_>,
    client_id: &str,
    input: &TokenRequest,
) -> Outcome<i32, TokenProblem> {
    let (Some(code), Some(verifier)) = (input.code.as_deref(), input.code_verifier.as_deref())
    else {
        return fail(TokenProblem::InvalidRequest);
    };
    let Some(hash) = SecretKind::Code.hash_of(code) else {
        return fail(TokenProblem::InvalidGrant);
    };
    let Some(issued) = query_consume_code(&hash, env.db()).await? else {
        return fail(TokenProblem::InvalidGrant);
    };
    let valid = issued.client_id == client_id
        && input
            .redirect_uri
            .as_deref()
            .is_none_or(|uri| uri == issued.redirect_uri)
        && env.now() - issued.acted_at < CODE_LIFETIME
        && challenge_of(verifier) == issued.code_challenge;
    if !valid {
        return fail(TokenProblem::InvalidGrant);
    }
    Ok(Ok(issued.connection_id))
}

pub fn challenge_of(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

async fn refresh(
    env: &Env<'_>,
    client_id: &str,
    input: &TokenRequest,
) -> Outcome<i32, TokenProblem> {
    let Some(raw) = input.refresh_token.as_deref() else {
        return fail(TokenProblem::InvalidRequest);
    };
    let Some(hash) = SecretKind::RefreshToken.hash_of(raw) else {
        return fail(TokenProblem::InvalidGrant);
    };
    let Some(token) = query_get_refresh_token(&hash, env.db()).await? else {
        return fail(TokenProblem::InvalidGrant);
    };
    if token.client_id != client_id || token.expires_at <= env.now() {
        return fail(TokenProblem::InvalidGrant);
    }
    if token.replaced_at.is_some()
        || !query_replace_refresh_token(token.id, env.now(), env.db()).await?
    {
        end_connection(env, token.connection_id).await?;
        return fail(TokenProblem::InvalidGrant);
    }
    Ok(Ok(token.connection_id))
}

async fn store_tokens(
    env: &Env<'_>,
    connection_id: i32,
    tx: &mut PgConnection,
) -> Result<Option<TokenResponse>, sqlx::Error> {
    query_delete_access_tokens(connection_id, &mut *tx).await?;
    let access = SecretKind::AccessToken.generate();
    let refresh = SecretKind::RefreshToken.generate();
    for (kind, secret) in [
        (SecretKind::AccessToken, &access),
        (SecretKind::RefreshToken, &refresh),
    ] {
        let token = NewToken {
            connection_id,
            kind: kind.as_str(),
            token_hash: &secret.hash,
            expires_at: env.now() + kind.lifetime(),
        };
        query_insert_token(&token, &mut *tx).await?;
    }
    let scopes = query_get_connection_scopes(connection_id, &mut *tx).await?;
    Ok(scopes.map(|scopes| TokenResponse {
        access_token: access.raw,
        refresh_token: refresh.raw,
        token_type: "Bearer",
        expires_in: ACCESS_TOKEN_LIFETIME.whole_seconds(),
        scope: scopes.join(" "),
    }))
}

pub async fn end_connection(env: &Env<'_>, connection_id: i32) -> topcoat::Result<()> {
    let mut tx = env.db().begin().await?;
    if let Some(ended) = query_delete_connection_by_id(connection_id, &mut *tx).await? {
        let source = EventSource {
            user_id: ended.user_id,
            channel: EventChannel::App,
            app: Some(crate::oauth::app_label(&ended.name, &ended.return_host)),
        };
        record_disconnected(env, &source, &ended, &mut tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn revoke_token(env: &Env<'_>, token: &str, client_id: &str) -> topcoat::Result<()> {
    let hash = SecretKind::AccessToken
        .hash_of(token)
        .or_else(|| SecretKind::RefreshToken.hash_of(token));
    let Some(hash) = hash else {
        return Ok(());
    };
    match query_get_token_owner(&hash, env.db()).await? {
        Some(owner) if owner.client_id == client_id => {
            end_connection(env, owner.connection_id).await
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
