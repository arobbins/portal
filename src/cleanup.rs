use std::time::Duration;

use sqlx::PgPool;

use crate::user::queries::{query_delete_abandoned_signups, query_delete_expired_user_tokens};

const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
const ABANDONED_SIGNUP_AGE: time::Duration = time::Duration::days(7);

pub fn spawn(db: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(e) = run(&db).await {
                tracing::error!("cleanup failed: {}", e);
            }
        }
    });
}

pub async fn run(db: &PgPool) -> Result<(), sqlx::Error> {
    let tokens = query_delete_expired_user_tokens(db).await?;
    let signups = query_delete_abandoned_signups(ABANDONED_SIGNUP_AGE, db).await?;

    if tokens > 0 || signups > 0 {
        tracing::info!("cleanup removed {tokens} expired tokens and {signups} abandoned sign-ups");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::auth::hash_password;
    use crate::user::queries::{
        NewUser, query_get_user_by_id, query_insert_user, query_insert_user_token,
        query_set_password_and_verify_user,
    };
    use crate::user::tokens::TokenPurpose;

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
        query_insert_user_token(user_id, TokenPurpose::EmailVerification, hash, db)
            .await
            .unwrap();
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
        query_get_user_by_id(&user_id, db).await.unwrap().is_some()
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

        run(&db).await.unwrap();

        assert!(!exists(abandoned, &db).await);
        assert!(exists(still_has_a_link, &db).await);
        assert!(exists(recent, &db).await);
        assert!(exists(active, &db).await);
        assert_eq!(token_count(&db).await, 1);
    }
}
