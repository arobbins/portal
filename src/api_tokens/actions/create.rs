use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::account::tabs::AccountSection;
use crate::action::{ActionProblem, ActionResult, Caller, Outcome, ProblemSpec, authorize, fail};
use crate::api::scope::{ApiScope, TokenAudience};
use crate::api_tokens::emails::api_token_created_email;
use crate::api_tokens::lifetime::TokenLifetime;
use crate::api_tokens::queries::{NewApiTokenRow, query_insert_api_token};
use crate::api_tokens::token::{NewApiToken, generate_api_token};
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::name::{MAX_NAME_LENGTH, NameProblem, check_name};

const MAX_TOKENS_PER_USER: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateTokenProblem {
    Reauth(ReauthProblem),
    Name(NameProblem),
    UnknownAccess,
    TooManyTokens,
}

impl ActionProblem for CreateTokenProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Reauth(problem) => problem.spec(),
            Self::Name(problem) => problem.spec(),
            Self::UnknownAccess => {
                ProblemSpec::new("unknown_access", "Choose what the token can access.")
            }
            Self::TooManyTokens => ProblemSpec::new(
                "too_many_tokens",
                "You have too many API tokens. Revoke one you no longer use.",
            ),
        }
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
    #[serde(default = "default_audience")]
    #[schemars(description = "What will use the token: the JSON API (default) or an MCP client.")]
    pub audience: TokenAudience,
    #[serde(default)]
    #[schemars(
        description = "The account's current password; leave it empty right after confirming with a passkey."
    )]
    pub current_password: String,
}

fn default_audience() -> TokenAudience {
    TokenAudience::DEFAULT
}

pub async fn create_api_token(
    env: &Env<'_>,
    caller: &Caller,
    input: &CreateToken,
    passkey: Option<PasskeyProof>,
) -> ActionResult<String, CreateTokenProblem> {
    authorize!(caller.require(Permission::ManageApiTokens));
    let name = match check_request(env, caller, input, passkey).await? {
        Ok(name) => name,
        Err(problem) => return fail(problem),
    };

    let token = generate_api_token();
    let mut tx = env.db().begin().await?;
    let Some(public_id) = store_token(env, caller, input, name, &token, &mut tx).await? else {
        return fail(CreateTokenProblem::TooManyTokens);
    };
    let detail = json!({
        "token": public_id,
        "name": name,
        "scopes": [input.access.as_str()],
        "audience": input.audience.as_str(),
    });
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::ApiTokenCreated,
        detail,
        &mut *tx,
    )
    .await?;
    announce(env, caller, name, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(token.raw))
}

async fn check_request<'a>(
    env: &Env<'_>,
    caller: &Caller,
    input: &'a CreateToken,
    passkey: Option<PasskeyProof>,
) -> Outcome<&'a str, CreateTokenProblem> {
    let password = &input.current_password;
    if let Err(problem) = check_reauth(env, &caller.user, password, passkey).await? {
        return Ok(Err(CreateTokenProblem::Reauth(problem)));
    }
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return Ok(Err(CreateTokenProblem::Name(problem))),
    };
    if !input.access.available_to(&caller.user) {
        return Ok(Err(CreateTokenProblem::UnknownAccess));
    }
    Ok(Ok(name))
}

async fn store_token(
    env: &Env<'_>,
    caller: &Caller,
    input: &CreateToken,
    name: &str,
    token: &NewApiToken,
    tx: &mut sqlx::PgConnection,
) -> Result<Option<String>, sqlx::Error> {
    let public_id = PublicIdKind::ApiToken.generate();
    let scopes = vec![input.access.as_str().to_string()];
    let row = NewApiTokenRow {
        user_id: caller.owner_id(),
        public_id: &public_id,
        name,
        token_hash: &token.hash,
        last_four: &token.last_four,
        scopes: &scopes,
        audience: input.audience.as_str(),
        expires_at: env.now() + time::Duration::days(input.lifetime.days()),
        max_per_user: MAX_TOKENS_PER_USER,
    };
    let inserted = query_insert_api_token(&row, &mut *tx).await?;
    Ok(inserted.map(|_| public_id))
}

async fn announce(
    env: &Env<'_>,
    caller: &Caller,
    name: &str,
    tx: &mut sqlx::PgConnection,
) -> Result<(), sqlx::Error> {
    let user = &caller.user;
    if !env.limits().allow_email_to(&user.email).await? {
        return Ok(());
    }
    let developer_url = env.app.url(&AccountSection::ApiTokens.path());
    let email = api_token_created_email(&user.email, name, &developer_url);
    enqueue(&SendEmail(email), &mut *tx).await
}
