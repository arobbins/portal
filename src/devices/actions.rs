use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use time::OffsetDateTime;
use topcoat::router::StatusCode;

use super::queries::query_move_device_to_version;
use super::queries::{Device, query_delete_device, query_delete_other_devices, query_list_devices};
use super::user_agent::describe_user_agent;
use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::env::Env;
use crate::oauth::lifecycle::disconnect_all_for_user;
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;
use crate::user::queries::query_bump_session_version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub ip: Option<String>,
    pub signed_in_at: OffsetDateTime,
    pub last_active_at: OffsetDateTime,
    pub current: bool,
}

impl DeviceView {
    fn of(device: Device, current: Option<&str>) -> Self {
        Self {
            current: current == Some(device.public_id.as_str()),
            name: describe_user_agent(&device.user_agent),
            id: device.public_id,
            ip: device.ip,
            signed_in_at: device.created_at,
            last_active_at: device.last_seen_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Sign out one of your other devices.")]
pub struct SignOutDevice {
    #[schemars(description = "The device's id (dev_…).")]
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignOutDeviceProblem {
    NotFound,
    ThisDevice,
}

impl ActionProblem for SignOutDeviceProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::NotFound => {
                ProblemSpec::new("device_not_found", "That device isn't signed in anymore.")
                    .with_status(StatusCode::NOT_FOUND)
            }
            Self::ThisDevice => ProblemSpec::new(
                "device_is_current",
                "That's the device you're using. Use Sign out in the menu instead.",
            ),
        }
    }
}

pub async fn list_devices(
    env: &Env<'_>,
    caller: &Caller,
    current: Option<&str>,
) -> ActionResult<Vec<DeviceView>, Infallible> {
    authorize!(caller.require(Permission::ManageDevices));
    let devices = query_list_devices(caller.owner_id(), env.db()).await?;
    let mut views: Vec<DeviceView> = devices
        .into_iter()
        .map(|device| DeviceView::of(device, current))
        .collect();
    views.sort_by_key(|view| !view.current);

    Ok(Ok(views))
}

pub async fn sign_out_device(
    env: &Env<'_>,
    caller: &Caller,
    input: &SignOutDevice,
    current: Option<&str>,
) -> ActionResult<(), SignOutDeviceProblem> {
    authorize!(caller.require(Permission::ManageDevices));
    if current == Some(input.id.as_str()) {
        return fail(SignOutDeviceProblem::ThisDevice);
    }
    if !PublicIdKind::Device.is_valid(&input.id) {
        return fail(SignOutDeviceProblem::NotFound);
    }

    let mut tx = env.db().begin().await?;
    let Some(user_agent) = query_delete_device(caller.owner_id(), &input.id, &mut *tx).await?
    else {
        return fail(SignOutDeviceProblem::NotFound);
    };
    let detail = json!({ "device": input.id, "name": describe_user_agent(&user_agent) });
    let kind = AccountEventKind::DeviceSignedOut;
    record_event(env, &EventSource::of(caller), kind, detail, &mut *tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}

pub async fn sign_out_other_devices(
    env: &Env<'_>,
    caller: &Caller,
    current: &str,
) -> ActionResult<User, Infallible> {
    authorize!(caller.require(Permission::ManageDevices));

    let mut tx = env.db().begin().await?;
    let user = query_bump_session_version(caller.owner_id(), &mut *tx).await?;
    if !query_move_device_to_version(current, &user, &mut *tx).await? {
        return Err(topcoat::Error::msg("this device is no longer signed in"));
    }
    let signed_out = query_delete_other_devices(user.id, current, &mut *tx).await?;
    let apps = disconnect_all_for_user(user.id, &mut *tx).await?;
    let detail = json!({ "devices": signed_out, "apps_disconnected": apps });
    let kind = AccountEventKind::OtherDevicesSignedOut;
    record_event(env, &EventSource::of(caller), kind, detail, &mut *tx).await?;
    tx.commit().await?;

    Ok(Ok(user))
}

#[cfg(test)]
mod tests;
