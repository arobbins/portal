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

use crate::account::tabs::AccountSection;
use crate::action::{FieldError, browser_caller, require_browser_caller};
use crate::flash::{Flash, set_flash};
use crate::preferences::actions::{UpdateNotifications, update_notifications};
use crate::web::{env, session};

#[procedure]
pub(super) async fn save_notifications(
    cx: &Cx,
    sign_in_alerts: bool,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let input = UpdateNotifications { sign_in_alerts };

    match update_notifications(&env(cx), &caller, &input).await? {
        Ok(_) => Ok(Ok(String::new())),
        Err(error) => match error.for_browser()? {},
    }
}

#[route(POST "/account/notifications/update")]
pub async fn update_notifications_submit(
    cx: &Cx,
    Form(input): Form<UpdateNotifications>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match update_notifications(&env(cx), &caller, &input).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::NotificationsSaved).await?;
            Ok(see_other(AccountSection::Notifications.path()))
        }
        Err(error) => match error.for_browser()? {},
    }
}
