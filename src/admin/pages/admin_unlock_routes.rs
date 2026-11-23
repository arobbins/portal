use topcoat::{Result, context::Cx, runtime::procedure};

use super::admin_access::{admin_caller, admin_destination, unlock_admin};
use crate::action::FieldError;
use crate::admin::actions::{ConfirmAdmin, confirm_admin};
use crate::auth::reauth::{return_passkey_proof, take_passkey_proof};
use crate::web::{env, session};

#[procedure]
pub(super) async fn unlock_admin_js(
    cx: &Cx,
    current_password: String,
    next: String,
) -> Result<std::result::Result<String, FieldError>> {
    let Ok(caller) = admin_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;
    let input = ConfirmAdmin { current_password };

    match confirm_admin(&env, &caller, &input, passkey).await? {
        Ok(()) => {
            unlock_admin(&session, &caller.user, env.now()).await?;
            Ok(Ok(admin_destination(cx, Some(next))))
        }
        Err(error) => {
            return_passkey_proof(&session, passkey).await?;
            Ok(Err(FieldError::of(error.for_browser()?)))
        }
    }
}
