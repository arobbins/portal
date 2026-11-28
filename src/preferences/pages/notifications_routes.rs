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
use crate::action::{FieldError, ForPage, browser_caller, require_browser_caller};
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

    update_notifications(&env(cx), &caller, &input)
        .await?
        .for_page()?;
    Ok(Ok(String::new()))
}

#[route(POST "/account/notifications/update")]
pub async fn update_notifications_submit(
    cx: &Cx,
    Form(input): Form<UpdateNotifications>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    update_notifications(&env(cx), &caller, &input)
        .await?
        .for_page()?;
    set_flash(&session(cx)?, Flash::NotificationsSaved).await?;
    Ok(see_other(AccountSection::Notifications.path()))
}
