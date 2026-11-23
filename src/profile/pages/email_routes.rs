use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        route,
    },
    runtime::procedure,
};

use crate::account::tabs::AccountPage;
use crate::action::{ActionResult, Caller, FieldError, browser_caller, require_browser_caller};
use crate::auth::actions::{ChangeEmail, ChangeEmailProblem, request_email_change};
use crate::auth::reauth::{return_passkey_proof, take_passkey_proof};
use crate::flash::{Flash, set_flash};
use crate::web::{env, redisplay, session};

struct EmailChangeFailed {
    problem: ChangeEmailProblem,
}

#[procedure]
pub(super) async fn send_email_change_link(
    cx: &Cx,
    new_email: String,
    current_password: String,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let input = ChangeEmail {
        new_email,
        current_password,
    };

    match change_email(cx, &caller, &input).await? {
        Ok(()) => Ok(Ok(String::new())),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/email/change")]
pub async fn change_email_submit(cx: &Cx, Form(input): Form<ChangeEmail>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match change_email(cx, &caller, &input).await? {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::EmailChangeSent).await?;
            Ok(see_other(AccountPage::Profile.path()))
        }
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(
                AccountPage::Profile.path(),
                EmailChangeFailed { problem },
            ))
        }
    }
}

async fn change_email(
    cx: &Cx,
    caller: &Caller,
    input: &ChangeEmail,
) -> ActionResult<(), ChangeEmailProblem> {
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;
    let outcome = request_email_change(&env, caller, input, passkey).await?;
    if outcome.is_err() {
        return_passkey_proof(&session, passkey).await?;
    }
    Ok(outcome)
}

pub(super) fn email_change_problem(cx: &Cx) -> Option<ChangeEmailProblem> {
    try_context::<EmailChangeFailed>(cx).map(|failed| failed.problem)
}
