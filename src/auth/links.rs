use sqlx::PgConnection;
use time::{Duration, OffsetDateTime};

use crate::auth::emails::register_verification_email;
use crate::auth::queries::{
    NewUserToken, query_delete_expired_user_tokens, query_delete_user_tokens,
    query_insert_user_token,
};
use crate::config::Config;
use crate::env::App;
use crate::jobs::enqueue;
use crate::mail::{Email, SendEmail};
use crate::secret::{hash_secret, random_base62};
use crate::user::queries::query_delete_abandoned_signups;

const LINK_TOKEN_LENGTH: usize = 32;
const ABANDONED_SIGN_UP_AGE: Duration = Duration::days(7);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    EmailVerification,
    PasswordReset,
    EmailChange,
}

impl TokenPurpose {
    #[cfg(test)]
    pub const ALL: [Self; 3] = [
        Self::EmailVerification,
        Self::PasswordReset,
        Self::EmailChange,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
            Self::PasswordReset => "password_reset",
            Self::EmailChange => "email_change",
        }
    }

    pub fn lifetime(self) -> Duration {
        match self {
            Self::EmailVerification | Self::EmailChange => Duration::hours(24),
            Self::PasswordReset => Duration::hours(1),
        }
    }

    pub fn lifetime_text(self) -> &'static str {
        match self {
            Self::EmailVerification | Self::EmailChange => "24 hours",
            Self::PasswordReset => "1 hour",
        }
    }

    pub fn path(self, token: &str) -> String {
        match self {
            Self::EmailVerification => format!("/verify-email/{token}"),
            Self::PasswordReset => format!("/reset-password/{token}"),
            Self::EmailChange => format!("/confirm-email/{token}"),
        }
    }

    pub fn url(self, config: &Config, token: &str) -> String {
        config.url(&self.path(token))
    }
}

pub struct NewLink<'a> {
    pub user_id: i32,
    pub purpose: TokenPurpose,
    pub payload: Option<&'a str>,
    pub email: &'a (dyn Fn(&str) -> Email + Sync),
}

pub async fn email_user_link(app: &App, link: NewLink<'_>) -> Result<(), sqlx::Error> {
    let mut tx = app.db.begin().await?;
    let token = replace_user_token(app.clock.now(), &link, &mut tx).await?;
    let url = link.purpose.url(&app.config, &token);
    enqueue(&SendEmail((link.email)(&url)), &mut *tx).await?;
    tx.commit().await
}

pub async fn send_verification_link(
    app: &App,
    user_id: i32,
    email: &str,
) -> Result<(), sqlx::Error> {
    email_user_link(
        app,
        NewLink {
            user_id,
            purpose: TokenPurpose::EmailVerification,
            payload: None,
            email: &|url| register_verification_email(email, url),
        },
    )
    .await
}

pub async fn delete_stale_sign_ups(app: &App) -> Result<(u64, u64), sqlx::Error> {
    let links = query_delete_expired_user_tokens(&app.db).await?;
    let cutoff = app.clock.now() - ABANDONED_SIGN_UP_AGE;
    let sign_ups = query_delete_abandoned_signups(cutoff, &app.db).await?;
    Ok((links, sign_ups))
}

async fn replace_user_token(
    now: OffsetDateTime,
    link: &NewLink<'_>,
    db: &mut PgConnection,
) -> Result<String, sqlx::Error> {
    query_delete_user_tokens(link.user_id, link.purpose, &mut *db).await?;
    let raw = random_base62(LINK_TOKEN_LENGTH);
    let token = NewUserToken {
        user_id: link.user_id,
        purpose: link.purpose,
        token_hash: &hash_secret(&raw),
        payload: link.payload,
        expires_at: now + link.purpose.lifetime(),
    };
    query_insert_user_token(&token, &mut *db).await?;

    Ok(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::password::hash_password;
    use crate::user::queries::{
        NewUser, query_get_user_by_id, query_insert_user, query_set_password_and_verify_user,
    };
    use sqlx::PgPool;

    async fn insert_user(email: &str, days_old: i64, db: &PgPool) -> i32 {
        let new_user = NewUser {
            name: "Tester",
            email,
        };
        let user_id = query_insert_user(&new_user, db).await.unwrap().unwrap();
        sqlx::query!(
            "UPDATE users SET created_at = now() - make_interval(days => $1) WHERE id = $2",
            days_old as i32,
            user_id,
        )
        .execute(db)
        .await
        .unwrap();
        user_id
    }

    async fn insert_token(user_id: i32, hash: &str, expired: bool, db: &PgPool) {
        let token = NewUserToken {
            user_id,
            purpose: TokenPurpose::EmailVerification,
            token_hash: hash,
            payload: None,
            expires_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
        };
        query_insert_user_token(&token, db).await.unwrap();
        if expired {
            sqlx::query!(
                "UPDATE user_tokens SET expires_at = now() - interval '1 hour' WHERE token_hash = $1",
                hash,
            )
            .execute(db)
            .await
            .unwrap();
        }
    }

    async fn exists(user_id: i32, db: &PgPool) -> bool {
        query_get_user_by_id(user_id, db).await.unwrap().is_some()
    }

    async fn token_count(db: &PgPool) -> i64 {
        sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM user_tokens"#)
            .fetch_one(db)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn removes_expired_tokens_and_abandoned_sign_ups(db: PgPool) {
        let abandoned = insert_user("abandoned@example.test", 8, &db).await;
        insert_token(abandoned, "expired-hash", true, &db).await;

        let still_has_a_link = insert_user("waiting@example.test", 8, &db).await;
        insert_token(still_has_a_link, "valid-hash", false, &db).await;

        let recent = insert_user("recent@example.test", 1, &db).await;

        let active = insert_user("active@example.test", 30, &db).await;
        let password_hash = hash_password("a-strong-test-password").unwrap();
        query_set_password_and_verify_user(active, &password_hash, &db)
            .await
            .unwrap();

        delete_stale_sign_ups(&App::for_tests(db.clone()))
            .await
            .unwrap();

        assert!(!exists(abandoned, &db).await);
        assert!(exists(still_has_a_link, &db).await);
        assert!(exists(recent, &db).await);
        assert!(exists(active, &db).await);
        assert_eq!(token_count(&db).await, 1);
    }

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
        assert_eq!(TokenPurpose::EmailChange.path("abc"), "/confirm-email/abc");
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
