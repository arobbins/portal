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
