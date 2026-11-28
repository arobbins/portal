use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, not_found, see_other},
        route,
    },
    runtime::procedure,
};

use crate::account::tabs::{AccountSection, AccountTab};
use crate::action::{ActionResult, Caller, FieldError, browser_caller, require_browser_caller};
use crate::api::scope::{ApiScope, TokenAudience};
use crate::api_tokens::actions::{
    CreateToken, CreateTokenProblem, RevokeToken, create_api_token, revoke_api_token,
};
use crate::api_tokens::lifetime::TokenLifetime;
use crate::api_tokens::token::RevealedToken;
use crate::auth::reauth::{return_passkey_proof, take_passkey_proof};
use crate::flash::{Flash, set_flash};
use crate::web::{env, redisplay, session};

pub(super) struct CreateTokenFailed {
    pub problem: CreateTokenProblem,
}

pub(super) struct TokenCreated {
    pub token: RevealedToken,
}

#[route(POST "/account/api-tokens/create")]
pub async fn create_api_token_submit(cx: &Cx, Form(input): Form<CreateToken>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let page = AccountTab::Developer.path();

    match create_token(cx, &caller, &input).await? {
        Ok(token) => Err(redisplay(page, TokenCreated { token })),
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(page, CreateTokenFailed { problem }))
        }
    }
}

#[route(POST "/account/api-tokens/revoke")]
pub async fn revoke_api_token_submit(cx: &Cx, Form(input): Form<RevokeToken>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    if let Err(error) = revoke_api_token(&env(cx), &caller, &input).await? {
        error.for_browser()?;
        return Err(not_found().into());
    }
    set_flash(&session(cx)?, Flash::ApiTokenRevoked).await?;

    Ok(see_other(AccountSection::ApiTokens.path()))
}

#[procedure]
pub(super) async fn create_api_token_js(
    cx: &Cx,
    name: String,
    access: String,
    lifetime: String,
    audience: String,
    current_password: String,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let (Ok(access), Ok(lifetime), Some(audience)) = (
        serde_json::from_value::<ApiScope>(access.into()),
        serde_json::from_value::<TokenLifetime>(lifetime.into()),
        TokenAudience::from_str(&audience),
    ) else {
        return Ok(Err(FieldError::of(CreateTokenProblem::UnknownAccess)));
    };
    let input = CreateToken {
        name,
        access,
        lifetime,
        audience,
        current_password,
    };

    match create_token(cx, &caller, &input).await? {
        Ok(token) => Ok(Ok(token.expose().to_string())),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

async fn create_token(
    cx: &Cx,
    caller: &Caller,
    input: &CreateToken,
) -> ActionResult<RevealedToken, CreateTokenProblem> {
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;
    let outcome = create_api_token(&env, caller, input, passkey).await?;
    if outcome.is_err() {
        return_passkey_proof(&session, passkey).await?;
    }
    Ok(outcome)
}
