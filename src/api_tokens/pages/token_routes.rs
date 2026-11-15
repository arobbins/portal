use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, not_found, see_other},
        route,
    },
};

use crate::account::tabs::{AccountSection, AccountTab};
use crate::action::require_browser_caller;
use crate::api_tokens::actions::{
    CreateToken, CreateTokenProblem, RevokeToken, create_api_token, revoke_api_token,
};
use crate::flash::{Flash, set_flash};
use crate::web::{env, redisplay, session};

pub(super) struct CreateTokenFailed {
    pub problem: CreateTokenProblem,
}

pub(super) struct TokenCreated {
    pub token: String,
}

#[route(POST "/account/api-tokens/create")]
pub async fn create_api_token_submit(cx: &Cx, Form(input): Form<CreateToken>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let page = AccountTab::Developer.path();

    match create_api_token(&env(cx), &caller, &input).await? {
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
