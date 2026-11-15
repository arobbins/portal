use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        route,
    },
    runtime::procedure,
};

use crate::account::tabs::AccountTab;
use crate::action::{FieldError, browser_caller, require_browser_caller};
use crate::auth::actions::{ChangePassword, ChangePasswordProblem, change_password};
use crate::auth::session::sign_in;
use crate::flash::{Flash, set_flash};
use crate::web::{env, redisplay, session};

pub(super) struct ChangePasswordFailed {
    pub problem: ChangePasswordProblem,
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
            sign_in(&session(cx)?, &user).await?;
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
            sign_in(&session, &user).await?;
            set_flash(&session, Flash::PasswordChanged).await?;
            Ok(see_other(AccountTab::Security.path()))
        }
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(
                AccountTab::Security.path(),
                ChangePasswordFailed { problem },
            ))
        }
    }
}
