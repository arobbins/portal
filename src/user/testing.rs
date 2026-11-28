use std::sync::Arc;

use sqlx::PgPool;

use super::model::User;
use super::queries::{NewUser, query_insert_user, query_set_password_and_verify_user};
use crate::auth::password::hash_password;

pub const TEST_PASSWORD: &str = "a-strong-test-password";

pub async fn active_user(email: &str, db: &PgPool) -> Arc<User> {
    let new_user = NewUser {
        name: "Test User",
        email,
    };
    let user_id = query_insert_user(&new_user, db).await.unwrap().unwrap();
    activate(user_id, db).await
}

pub async fn activate(user_id: i32, db: &PgPool) -> Arc<User> {
    let password_hash = hash_password(TEST_PASSWORD).unwrap();
    let user = query_set_password_and_verify_user(user_id, &password_hash, db)
        .await
        .unwrap()
        .unwrap();
    Arc::new(user)
}
