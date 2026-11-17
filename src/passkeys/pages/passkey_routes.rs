use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        content::Form,
        error::{SeeOther, not_found, see_other},
        route,
    },
};

use crate::account::tabs::{AccountSection, AccountTab};
use crate::action::require_browser_caller;
use crate::auth::reauth::{forget_passkey_proof, return_passkey_proof, take_passkey_proof};
use crate::flash::{Flash, set_flash};
use crate::passkeys::actions::{
    RemovePasskey, RemovePasskeyProblem, RenamePasskey, RenamePasskeyProblem, remove_passkey,
    rename_passkey,
};
use crate::web::{env, redisplay, session};

struct PasskeyRenameFailed {
    problem: RenamePasskeyProblem,
}

struct PasskeyRemoveFailed {
    problem: RemovePasskeyProblem,
}

#[route(POST "/account/passkeys/rename")]
pub async fn rename_passkey_submit(cx: &Cx, Form(input): Form<RenamePasskey>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match rename_passkey(&env(cx), &caller, &input).await? {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::PasskeyRenamed).await?;
            Ok(see_other(AccountSection::Passkeys.path()))
        }
        Err(error) => match error.for_browser()? {
            RenamePasskeyProblem::NotFound => Err(not_found().into()),
            problem => Err(redisplay(
                AccountTab::Security.path(),
                PasskeyRenameFailed { problem },
            )),
        },
    }
}

#[route(POST "/account/passkeys/remove")]
pub async fn remove_passkey_submit(cx: &Cx, Form(input): Form<RemovePasskey>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;

    if let Err(error) = remove_passkey(&env, &caller, &input, passkey).await? {
        return_passkey_proof(&session, passkey).await?;
        return match error.for_browser()? {
            RemovePasskeyProblem::NotFound => Err(not_found().into()),
            problem => Err(redisplay(
                AccountTab::Security.path(),
                PasskeyRemoveFailed { problem },
            )),
        };
    }
    forget_passkey_proof(&session).await?;
    set_flash(&session, Flash::PasskeyRemoved).await?;

    Ok(see_other(AccountSection::Passkeys.path()))
}

pub fn rename_problem(cx: &Cx) -> Option<RenamePasskeyProblem> {
    try_context::<PasskeyRenameFailed>(cx).map(|failed| failed.problem)
}

pub fn remove_problem(cx: &Cx) -> Option<RemovePasskeyProblem> {
    try_context::<PasskeyRemoveFailed>(cx).map(|failed| failed.problem)
}
