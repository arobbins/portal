use std::sync::Arc;

use axum::http::{
    HeaderValue,
    header::{AUTHORIZATION, RETRY_AFTER, WWW_AUTHENTICATE},
};
use sqlx::PgPool;
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, request},
};

use crate::action::Channel;
use crate::action::{ActionProblem, Caller, ProblemSpec};
use crate::api::problem::{ApiProblem, ApiResponse, REALM};
use crate::api::scope::{ApiScope, TokenAudience};
use crate::api_tokens::lifecycle::{find_grant, record_use};
use crate::env::Env;
use crate::oauth::access::{app_caller, is_app_token, record_app_use};
use crate::user::queries::query_get_user_by_id;
use crate::web::env;

const BAD_TOKEN_RETRY_SECS: &str = "600";
const RATE_LIMIT_RETRY_SECS: &str = "1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiAuthProblem {
    MissingToken,
    InvalidToken,
    TooManyInvalidTokens,
    RateLimited,
}

impl ApiAuthProblem {
    pub const ALL: [Self; 4] = [
        Self::MissingToken,
        Self::InvalidToken,
        Self::TooManyInvalidTokens,
        Self::RateLimited,
    ];

    pub fn response(self) -> ApiResponse {
        let problem = ApiProblem::of(self);
        let problem = match self {
            Self::MissingToken => {
                problem.header(WWW_AUTHENTICATE, &format!("Bearer realm=\"{REALM}\""))
            }
            Self::InvalidToken => problem.header(
                WWW_AUTHENTICATE,
                &format!(
                    "Bearer realm=\"{REALM}\", error=\"invalid_token\", error_description=\"{}\"",
                    self.message()
                ),
            ),
            Self::TooManyInvalidTokens => problem.header(RETRY_AFTER, BAD_TOKEN_RETRY_SECS),
            Self::RateLimited => problem.header(RETRY_AFTER, RATE_LIMIT_RETRY_SECS),
        };

        problem.response()
    }
}

impl ActionProblem for ApiAuthProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::MissingToken => ProblemSpec::new(
                "unauthorized",
                "Send an API token in the Authorization header: Bearer <token>.",
            )
            .with_status(StatusCode::UNAUTHORIZED),
            Self::InvalidToken => ProblemSpec::new(
                "invalid_token",
                "The API token is invalid, expired or revoked.",
            )
            .with_status(StatusCode::UNAUTHORIZED),
            Self::TooManyInvalidTokens => ProblemSpec::new(
                "too_many_invalid_tokens",
                "Too many invalid API tokens from this address. Wait 10 minutes and try again.",
            )
            .with_status(StatusCode::TOO_MANY_REQUESTS),
            Self::RateLimited => ProblemSpec::new(
                "rate_limited",
                "This API token is sending requests too quickly. Slow down and retry after the Retry-After delay.",
            )
            .with_status(StatusCode::TOO_MANY_REQUESTS),
        }
    }
}

pub async fn authenticate(
    cx: &Cx,
    audience: TokenAudience,
) -> Result<std::result::Result<Caller, ApiAuthProblem>> {
    let header = request::headers(cx).get(AUTHORIZATION);
    authenticate_header(&env(cx), header, audience).await
}

async fn authenticate_header(
    env: &Env<'_>,
    header: Option<&HeaderValue>,
    audience: TokenAudience,
) -> Result<std::result::Result<Caller, ApiAuthProblem>> {
    let limits = env.limits();
    if limits.api_auth_blocked(env.ip).await? {
        return Ok(Err(ApiAuthProblem::TooManyInvalidTokens));
    }
    let caller = match check_bearer(header, audience, env.db(), env.now()).await? {
        Ok(caller) => caller,
        Err(problem) => {
            if problem == ApiAuthProblem::InvalidToken {
                limits.record_failed_api_auth(env.ip).await?;
            }
            return Ok(Err(problem));
        }
    };
    let allowed = take_call(env, &caller.via).await?;
    if !allowed {
        return Ok(Err(ApiAuthProblem::RateLimited));
    }

    Ok(Ok(caller))
}

