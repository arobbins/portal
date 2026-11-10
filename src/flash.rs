use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    runtime::Signal,
    view::{View, component, view},
};
use tower_sessions::Session;

use crate::components::notice::NoticeKind;
use crate::components::toaster::{NO_TOAST, toaster};
use crate::web::session;

const SESSION_FLASH: &str = "flash";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flash {
    RegistrationSent,
    ProfileSaved,
    EmailConfirmed,
    ResetLinkSent,
    PasswordReset,
}

impl Flash {
    pub fn name(self) -> &'static str {
        match self {
            Self::RegistrationSent => "registration-sent",
            Self::ProfileSaved => "profile-saved",
            Self::EmailConfirmed => "email-confirmed",
            Self::ResetLinkSent => "reset-link-sent",
            Self::PasswordReset => "password-reset",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::RegistrationSent => "Check your email to confirm your registration.",
            Self::ProfileSaved => "Profile saved.",
            Self::EmailConfirmed => "Your email is confirmed. Welcome!",
            Self::ResetLinkSent => {
                "If an account exists for that email, we've sent a link to reset your password."
            }
            Self::PasswordReset => "Your password has been reset.",
        }
    }
}

pub fn toast_name(flash: Option<Flash>) -> &'static str {
    flash.map_or(NO_TOAST, Flash::name)
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
                kind: NoticeKind::Success,
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
