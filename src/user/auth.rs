use std::sync::{Arc, LazyLock};

use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::web::globals;

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use password_hash::{PasswordHash, SaltString};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use topcoat::{Result, context::Cx, router::error::redirect};
use tower_sessions::Session;

const SESSION_AUTH: &str = "auth";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct SessionAuth {
    user_id: i32,
    session_version: i32,
}

static UNMATCHABLE_HASH: LazyLock<String> = LazyLock::new(|| {
    hash_password(&crate::user::tokens::generate_token().raw)
        .expect("argon2 can hash a random password")
});

pub fn password_matches(password_hash: Option<&str>, password: &str) -> bool {
    let checked_hash = password_hash.unwrap_or(&UNMATCHABLE_HASH);
    let matches = PasswordHash::new(checked_hash).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    });

    matches && password_hash.is_some()
}

pub fn hash_password(password: &str) -> Result<String, password_hash::Error> {
    let salt = SaltString::generate(rand::thread_rng());
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

pub async fn load_signed_in_user(
    session: &Session,
    db: &PgPool,
) -> std::result::Result<Option<Arc<User>>, String> {
    let auth: Option<SessionAuth> = session.get(SESSION_AUTH).await.map_err(|e| e.to_string())?;

    let Some(auth) = auth else {
        return Ok(None);
    };

    let user = query_get_user_by_id(&auth.user_id, db)
        .await
        .map_err(|e| e.to_string())?;

    match user {
        Some(user) if user.session_version == auth.session_version => Ok(Some(Arc::new(user))),
        _ => {
            session.flush().await.map_err(|e| e.to_string())?;
            Ok(None)
        }
    }
}

pub fn current_user(cx: &Cx) -> Option<Arc<User>> {
    globals(cx).current_user
}

pub fn require_user(cx: &Cx) -> Result<Arc<User>> {
    current_user(cx).ok_or_else(|| redirect("/login").into())
}

pub fn redirect_signed_in_users(cx: &Cx) -> Result<()> {
    if current_user(cx).is_some() {
        return Err(redirect("/account").into());
    }

    Ok(())
}

pub async fn sign_in(session: &Session, user: &User) -> Result<()> {
    session.cycle_id().await?;
    session
        .insert(
            SESSION_AUTH,
            SessionAuth {
                user_id: user.id,
                session_version: user.session_version,
            },
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::queries::{NewUser, query_insert_user, query_set_password_and_verify_user};
    use tower_sessions::MemoryStore;

    async fn active_user(db: &PgPool) -> User {
        let new_user = NewUser {
            name: "Tester",
            email: "tester@example.test",
        };
        let user_id = query_insert_user(&new_user, db).await.unwrap().unwrap();
        let password_hash = hash_password("a-strong-test-password").unwrap();
        query_set_password_and_verify_user(user_id, &password_hash, db)
            .await
            .unwrap()
    }

    #[test]
    fn passwords_match_only_their_own_hash() {
        let hash = hash_password("correct-password").unwrap();

        assert!(password_matches(Some(&hash), "correct-password"));
        assert!(!password_matches(Some(&hash), "wrong-password"));
        assert!(!password_matches(Some("not-a-hash"), "correct-password"));
        assert!(!password_matches(None, "correct-password"));
    }

    #[sqlx::test]
    async fn a_password_change_signs_out_existing_sessions(db: PgPool) {
        let user = active_user(&db).await;
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        sign_in(&session, &user).await.unwrap();
        assert!(load_signed_in_user(&session, &db).await.unwrap().is_some());

        let password_hash = hash_password("a-new-test-password").unwrap();
        query_set_password_and_verify_user(user.id, &password_hash, &db)
            .await
            .unwrap();

        assert!(load_signed_in_user(&session, &db).await.unwrap().is_none());
        assert!(load_signed_in_user(&session, &db).await.unwrap().is_none());
    }

    #[sqlx::test]
    async fn a_session_for_a_deleted_user_is_signed_out(db: PgPool) {
        let user = active_user(&db).await;
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        sign_in(&session, &user).await.unwrap();
        sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
            .execute(&db)
            .await
            .unwrap();

        assert!(load_signed_in_user(&session, &db).await.unwrap().is_none());
        assert!(session.is_empty().await);
    }
}