async fn take_call(env: &Env<'_>, via: &Channel) -> std::result::Result<bool, sqlx::Error> {
    let limits = env.limits();
    let allowed = match via {
        Channel::App(app) => limits.allow_app_call(app.connection_id).await?,
        Channel::ApiToken { id, .. } => limits.allow_api_call(*id).await?,
        Channel::Browser => false,
    };
    match via {
        Channel::App(app) if allowed => {
            record_app_use(app.connection_id, env.now(), env.db()).await?
        }
        Channel::ApiToken { id, .. } if allowed => record_use(*id, env.now(), env.db()).await?,
        _ => {}
    }
    Ok(allowed)
}

async fn check_bearer(
    header: Option<&HeaderValue>,
    audience: TokenAudience,
    db: &PgPool,
    now: time::OffsetDateTime,
) -> std::result::Result<std::result::Result<Caller, ApiAuthProblem>, sqlx::Error> {
    let raw = match bearer_token(header) {
        Bearer::Missing => return Ok(Err(ApiAuthProblem::MissingToken)),
        Bearer::Malformed => return Ok(Err(ApiAuthProblem::InvalidToken)),
        Bearer::Token(raw) => raw,
    };
    if audience == TokenAudience::Mcp && is_app_token(raw) {
        return Ok(app_caller(raw, now, db)
            .await?
            .ok_or(ApiAuthProblem::InvalidToken));
    }
    let Some(grant) = find_grant(raw, audience, now, db).await? else {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    };
    let Some(user) = query_get_user_by_id(grant.user_id, db).await? else {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    };
    if !user.is_active() {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    }

    let scopes: Vec<ApiScope> = grant
        .scopes
        .into_iter()
        .filter(|scope| scope.available_to(&user))
        .collect();

    Ok(Ok(Caller::api_token(Arc::new(user), grant.id, scopes)))
}

enum Bearer<'a> {
    Missing,
    Malformed,
    Token(&'a str),
}

