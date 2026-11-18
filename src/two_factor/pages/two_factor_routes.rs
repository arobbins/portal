use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        route,
    },
};

use crate::account::tabs::AccountSection;
use crate::action::{ActionError, Caller, require_browser_caller};
use crate::auth::reauth::{PasskeyProof, return_passkey_proof, take_passkey_proof};
use crate::flash::{Flash, set_flash};
use crate::two_factor::actions::{
    ConfirmTwoFactorChange, EnableTwoFactor, TwoFactorProblem, disable_two_factor,
    enable_two_factor, regenerate_recovery_codes,
};
use crate::web::{env, redisplay, session};

pub struct RecoveryCodesShown {
    pub codes: Vec<String>,
}

pub struct TwoFactorFailed {
    pub problem: TwoFactorProblem,
    pub secret: Option<String>,
}

pub fn two_factor_failure(cx: &Cx) -> Option<&TwoFactorFailed> {
    try_context::<TwoFactorFailed>(cx)
}

pub fn shown_recovery_codes(cx: &Cx) -> Option<&RecoveryCodesShown> {
    try_context::<RecoveryCodesShown>(cx)
}

fn page() -> String {
    AccountSection::TwoFactor.page().path()
}

async fn proof(cx: &Cx, caller: &Caller) -> Result<Option<PasskeyProof>> {
    let env = env(cx);
    take_passkey_proof(&session(cx)?, &caller.user, env.now()).await
}

async fn failed(
    cx: &Cx,
    passkey: Option<PasskeyProof>,
    error: ActionError<TwoFactorProblem>,
    secret: Option<String>,
) -> Result<SeeOther> {
    return_passkey_proof(&session(cx)?, passkey).await?;
    let problem = error.for_browser()?;
    Err(redisplay(page(), TwoFactorFailed { problem, secret }))
}

#[route(POST "/account/two-factor/enable")]
pub async fn enable_two_factor_submit(
    cx: &Cx,
    Form(input): Form<EnableTwoFactor>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let passkey = proof(cx, &caller).await?;

    match enable_two_factor(&env(cx), &caller, &input, passkey).await? {
        Ok(codes) => Err(redisplay(page(), RecoveryCodesShown { codes })),
        Err(error) => failed(cx, passkey, error, Some(input.secret)).await,
    }
}

#[route(POST "/account/two-factor/recovery-codes")]
pub async fn regenerate_recovery_codes_submit(
    cx: &Cx,
    Form(input): Form<ConfirmTwoFactorChange>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let passkey = proof(cx, &caller).await?;

    match regenerate_recovery_codes(&env(cx), &caller, &input, passkey).await? {
        Ok(codes) => Err(redisplay(page(), RecoveryCodesShown { codes })),
        Err(error) => failed(cx, passkey, error, None).await,
    }
}

#[route(POST "/account/two-factor/disable")]
pub async fn disable_two_factor_submit(
    cx: &Cx,
    Form(input): Form<ConfirmTwoFactorChange>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let passkey = proof(cx, &caller).await?;

    match disable_two_factor(&env(cx), &caller, &input, passkey).await? {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::TwoFactorDisabled).await?;
            Ok(see_other(page()))
        }
        Err(error) => failed(cx, passkey, error, None).await,
    }
}
