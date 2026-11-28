use sqlx::PgPool;

use super::*;
use crate::action::Caller;
use crate::api::scope::ApiScope;
use crate::auth::actions::{ChangePassword, change_password};
use crate::auth::queries::{NewUserToken, query_insert_user_token};
use crate::env::App;
use crate::oauth::testing::{app_still_works, connect_test_app};
use crate::user::testing::{TEST_PASSWORD, active_user};

const RAW_TOKEN: &str = "a-raw-reset-token";
const NEW_PASSWORD: &str = "a-brand-new-password";

#[sqlx::test]
async fn a_password_reset_disconnects_every_app(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("reset@example.test", &db).await;
    let (_, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let reset = NewUserToken {
        user_id: user.id,
        purpose: TokenPurpose::PasswordReset,
        token_hash: &hash_secret(RAW_TOKEN),
        payload: None,
        expires_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    query_insert_user_token(&reset, &db).await.unwrap();
    assert!(app_still_works(&app, &tokens).await);

    let chosen = NewPassword {
        password: NEW_PASSWORD.to_string(),
        confirm_password: NEW_PASSWORD.to_string(),
    };
    set_password_from_link(&env, TokenPurpose::PasswordReset, RAW_TOKEN, &chosen)
        .await
        .unwrap()
        .unwrap();

    assert!(!app_still_works(&app, &tokens).await);
    let detail = sqlx::query_scalar!(
        "SELECT detail FROM account_events WHERE user_id = $1 AND kind = 'password_reset'",
        user.id
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(detail["apps_disconnected"], json!(1));
}

#[sqlx::test]
async fn a_routine_password_change_keeps_every_app(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("routine@example.test", &db).await;
    let (_, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let input = ChangePassword {
        current_password: TEST_PASSWORD.to_string(),
        password: NEW_PASSWORD.to_string(),
        confirm_password: NEW_PASSWORD.to_string(),
    };

    change_password(&env, &Caller::browser(user), &input)
        .await
        .unwrap()
        .unwrap();

    assert!(app_still_works(&app, &tokens).await);
}

async fn insert_link(user_id: i32, purpose: TokenPurpose, db: &PgPool) {
    let link = NewUserToken {
        user_id,
        purpose,
        token_hash: &hash_secret(RAW_TOKEN),
        payload: None,
        expires_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    query_insert_user_token(&link, db).await.unwrap();
}

fn chosen(password: &str) -> NewPassword {
    NewPassword {
        password: password.to_string(),
        confirm_password: password.to_string(),
    }
}

#[sqlx::test]
async fn a_verification_link_cannot_set_an_active_users_password(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("stray@example.test", &db).await;
    insert_link(user.id, TokenPurpose::EmailVerification, &db).await;

    let outcome = set_password_from_link(
        &env,
        TokenPurpose::EmailVerification,
        RAW_TOKEN,
        &chosen(NEW_PASSWORD),
    )
    .await
    .unwrap();

    assert_eq!(outcome.unwrap_err(), LinkPasswordProblem::ExpiredLink);
    let password = sqlx::query_scalar!("SELECT password FROM users WHERE id = $1", user.id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(password, user.password);
    let still_valid = link_is_valid(&env, TokenPurpose::EmailVerification, RAW_TOKEN)
        .await
        .unwrap();
    assert!(!still_valid);
}

#[sqlx::test]
async fn a_password_change_refuses_earlier_reset_links(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("changed-then-reset@example.test", &db).await;
    insert_link(user.id, TokenPurpose::PasswordReset, &db).await;
    let input = ChangePassword {
        current_password: TEST_PASSWORD.to_string(),
        password: NEW_PASSWORD.to_string(),
        confirm_password: NEW_PASSWORD.to_string(),
    };
    change_password(&env, &Caller::browser(user), &input)
        .await
        .unwrap()
        .unwrap();

    let outcome = set_password_from_link(
        &env,
        TokenPurpose::PasswordReset,
        RAW_TOKEN,
        &chosen("yet-another-password"),
    )
    .await
    .unwrap();

    assert_eq!(outcome.unwrap_err(), LinkPasswordProblem::ExpiredLink);
}
