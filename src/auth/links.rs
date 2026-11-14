use sqlx::{PgConnection, PgPool};
use time::{Duration, OffsetDateTime};

use crate::auth::queries::{query_delete_user_tokens, query_insert_user_token};
use crate::config::Config;
use crate::jobs::{Job, enqueue};
use crate::mail::Email;
use crate::secret::{hash_secret, random_base62};

const LINK_TOKEN_LENGTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    EmailVerification,
    PasswordReset,
}

impl TokenPurpose {
    #[cfg(test)]
    pub const ALL: [Self; 2] = [Self::EmailVerification, Self::PasswordReset];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
        }
    }

    pub fn lifetime(self) -> Duration {
        match self {
            Self::EmailVerification => Duration::hours(24),
            Self::PasswordReset => Duration::hours(1),
        }
    }

    pub fn lifetime_text(self) -> &'static str {
        match self {
            Self::EmailVerification => "24 hours",
            Self::PasswordReset => "1 hour",
        }
    }

    pub fn expires_at(self) -> OffsetDateTime {
        OffsetDateTime::now_utc() + self.lifetime()
    }

    pub fn path(self, token: &str) -> String {
        match self {
            Self::EmailVerification => format!("/verify-email/{token}"),
            Self::PasswordReset => format!("/reset-password/{token}"),
        }
    }

    pub fn url(self, config: &Config, token: &str) -> String {
        config.url(&self.path(token))
    }
}

pub async fn replace_user_token(
    user_id: i32,
    purpose: TokenPurpose,
    db: &mut PgConnection,
) -> Result<String, sqlx::Error> {
    query_delete_user_tokens(user_id, purpose, &mut *db).await?;
    let raw = random_base62(LINK_TOKEN_LENGTH);
    query_insert_user_token(user_id, purpose, &hash_secret(&raw), &mut *db).await?;

    Ok(raw)
}

pub async fn email_user_link(
    db: &PgPool,
    config: &Config,
    user_id: i32,
    purpose: TokenPurpose,
    email: impl FnOnce(&str) -> Email,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    let token = replace_user_token(user_id, purpose, &mut tx).await?;
    enqueue(
        &Job::SendEmail(email(&purpose.url(config, &token))),
        &mut *tx,
    )
    .await?;
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_paths_carry_the_token() {
        assert_eq!(
            TokenPurpose::EmailVerification.path("abc"),
            "/verify-email/abc"
        );
        assert_eq!(
            TokenPurpose::PasswordReset.path("abc"),
            "/reset-password/abc"
        );
    }

    #[test]
    fn the_lifetime_text_matches_the_lifetime() {
        for purpose in TokenPurpose::ALL {
            let hours = purpose.lifetime().whole_hours();
            let unit = if hours == 1 { "hour" } else { "hours" };
            assert_eq!(purpose.lifetime_text(), format!("{hours} {unit}"));
        }
    }
}
