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

use crate::account::tabs::AccountPage;
use crate::action::{FieldError, browser_caller, require_browser_caller};
use crate::flash::{Flash, set_flash};
use crate::profile::actions::{UpdateProfile, update_profile};
use crate::user::name::NameProblem;
use crate::web::{env, redisplay, session};

pub(super) struct ProfileUpdateFailed {
    pub name: String,
    pub problem: NameProblem,
}

#[procedure]
pub(super) async fn save_profile_name(cx: &Cx, name: String) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };

    match update_profile(&env(cx), &caller, &UpdateProfile { name }).await? {
        Ok(profile) => Ok(Ok(profile.name)),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/profile/update")]
pub async fn update_profile_submit(cx: &Cx, Form(input): Form<UpdateProfile>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match update_profile(&env(cx), &caller, &input).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::ProfileSaved).await?;
            Ok(see_other(AccountPage::Profile.path()))
        }
        Err(error) => {
            let problem = error.for_browser()?;
            let failed = ProfileUpdateFailed {
                name: input.name,
                problem,
            };
            Err(redisplay(AccountPage::Profile.path(), failed))
        }
    }
}
