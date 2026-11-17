use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    runtime::{Signal, signal},
    view::{View, component, view},
};
use tower_sessions::Session;

use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::tone::Tone;
use crate::web::session;

pub const SESSION_FLASH: &str = "flash";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flash {
    RegistrationSent,
    ProfileSaved,
    EmailConfirmed,
    EmailChangeSent,
    EmailChanged,
    ResetLinkSent,
    PasswordReset,
    PasswordChanged,
    PasskeyAdded,
    PasskeyRenamed,
    PasskeyRemoved,
    DeviceSignedOut,
    OtherDevicesSignedOut,
    ApiTokenRevoked,
    AccountDeleted,
    UserDisabled,
    UserEnabled,
    UserSignedOut,
    VerificationResent,
    UserDeleted,
}

impl Flash {
    pub fn name(self) -> &'static str {
        match self {
            Self::RegistrationSent => "registration-sent",
            Self::ProfileSaved => "profile-saved",
            Self::EmailConfirmed => "email-confirmed",
            Self::EmailChangeSent => "email-change-sent",
            Self::EmailChanged => "email-changed",
            Self::ResetLinkSent => "reset-link-sent",
            Self::PasswordReset => "password-reset",
            Self::PasswordChanged => "password-changed",
            Self::PasskeyAdded => "passkey-added",
            Self::PasskeyRenamed => "passkey-renamed",
            Self::PasskeyRemoved => "passkey-removed",
            Self::DeviceSignedOut => "device-signed-out",
            Self::OtherDevicesSignedOut => "other-devices-signed-out",
            Self::ApiTokenRevoked => "api-token-revoked",
            Self::AccountDeleted => "account-deleted",
            Self::UserDisabled => "user-disabled",
            Self::UserEnabled => "user-enabled",
            Self::UserSignedOut => "user-signed-out",
            Self::VerificationResent => "verification-resent",
            Self::UserDeleted => "user-deleted",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::RegistrationSent => "Check your email to confirm your registration.",
            Self::ProfileSaved => "Profile saved.",
            Self::EmailConfirmed => "Your email is confirmed. Welcome!",
            Self::EmailChangeSent => {
                "Check your new inbox for a link to confirm the change. Your email stays the same until you do."
            }
            Self::EmailChanged => "Your email has been changed.",
            Self::ResetLinkSent => {
                "If an account exists for that email, we've sent a link to reset your password."
            }
            Self::PasswordReset => "Your password has been reset.",
            Self::PasskeyAdded => "Passkey added. You can now use it to sign in.",
            Self::PasskeyRenamed => "Passkey renamed.",
            Self::PasskeyRemoved => "Passkey removed.",
            Self::DeviceSignedOut => "That device has been signed out.",
            Self::OtherDevicesSignedOut => "Your other devices have been signed out.",
            Self::ApiTokenRevoked => "API token revoked. Apps using it can no longer sign in.",
            Self::AccountDeleted => {
                "Your account has been deleted. You've been signed out everywhere."
            }
            Self::PasswordChanged => {
                "Your password has been changed. Your other devices have been signed out."
            }
            Self::UserDisabled => "Account disabled. They've been signed out everywhere.",
            Self::UserEnabled => "Account enabled. They can sign in again.",
            Self::UserSignedOut => "They've been signed out of every device.",
            Self::VerificationResent => "A new confirmation email is on its way.",
            Self::UserDeleted => "The account has been deleted.",
        }
    }
}

pub fn flash_toast(cx: &Cx, flash: Option<Flash>) -> Signal<String> {
    let name = flash.map_or(NO_TOAST, Flash::name);
    signal(cx, move || name.to_string())
}

#[component]
pub async fn flash_toasts(
    cx: &Cx,
    toast: &Signal<String>,
    flashes: &'static [Flash],
) -> Result<impl View> {
    Ok(view! {
        #[key(flash.name())]
        for flash in flashes {
            toaster(
                toast: toast,
                name: flash.name(),
                tone: Tone::Success,
                (flash.message())
            )
        }
    })
}

pub async fn set_flash(session: &Session, flash: Flash) -> Result<()> {
    session.insert(SESSION_FLASH, flash).await?;
    Ok(())
}

pub async fn take_flash(cx: &Cx, accepted: &[Flash]) -> Result<Option<Flash>> {
    let session = session(cx)?;
    let Some(flash) = session.get::<Flash>(SESSION_FLASH).await? else {
        return Ok(None);
    };

    if !accepted.contains(&flash) {
        return Ok(None);
    }

    session.remove::<Flash>(SESSION_FLASH).await?;
    Ok(Some(flash))
}
