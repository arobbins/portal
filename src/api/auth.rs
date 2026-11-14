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

use crate::action::{ActionProblem, Caller};
use crate::api::problem::{ApiProblem, ApiResponse, REALM};
use crate::api::scope::{ApiScope, TokenAudience, parse_scopes};
use crate::api_tokens::queries::{query_get_api_token_by_hash, query_record_api_token_use};
use crate::api_tokens::token::is_well_formed;
use crate::secret::hash_secret;
use crate::user::model::AccountStatus;
use crate::user::queries::query_get_user_by_id;
use crate::web::{db, rate_limits};

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
    fn status(self) -> StatusCode {
        match self {
            Self::MissingToken | Self::InvalidToken => StatusCode::UNAUTHORIZED,
            Self::TooManyInvalidTokens | Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::MissingToken => "unauthorized",
            Self::InvalidToken => "invalid_token",
            Self::TooManyInvalidTokens => "too_many_invalid_tokens",
            Self::RateLimited => "rate_limited",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::MissingToken => "Send an API token in the Authorization header: Bearer <token>.",
            Self::InvalidToken => "The API token is invalid, expired or revoked.",
            Self::TooManyInvalidTokens => {
                "Too many invalid API tokens from this address. Wait 10 minutes and try again."
            }
            Self::RateLimited => {
                "This API token is sending requests too quickly. Slow down and retry after the Retry-After delay."
            }
        }
    }
}

pub async fn authenticate(
    cx: &Cx,
    audience: TokenAudience,
) -> Result<std::result::Result<Caller, ApiAuthProblem>> {
    let limits = rate_limits(cx);
    let ip = request::client_ip(cx);
    if limits.api_auth_blocked(ip) {
        return Ok(Err(ApiAuthProblem::TooManyInvalidTokens));
    }

    let header = request::headers(cx).get(AUTHORIZATION);
    let caller = match check_bearer(header, audience, db(cx)).await? {
        Ok(caller) => caller,
        Err(problem) => {
            if problem == ApiAuthProblem::InvalidToken {
                limits.record_failed_api_auth(ip);
            }
            return Ok(Err(problem));
        }
    };
    let token_id = caller.api_token_id().unwrap_or_default();
    if !limits.allow_api_call(token_id) {
        return Ok(Err(ApiAuthProblem::RateLimited));
    }
    query_record_api_token_use(token_id, db(cx)).await?;

    Ok(Ok(caller))
}

async fn check_bearer(
    header: Option<&HeaderValue>,
    audience: TokenAudience,
    db: &PgPool,
) -> std::result::Result<std::result::Result<Caller, ApiAuthProblem>, sqlx::Error> {
    let raw = match bearer_token(header) {
        Bearer::Missing => return Ok(Err(ApiAuthProblem::MissingToken)),
        Bearer::Malformed => return Ok(Err(ApiAuthProblem::InvalidToken)),
        Bearer::Token(raw) => raw,
    };
    if !is_well_formed(raw) {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    }

    let hash = hash_secret(raw);
    let Some(grant) = query_get_api_token_by_hash(&hash, db).await? else {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    };
    if grant.audience != audience.as_str() {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    }
    let Some(user) = query_get_user_by_id(grant.user_id, db).await? else {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    };
    if !matches!(user.status(), AccountStatus::Active { .. }) {
        return Ok(Err(ApiAuthProblem::InvalidToken));
    }

    let scopes: Vec<ApiScope> = parse_scopes(&grant.scopes)
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
    use crate::api_tokens::queries::{
        NewApiTokenRow, query_delete_api_token, query_insert_api_token,
    };
    use crate::api_tokens::token::generate_api_token;
    use crate::auth::password::hash_password;
    use crate::rate_limit::BAD_API_TOKEN_WINDOW_MINUTES;
    use crate::secret::PublicIdKind;
    use crate::user::model::User;
    use crate::user::queries::{NewUser, query_insert_user, query_set_password_and_verify_user};

    async fn active_user(db: &PgPool) -> User {
        let user_id = query_insert_user(
            &NewUser {
                name: "Api Tester",
                email: "api@example.test",
            },
            db,
        )
        .await
        .unwrap()
        .unwrap();
        let password_hash = hash_password("a-strong-test-password").unwrap();
        query_set_password_and_verify_user(user_id, &password_hash, db)
            .await
            .unwrap()
    }

    async fn issue(
        user: &User,
        scopes: &[&str],
        expires_in: time::Duration,
        db: &PgPool,
    ) -> (String, i32, String) {
        let token = generate_api_token();
        let public_id = PublicIdKind::ApiToken.generate();
        let scopes: Vec<String> = scopes.iter().map(|scope| scope.to_string()).collect();
        let id = query_insert_api_token(
            &NewApiTokenRow {
                user_id: user.id,
                public_id: &public_id,
                name: "Test token",
                token_hash: &token.hash,
                last_four: &token.last_four,
                scopes: &scopes,
                audience: TokenAudience::Api.as_str(),
                expires_at: time::OffsetDateTime::now_utc() + expires_in,
                max_per_user: 50,
            },
            db,
        )
        .await
        .unwrap()
        .unwrap();
        (token.raw, id, public_id)
    }

    async fn check(raw: &str, db: &PgPool) -> std::result::Result<Caller, ApiAuthProblem> {
        let header = HeaderValue::from_str(&format!("Bearer {raw}")).unwrap();
        check_bearer(Some(&header), TokenAudience::Api, db)
            .await
            .unwrap()
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
        let user = active_user(&db).await;
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
            caller.require(ApiScope::ProfileWrite),
            Err(Denied::MissingScope(ApiScope::ProfileWrite))
        );
    }

    #[sqlx::test]
    async fn expired_revoked_and_unknown_tokens_look_the_same(db: PgPool) {
        let user = active_user(&db).await;
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
        query_delete_api_token(&revoked_id, user.id, &db)
            .await
            .unwrap();
        let unknown = generate_api_token().raw;

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

    #[sqlx::test]
    async fn a_token_cannot_be_revoked_by_another_user(db: PgPool) {
        let user = active_user(&db).await;
        let (raw, _, id) = issue(&user, &["profile:read"], time::Duration::days(1), &db).await;

        assert_eq!(
            query_delete_api_token(&id, user.id + 1, &db).await.unwrap(),
            None
        );
        assert!(check(&raw, &db).await.is_ok());
    }

    #[sqlx::test]
    async fn each_user_has_a_token_limit(db: PgPool) {
        let user = active_user(&db).await;
        let token = generate_api_token();
        let scopes = vec!["profile:read".to_string()];
        let first_id = PublicIdKind::ApiToken.generate();
        let row = NewApiTokenRow {
            user_id: user.id,
            public_id: &first_id,
            name: "Limited",
            token_hash: &token.hash,
            last_four: &token.last_four,
            scopes: &scopes,
            audience: TokenAudience::Api.as_str(),
            expires_at: time::OffsetDateTime::now_utc() + time::Duration::days(1),
            max_per_user: 1,
        };
        assert!(query_insert_api_token(&row, &db).await.unwrap().is_some());

        let second = generate_api_token();
        let second_id = PublicIdKind::ApiToken.generate();
        let row = NewApiTokenRow {
            public_id: &second_id,
            token_hash: &second.hash,
            last_four: &second.last_four,
            ..row
        };
        assert_eq!(query_insert_api_token(&row, &db).await.unwrap(), None);
    }
}
