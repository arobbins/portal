use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use topcoat::{context::Cx, router::StatusCode};

use crate::account::tabs::AccountSection;
use crate::action::{ActionProblem, ActionResult, Caller, authorize, fail};
use crate::api::scope::{ApiScope, TokenAudience};
use crate::api_tokens::emails::api_token_created_email;
use crate::api_tokens::queries::{
    ApiTokenSummary, NewApiTokenRow, query_delete_api_token, query_insert_api_token,
    query_list_api_tokens,
};
use crate::api_tokens::token::generate_api_token;
use crate::auth::password::{CurrentPasswordProblem, check_current_password};
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::name::{MAX_NAME_LENGTH, NAME_FIELD, NameProblem, check_name};
use crate::web::{app_url, db, mailer, rate_limits};

const MAX_TOKENS_PER_USER: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum TokenLifetime {
    #[serde(rename = "7")]
    Week,
    #[serde(rename = "30")]
    Month,
    #[serde(rename = "90")]
    Quarter,
    #[serde(rename = "365")]
    Year,
}

impl TokenLifetime {
    pub const ALL: [Self; 4] = [Self::Week, Self::Month, Self::Quarter, Self::Year];
    pub const DEFAULT: Self = Self::Quarter;

    pub fn days(self) -> i64 {
        match self {
            Self::Week => 7,
            Self::Month => 30,
            Self::Quarter => 90,
            Self::Year => 365,
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::Week => "7",
            Self::Month => "30",
            Self::Quarter => "90",
            Self::Year => "365",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Week => "7 days",
            Self::Month => "30 days",
            Self::Quarter => "90 days",
            Self::Year => "1 year",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateTokenProblem {
    CurrentPassword(CurrentPasswordProblem),
    Name(NameProblem),
    UnknownAccess,
    TooManyTokens,
}

impl ActionProblem for CreateTokenProblem {
    fn code(self) -> &'static str {
        match self {
            Self::CurrentPassword(problem) => problem.code(),
            Self::Name(problem) => problem.code(),
            Self::UnknownAccess => "unknown_access",
            Self::TooManyTokens => "too_many_tokens",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::CurrentPassword(problem) => problem.message(),
            Self::Name(problem) => problem.message(),
            Self::UnknownAccess => "Choose what the token can access.",
            Self::TooManyTokens => "You have too many API tokens. Revoke one you no longer use.",
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::CurrentPassword(problem) => problem.field(),
            Self::Name(_) => Some(NAME_FIELD),
            Self::UnknownAccess | Self::TooManyTokens => None,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::CurrentPassword(problem) => problem.status(),
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeTokenProblem {
    NotFound,
}

impl ActionProblem for RevokeTokenProblem {
    fn code(self) -> &'static str {
        "token_not_found"
    }

    fn message(self) -> &'static str {
        "That API token doesn't exist."
    }

    fn status(self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Create an API token. The token is shown once.")]
pub struct CreateToken {
    #[schemars(description = "A name to recognize the token by.", length(min = 1, max = MAX_NAME_LENGTH))]
    pub name: String,
    #[schemars(description = "What the token can access.")]
    pub access: ApiScope,
    #[schemars(description = "How many days until the token expires.")]
    pub lifetime: TokenLifetime,
    #[schemars(description = "The account's current password.")]
    pub current_password: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Revoke an API token.")]
pub struct RevokeToken {
    #[schemars(description = "The token's id (tok_…).")]
    pub id: String,
}

pub async fn list_api_tokens(
    cx: &Cx,
    caller: &Caller,
) -> ActionResult<Vec<ApiTokenSummary>, Infallible> {
    authorize!(caller.require_browser());

    Ok(Ok(query_list_api_tokens(caller.owner_id(), db(cx)).await?))
}

pub async fn create_api_token(
    cx: &Cx,
    caller: &Caller,
    input: &CreateToken,
) -> ActionResult<String, CreateTokenProblem> {
    authorize!(caller.require_browser());
    let user = &caller.user;
    if let Err(problem) = check_current_password(cx, user, &input.current_password) {
        return fail(CreateTokenProblem::CurrentPassword(problem));
    }
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return fail(CreateTokenProblem::Name(problem)),
    };
    if !input.access.available_to(user) {
        return fail(CreateTokenProblem::UnknownAccess);
    }

    let token = generate_api_token();
    let public_id = PublicIdKind::ApiToken.generate();
    let scopes = vec![input.access.as_str().to_string()];
    let mut tx = db(cx).begin().await?;
    let inserted = query_insert_api_token(
        &NewApiTokenRow {
            user_id: caller.owner_id(),
            public_id: &public_id,
            name,
            token_hash: &token.hash,
            last_four: &token.last_four,
            scopes: &scopes,
            audience: TokenAudience::Api.as_str(),
            expires_at: time::OffsetDateTime::now_utc()
                + time::Duration::days(input.lifetime.days()),
            max_per_user: MAX_TOKENS_PER_USER,
        },
        &mut *tx,
    )
    .await?;
    if inserted.is_none() {
        return fail(CreateTokenProblem::TooManyTokens);
    }
    record_event(
        cx,
        &EventSource::of(caller),
        AccountEventKind::ApiTokenCreated,
        json!({ "token": public_id, "name": name, "scopes": scopes }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    if rate_limits(cx).allow_email_to(&user.email) {
        let developer_url = app_url(cx, &AccountSection::ApiTokens.href(cx));
        if let Err(e) = mailer(cx)
            .send(&api_token_created_email(&user.email, name, &developer_url))
            .await
        {
            tracing::error!("Failed to send API-token-created email: {e}");
        }
    }

    Ok(Ok(token.raw))
}

pub async fn revoke_api_token(
    cx: &Cx,
    caller: &Caller,
    input: &RevokeToken,
) -> ActionResult<String, RevokeTokenProblem> {
    authorize!(caller.require_browser());
    if !PublicIdKind::ApiToken.is_valid(&input.id) {
        return fail(RevokeTokenProblem::NotFound);
    }
    let mut tx = db(cx).begin().await?;
    let Some(name) = query_delete_api_token(&input.id, caller.owner_id(), &mut *tx).await? else {
        return fail(RevokeTokenProblem::NotFound);
    };
    record_event(
        cx,
        &EventSource::of(caller),
        AccountEventKind::ApiTokenRevoked,
        json!({ "token": input.id, "name": name }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, Denied};
    use crate::auth::password::hash_password;
    use crate::user::queries::{NewUser, query_insert_user, query_set_password_and_verify_user};
    use sqlx::PgPool;
    use std::sync::Arc;

    #[sqlx::test]
    async fn an_api_token_can_never_create_or_revoke_tokens(db: PgPool) {
        let user_id = query_insert_user(
            &NewUser {
                name: "Guarded",
                email: "guarded@example.test",
            },
            &db,
        )
        .await
        .unwrap()
        .unwrap();
        let hash = hash_password("a-strong-test-password").unwrap();
        let user = Arc::new(
            query_set_password_and_verify_user(user_id, &hash, &db)
                .await
                .unwrap(),
        );
        let token = generate_api_token();
        let public_id = PublicIdKind::ApiToken.generate();
        let scopes = vec!["profile:write".to_string()];
        let existing = query_insert_api_token(
            &NewApiTokenRow {
                user_id,
                public_id: &public_id,
                name: "Existing",
                token_hash: &token.hash,
                last_four: &token.last_four,
                scopes: &scopes,
                audience: TokenAudience::Api.as_str(),
                expires_at: time::OffsetDateTime::now_utc() + time::Duration::days(1),
                max_per_user: MAX_TOKENS_PER_USER,
            },
            &db,
        )
        .await
        .unwrap()
        .unwrap();
        let cx = Cx::new().with(db.clone());
        let caller = Caller::api_token(user, existing, ApiScope::ALL.to_vec());

        let created = create_api_token(
            &cx,
            &caller,
            &CreateToken {
                name: "Escalation".to_string(),
                access: ApiScope::ProfileWrite,
                lifetime: TokenLifetime::Year,
                current_password: "a-strong-test-password".to_string(),
            },
        )
        .await
        .unwrap();
        let revoked = revoke_api_token(&cx, &caller, &RevokeToken { id: public_id })
            .await
            .unwrap();

        assert_eq!(created, Err(ActionError::Denied(Denied::BrowserOnly)));
        assert_eq!(revoked, Err(ActionError::Denied(Denied::BrowserOnly)));
        assert_eq!(query_list_api_tokens(user_id, &db).await.unwrap().len(), 1);
        let events = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM account_events WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(events, 0);
    }

    #[test]
    fn every_label_matches_the_lifetime() {
        for lifetime in TokenLifetime::ALL {
            let days = lifetime.days();
            let expected = if days == 365 {
                "1 year".to_string()
            } else {
                format!("{days} days")
            };
            assert_eq!(lifetime.label(), expected);
        }
    }

    #[test]
    fn lifetimes_parse_from_their_form_values() {
        for lifetime in TokenLifetime::ALL {
            let parsed: TokenLifetime =
                serde_json::from_value(serde_json::json!(lifetime.value())).unwrap();
            assert_eq!(parsed, lifetime);
            assert_eq!(lifetime.value(), lifetime.days().to_string());
        }
    }
}
