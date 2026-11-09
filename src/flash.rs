use serde::{Deserialize, Serialize};
use topcoat::{Result, context::Cx};
use tower_sessions::Session;

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
