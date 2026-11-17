use schemars::JsonSchema;
use serde::Deserialize;

use crate::action::{ActionResult, Caller, authorize, fail};
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
use crate::env::Env;
use crate::permission::Permission;

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Confirm it's you before opening the admin area.")]
pub struct ConfirmAdmin {
    #[serde(default)]
    #[schemars(description = "Your current password, unless you confirmed with a passkey.")]
    pub current_password: String,
}

pub async fn confirm_admin(
    env: &Env<'_>,
    caller: &Caller,
    input: &ConfirmAdmin,
    passkey: Option<PasskeyProof>,
) -> ActionResult<(), ReauthProblem> {
    authorize!(caller.require(Permission::AdministerUsers));

    match check_reauth(env, &caller.user, &input.current_password, passkey).await? {
        Ok(()) => Ok(Ok(())),
        Err(problem) => fail(problem),
    }
}
