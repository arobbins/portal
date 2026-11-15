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
use crate::flash::{Flash, set_flash};
use crate::passkeys::actions::{
    RemovePasskey, RenamePasskey, RenamePasskeyProblem, remove_passkey, rename_passkey,
};
use crate::web::{env, redisplay, session};

struct PasskeyRenameFailed {
    problem: RenamePasskeyProblem,
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

    if let Err(error) = remove_passkey(&env(cx), &caller, &input).await? {
        error.for_browser()?;
        return Err(not_found().into());
    }
    set_flash(&session(cx)?, Flash::PasskeyRemoved).await?;

    Ok(see_other(AccountSection::Passkeys.path()))
}

pub fn rename_problem(cx: &Cx) -> Option<RenamePasskeyProblem> {
    try_context::<PasskeyRenameFailed>(cx).map(|failed| failed.problem)
}
