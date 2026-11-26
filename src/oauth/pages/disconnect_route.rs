use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, not_found, see_other},
        route,
    },
};

use crate::account::tabs::AccountSection;
use crate::action::require_browser_caller;
use crate::flash::{Flash, set_flash};
use crate::oauth::actions::{DisconnectApp, disconnect_app};
use crate::web::{env, session};

#[route(POST "/account/connected-apps/disconnect")]
pub async fn disconnect_app_submit(cx: &Cx, Form(input): Form<DisconnectApp>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    if let Err(error) = disconnect_app(&env(cx), &caller, &input).await? {
        error.for_browser()?;
        return Err(not_found().into());
    }
    set_flash(&session(cx)?, Flash::AppDisconnected).await?;

    Ok(see_other(AccountSection::ConnectedApps.path()))
}
