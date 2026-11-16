use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, route,
    },
    runtime::procedure,
};

use crate::account::tabs::AccountTab;
use crate::action::{ActionResult, Caller, FieldError, browser_caller, require_browser_caller};
use crate::auth::actions::{DeleteAccount, DeleteAccountProblem, delete_account};
use crate::auth::pages::login::login_page;
use crate::auth::reauth::{return_passkey_proof, take_passkey_proof};
use crate::flash::{Flash, set_flash};
use crate::web::{env, redisplay, session};

struct DeleteAccountFailed {
    problem: DeleteAccountProblem,
}

#[procedure]
pub(super) async fn delete_account_js(
    cx: &Cx,
    confirmation: String,
    current_password: String,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let input = DeleteAccount {
        confirmation,
        current_password,
    };

    match remove_account(cx, &caller, &input).await? {
        Ok(()) => Ok(Ok(String::new())),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/user/delete")]
pub async fn delete_account_submit(cx: &Cx, Form(input): Form<DeleteAccount>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match remove_account(cx, &caller, &input).await? {
        Ok(()) => Ok(see_other(href!(login_page).resolve(cx))),
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(
                AccountTab::Profile.path(),
                DeleteAccountFailed { problem },
            ))
        }
    }
}

async fn remove_account(
    cx: &Cx,
    caller: &Caller,
    input: &DeleteAccount,
) -> ActionResult<(), DeleteAccountProblem> {
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;
    let outcome = delete_account(&env, caller, input, passkey).await?;
    if outcome.is_ok() {
        session.flush().await?;
        set_flash(&session, Flash::AccountDeleted).await?;
    } else {
        return_passkey_proof(&session, passkey).await?;
    }
    Ok(outcome)
}

pub(super) fn delete_account_problem(cx: &Cx) -> Option<DeleteAccountProblem> {
    try_context::<DeleteAccountFailed>(cx).map(|failed| failed.problem)
}
