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
use crate::action::{ForPage, require_browser_caller};
use crate::auth::session::{current_device, stay_signed_in};
use crate::devices::actions::{
    SignOutDevice, SignOutDeviceProblem, sign_out_device, sign_out_other_devices,
};
use crate::flash::{Flash, set_flash};
use crate::web::{app, env, session};

#[route(POST "/account/devices/sign-out")]
pub async fn sign_out_device_submit(cx: &Cx, Form(input): Form<SignOutDevice>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let session = session(cx)?;
    let current = current_device(&session).await?;

    match sign_out_device(&env(cx), &caller, &input, current.as_deref()).await? {
        Ok(()) => set_flash(&session, Flash::DeviceSignedOut).await?,
        Err(error) => match error.for_browser()? {
            SignOutDeviceProblem::NotFound | SignOutDeviceProblem::ThisDevice => {}
        },
    }

    Ok(see_other(AccountSection::Devices.path()))
}

#[route(POST "/account/devices/sign-out-others")]
pub async fn sign_out_other_devices_submit(cx: &Cx) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let session = session(cx)?;
    let Some(current) = current_device(&session).await? else {
        return Err(not_found().into());
    };

    let user = sign_out_other_devices(&env(cx), &caller, &current)
        .await?
        .for_page()?;
    stay_signed_in(app(cx), &session, &user).await?;
    set_flash(&session, Flash::OtherDevicesSignedOut).await?;

    Ok(see_other(AccountSection::Devices.path()))
}
