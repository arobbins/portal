use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, Postgres, Transaction};
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, Outcome, ProblemSpec, fail};
use crate::env::Env;
use crate::oauth::authorize::mcp_resource;
use crate::oauth::lifecycle::{active_owner, record_disconnected};
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
    let grant_type = match input.grant_type.as_deref() {
        Some("authorization_code") => GrantType::AuthorizationCode,
        Some("refresh_token") => GrantType::RefreshToken,
        _ => return fail(TokenProblem::UnsupportedGrantType),
    };
    let mut tx = env.db().begin().await?;
    let granted = match grant_type {
        GrantType::AuthorizationCode => exchange_code(env, client_id, input, &mut tx).await?,
        GrantType::RefreshToken => refresh(env, client_id, input, &mut tx).await?,
    };
    let connection_id = match granted {
        Granted::Connection(id) => id,
        refused => return refuse(env, refused, tx).await,
    };
    let Some(response) = store_tokens(env, connection_id, &mut tx).await? else {
        return fail(TokenProblem::InvalidGrant);
    };
    tx.commit().await?;
    Ok(Ok(response))
}

enum GrantType {
    AuthorizationCode,
    RefreshToken,
}

enum Granted {
    Connection(i32),
    Refused(TokenProblem),
    CodeSpent,
    Reused(i32),
}

async fn refuse(
    env: &Env<'_>,
    granted: Granted,
    tx: Transaction<'_, Postgres>,
) -> Outcome<TokenResponse, TokenProblem> {
    match granted {
        Granted::Refused(problem) => {
            tx.rollback().await?;
            fail(problem)
        }
        Granted::Connection(_) => {
            tx.rollback().await?;
            fail(TokenProblem::InvalidGrant)
        }
        Granted::CodeSpent => {
            tx.commit().await?;
            fail(TokenProblem::InvalidGrant)
        }
        Granted::Reused(connection_id) => {
            tx.rollback().await?;
            end_connection(env, connection_id).await?;
            fail(TokenProblem::InvalidGrant)
        }
    }
}

async fn exchange_code(
    env: &Env<'_>,
    client_id: &str,
    input: &TokenRequest,
    tx: &mut PgConnection,
) -> Result<Granted, sqlx::Error> {
    let (Some(code), Some(verifier)) = (input.code.as_deref(), input.code_verifier.as_deref())
    else {
        return Ok(Granted::Refused(TokenProblem::InvalidRequest));
    };
    let Some(hash) = SecretKind::Code.hash_of(code) else {
        return Ok(Granted::Refused(TokenProblem::InvalidGrant));
    };
    let Some(issued) = query_consume_code(&hash, &mut *tx).await? else {
        return Ok(Granted::Refused(TokenProblem::InvalidGrant));
    };
    let valid = issued.client_id == client_id
        && input.redirect_uri.as_deref() == Some(issued.redirect_uri.as_str())
        && env.now() - issued.acted_at < CODE_LIFETIME
        && challenge_of(verifier) == issued.code_challenge;
    if !valid {
        return Ok(Granted::CodeSpent);
    }
    Ok(Granted::Connection(issued.connection_id))
}

pub fn challenge_of(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

async fn refresh(
    env: &Env<'_>,
    client_id: &str,
    input: &TokenRequest,
    tx: &mut PgConnection,
) -> Result<Granted, sqlx::Error> {
    let refused = |problem| Ok(Granted::Refused(problem));
    let Some(raw) = input.refresh_token.as_deref() else {
        return refused(TokenProblem::InvalidRequest);
    };
    let Some(hash) = SecretKind::RefreshToken.hash_of(raw) else {
        return refused(TokenProblem::InvalidGrant);
    };
    let Some(token) = query_get_refresh_token(&hash, &mut *tx).await? else {
        return refused(TokenProblem::InvalidGrant);
    };
    if token.client_id != client_id || token.expires_at <= env.now() {
        return refused(TokenProblem::InvalidGrant);
    }
    if token.replaced_at.is_some() {
        return Ok(Granted::Reused(token.connection_id));
    }
    if active_owner(token.user_id, &mut *tx).await?.is_none() {
        return refused(TokenProblem::InvalidGrant);
    }
    if !query_replace_refresh_token(token.id, env.now(), &mut *tx).await? {
        return Ok(Granted::Reused(token.connection_id));
    }
    Ok(Granted::Connection(token.connection_id))
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