fn bearer_token(header: Option<&HeaderValue>) -> Bearer<'_> {
    let Some(header) = header else {
        return Bearer::Missing;
    };
    let Ok(value) = header.to_str() else {
        return Bearer::Malformed;
    };
    match value.split_once(' ') {
        Some((scheme, token)) if scheme.eq_ignore_ascii_case("bearer") && !token.is_empty() => {
            Bearer::Token(token.trim())
        }
        _ => Bearer::Malformed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Channel, Denied};
    use crate::api_tokens::testing::{delete_token, issue_token};
    use crate::rate_limit::BAD_API_TOKEN_WINDOW_MINUTES;

    use crate::user::model::User;
    use crate::user::testing::active_user;

    async fn issue(
        user: &User,
        scopes: &[&str],
        expires_in: time::Duration,
        db: &PgPool,
    ) -> (String, i32, String) {
        let token = issue_token(user.id, scopes, expires_in, db).await;
        (token.raw, token.id, token.public_id)
    }

    async fn check(raw: &str, db: &PgPool) -> std::result::Result<Caller, ApiAuthProblem> {
        check_for(raw, TokenAudience::Api, time::OffsetDateTime::now_utc(), db).await
    }

    async fn check_for(
        raw: &str,
        audience: TokenAudience,
        now: time::OffsetDateTime,
        db: &PgPool,
    ) -> std::result::Result<Caller, ApiAuthProblem> {
        let header = HeaderValue::from_str(&format!("Bearer {raw}")).unwrap();
        check_bearer(Some(&header), audience, db, now)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn a_sign_in_token_works_only_on_mcp(db: PgPool) {
        let app = crate::env::App::for_tests(db.clone());
        let user = active_user("audience@example.test", &db).await;
        let (_, tokens) =
            crate::oauth::testing::connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
        let now = app.clock.now();

        assert_eq!(
            check_for(&tokens.access_token, TokenAudience::Api, now, &db)
                .await
                .err(),
            Some(ApiAuthProblem::InvalidToken)
        );
        let caller = check_for(&tokens.access_token, TokenAudience::Mcp, now, &db)
            .await
            .unwrap();
        assert!(matches!(caller.via, Channel::App(_)));
    }

    #[test]
    fn the_lockout_message_matches_the_limit() {
        assert!(
            ApiAuthProblem::TooManyInvalidTokens
                .message()
                .contains(&format!("{BAD_API_TOKEN_WINDOW_MINUTES} minutes"))
        );
        assert_eq!(
            BAD_TOKEN_RETRY_SECS,
            (BAD_API_TOKEN_WINDOW_MINUTES * 60).to_string()
        );
    }

    #[test]
    fn only_the_bearer_scheme_is_accepted() {
        let token = HeaderValue::from_static("Bearer abc");
        let lowercase = HeaderValue::from_static("bearer abc");
        let basic = HeaderValue::from_static("Basic abc");
        let empty = HeaderValue::from_static("Bearer ");

        assert!(matches!(bearer_token(None), Bearer::Missing));
        assert!(matches!(bearer_token(Some(&token)), Bearer::Token("abc")));
        assert!(matches!(
            bearer_token(Some(&lowercase)),
            Bearer::Token("abc")
        ));
        assert!(matches!(bearer_token(Some(&basic)), Bearer::Malformed));
        assert!(matches!(bearer_token(Some(&empty)), Bearer::Malformed));
    }

    #[sqlx::test]
    async fn a_valid_token_becomes_a_caller_with_its_scopes(db: PgPool) {
        let user = active_user("api@example.test", &db).await;
        let (raw, id, _) = issue(&user, &["profile:read"], time::Duration::days(1), &db).await;

        let caller = check(&raw, &db).await.unwrap();

        assert_eq!(caller.user.id, user.id);
        assert_eq!(
            caller.via,
            Channel::ApiToken {
                id,
                scopes: vec![ApiScope::ProfileRead]
            }
        );
        assert_eq!(
            caller.require(crate::permission::Permission::EditProfile),
            Err(Denied::MissingScope(ApiScope::ProfileWrite))
        );
    }

    #[sqlx::test]
    async fn a_disabled_accounts_tokens_stop_working(db: PgPool) {
        let user = active_user("api@example.test", &db).await;
        let (raw, _, _) = issue(&user, &["profile:read"], time::Duration::days(1), &db).await;
        sqlx::query!(
            "UPDATE users SET disabled_at = now() WHERE id = $1",
            user.id
        )
        .execute(&db)
        .await
        .unwrap();

        assert_eq!(
            check(&raw, &db).await.unwrap_err(),
            ApiAuthProblem::InvalidToken
        );
    }

    #[sqlx::test]
    async fn expired_revoked_and_unknown_tokens_look_the_same(db: PgPool) {
        let user = active_user("api@example.test", &db).await;
        let (expired, expired_id, _) =
            issue(&user, &["profile:read"], time::Duration::days(1), &db).await;
        sqlx::query!(
            "UPDATE api_tokens SET created_at = now() - interval '2 days', expires_at = now() - interval '1 second' WHERE id = $1",
            expired_id,
        )
        .execute(&db)
        .await
        .unwrap();
        let (revoked, _, revoked_id) =
            issue(&user, &["profile:read"], time::Duration::days(1), &db).await;
        delete_token(&revoked_id, user.id, &db).await;
        let unknown = crate::api_tokens::token::generate_api_token()
            .raw
            .into_inner();

        for raw in [
            expired,
            revoked,
            unknown,
            "wsk_not-a-real-token".to_string(),
        ] {
            assert_eq!(
                check(&raw, &db).await.err(),
                Some(ApiAuthProblem::InvalidToken)
            );
        }
    }
}
