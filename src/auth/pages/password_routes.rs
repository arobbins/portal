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

use crate::account::tabs::AccountSection;
use crate::action::{
    ActionProblem, FieldError, ProblemSpec, browser_caller, require_browser_caller,
};
use crate::auth::actions::{ChangePassword, ChangePasswordProblem, change_password};
use crate::auth::session::stay_signed_in;
use crate::flash::{Flash, set_flash};
use crate::web::{app, env, redisplay, session};

struct ChangePasswordFailed {
    problem: ChangePasswordProblem,
}

pub fn password_failure(cx: &Cx) -> Option<ProblemSpec> {
    try_context::<ChangePasswordFailed>(cx).map(|failed| failed.problem.spec())
}

#[procedure]
pub(super) async fn save_new_password(
    cx: &Cx,
    current_password: String,
    password: String,
    confirm_password: String,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let input = ChangePassword {
        current_password,
        password,
        confirm_password,
    };

    match change_password(&env(cx), &caller, &input).await? {
        Ok(user) => {
            stay_signed_in(app(cx), &session(cx)?, &user).await?;
            Ok(Ok(String::new()))
        }
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/password/change")]
pub async fn change_password_submit(
    cx: &Cx,
    Form(input): Form<ChangePassword>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match change_password(&env(cx), &caller, &input).await? {
        Ok(user) => {
            let session = session(cx)?;
            stay_signed_in(app(cx), &session, &user).await?;
            set_flash(&session, Flash::PasswordChanged).await?;
            Ok(see_other(AccountSection::Password.page().path()))
        }
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(
                AccountSection::Password.page().path(),
                ChangePasswordFailed { problem },
            ))
        }
    }
}
