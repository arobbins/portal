use std::sync::Arc;

use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::web::globals;

use argon2::{Argon2, PasswordHasher};
use password_hash::SaltString;
use sqlx::PgPool;
use topcoat::{Result, context::Cx, router::error::redirect};
use tower_sessions::Session;

const SESSION_USER_ID: &str = "user_id";
const SESSION_VERSION: &str = "session_version";

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
    let user_id: Option<i32> = session
        .get(SESSION_USER_ID)
        .await
        .map_err(|e| e.to_string())?;

    let Some(user_id) = user_id else {
        return Ok(None);
    };

    let version: Option<i32> = session
        .get(SESSION_VERSION)
        .await
        .map_err(|e| e.to_string())?;
    let user = query_get_user_by_id(&user_id, db)
        .await
        .map_err(|e| e.to_string())?;

    match user {
        Some(user) if Some(user.session_version) == version => Ok(Some(Arc::new(user))),
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
    session.insert(SESSION_USER_ID, user.id).await?;
    session
        .insert(SESSION_VERSION, user.session_version)
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
    async fn a_session_from_before_versions_existed_is_signed_out(db: PgPool) {
        let user = active_user(&db).await;
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        session.insert(SESSION_USER_ID, user.id).await.unwrap();

        assert!(load_signed_in_user(&session, &db).await.unwrap().is_none());
    }
}
