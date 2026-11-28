use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::Caller;
use crate::api::scope::ApiScope;
use crate::api_tokens::testing::issue_token;
use crate::auth::actions::{ChangePassword, change_password};
use crate::auth::queries::{NewUserToken, query_insert_user_token};
use crate::env::App;
use crate::oauth::testing::{app_still_works, connect_test_app};
use crate::two_factor::actions::{
    EnableTwoFactor, SecondFactorProblem, enable_two_factor, verify_second_factor,
};
use crate::two_factor::codes::{code_at, new_secret};
use crate::user::queries::query_get_user_by_id;
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

    let chosen = chosen(NEW_PASSWORD);
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
        code: String::new(),
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

const EMAILS_SENT: &str = "SELECT COUNT(*) FROM jobs WHERE kind = 'send_email' AND payload->>'to' = (SELECT email FROM users WHERE id = $1)";
const RESET_EVENTS: &str =
    "SELECT COUNT(*) FROM account_events WHERE user_id = $1 AND kind = 'password_reset'";
const API_TOKENS: &str = "SELECT COUNT(*) FROM api_tokens WHERE user_id = $1";
const EMAIL_CHANGE_LINKS: &str =
    "SELECT COUNT(*) FROM user_tokens WHERE user_id = $1 AND purpose = 'email_change'";
const USED_RECOVERY_CODES: &str =
    "SELECT COUNT(*) FROM recovery_codes WHERE user_id = $1 AND used_at IS NOT NULL";
const ANOTHER_LINK: &str = "another-reset-token";
const WRONG_CODE_LINK: &str = "a-reset-token-for-a-wrong-code";

async fn with_two_factor(email: &str, db: &PgPool) -> (Arc<User>, String, Vec<String>) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user(email, db).await;
    let secret = new_secret();
    let input = EnableTwoFactor {
        secret: secret.clone(),
        code: code_at(&secret, env.now().unix_timestamp() - 30),
        current_password: TEST_PASSWORD.to_string(),
    };
    let codes = enable_two_factor(&env, &Caller::browser(user.clone()), &input, None)
        .await
        .unwrap()
        .unwrap();
    (user, secret, codes)
}

fn with_code(code: &str) -> NewPassword {
    NewPassword {
        code: code.to_string(),
        ..chosen(NEW_PASSWORD)
    }
}

fn from(last: u8) -> Option<IpAddr> {
    Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, last)))
}

async fn insert_reset_link(user_id: i32, raw: &str, db: &PgPool) {
    let link = NewUserToken {
        user_id,
        purpose: TokenPurpose::PasswordReset,
        token_hash: &hash_secret(raw),
        payload: None,
        expires_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    query_insert_user_token(&link, db).await.unwrap();
}

async fn reset(env: &Env<'_>, raw: &str, code: &str) -> Result<User, LinkPasswordProblem> {
    set_password_from_link(env, TokenPurpose::PasswordReset, raw, &with_code(code))
        .await
        .unwrap()
}

async fn reset_is_valid(env: &Env<'_>, raw: &str) -> bool {
    link_is_valid(env, TokenPurpose::PasswordReset, raw)
        .await
        .unwrap()
}

async fn count(sql: &str, user_id: i32, db: &PgPool) -> i64 {
    sqlx::query_scalar(sql)
        .bind(user_id)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_reset_with_two_factor_needs_the_code_from_the_app(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let (user, secret, _) = with_two_factor("reset-totp@example.test", &db).await;
    insert_reset_link(user.id, RAW_TOKEN, &db).await;

    assert!(
        link_needs_code(&env, TokenPurpose::PasswordReset, RAW_TOKEN)
            .await
            .unwrap()
    );
    let code = code_at(&secret, env.now().unix_timestamp());
    let reset_user = reset(&env, RAW_TOKEN, &code).await.unwrap();

    assert_eq!(reset_user.id, user.id);
    assert!(!reset_is_valid(&env, RAW_TOKEN).await);
    assert_eq!(count(RESET_EVENTS, user.id, &db).await, 1);
    assert_eq!(
        verify_second_factor(&env, &user, &code).await.unwrap(),
        Err(SecondFactorProblem::Wrong)
    );
}

#[sqlx::test]
async fn a_reset_without_two_factor_needs_no_code(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("reset-plain@example.test", &db).await;
    insert_reset_link(user.id, RAW_TOKEN, &db).await;

    assert!(
        !link_needs_code(&env, TokenPurpose::PasswordReset, RAW_TOKEN)
            .await
            .unwrap()
    );
    assert!(reset(&env, RAW_TOKEN, "").await.is_ok());
}

#[sqlx::test]
async fn a_recovery_code_resets_and_is_used_up(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let (user, _, codes) = with_two_factor("reset-recovery@example.test", &db).await;
    insert_reset_link(user.id, RAW_TOKEN, &db).await;

    assert!(reset(&env, RAW_TOKEN, &codes[0]).await.is_ok());

    assert_eq!(count(USED_RECOVERY_CODES, user.id, &db).await, 1);
    assert_eq!(
        verify_second_factor(&env, &user, &codes[0]).await.unwrap(),
        Err(SecondFactorProblem::Wrong)
    );
}

#[sqlx::test]
async fn a_wrong_code_uses_up_every_reset_link_and_changes_nothing_else(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let (user, _, _) = with_two_factor("reset-wrong@example.test", &db).await;
    let (_, apps) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    issue_token(user.id, &["profile:read"], time::Duration::days(1), &db).await;
    let email_change = NewUserToken {
        user_id: user.id,
        purpose: TokenPurpose::EmailChange,
        token_hash: &hash_secret(RAW_TOKEN),
        payload: Some("moved@example.test"),
        expires_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    query_insert_user_token(&email_change, &db).await.unwrap();
    insert_reset_link(user.id, WRONG_CODE_LINK, &db).await;
    insert_reset_link(user.id, ANOTHER_LINK, &db).await;
    let emails_before = count(EMAILS_SENT, user.id, &db).await;
    let tokens_before = count(API_TOKENS, user.id, &db).await;

    assert_eq!(
        reset(&env, WRONG_CODE_LINK, "000000").await.unwrap_err(),
        LinkPasswordProblem::WrongCode
    );

    assert!(!reset_is_valid(&env, WRONG_CODE_LINK).await);
    assert!(!reset_is_valid(&env, ANOTHER_LINK).await);
    let after = query_get_user_by_id(user.id, &db).await.unwrap().unwrap();
    assert_eq!(after.password, user.password);
    assert_eq!(after.session_version, user.session_version);
    assert!(app_still_works(&app, &apps).await);
    assert_eq!(count(API_TOKENS, user.id, &db).await, tokens_before);
    assert_eq!(count(EMAIL_CHANGE_LINKS, user.id, &db).await, 1);
    assert_eq!(count(RESET_EVENTS, user.id, &db).await, 0);
    assert_eq!(count(EMAILS_SENT, user.id, &db).await, emails_before);
}

#[sqlx::test]
async fn a_rejected_password_keeps_the_link_without_checking_the_code(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let (user, _, _) = with_two_factor("reset-short@example.test", &db).await;
    insert_reset_link(user.id, RAW_TOKEN, &db).await;
    let input = NewPassword {
        code: "000000".to_string(),
        ..chosen("short")
    };

    let outcome = set_password_from_link(&env, TokenPurpose::PasswordReset, RAW_TOKEN, &input)
        .await
        .unwrap();

    assert!(matches!(outcome, Err(LinkPasswordProblem::Password(_))));
    assert!(reset_is_valid(&env, RAW_TOKEN).await);
}

#[sqlx::test]
async fn wrong_reset_codes_count_toward_the_sign_in_limit(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let (user, secret, _) = with_two_factor("reset-limit@example.test", &db).await;
    for attempt in 0..3 {
        let raw = format!("reset-guess-{attempt}");
        insert_reset_link(user.id, &raw, &db).await;
        assert_eq!(
            reset(&env, &raw, "000000").await.unwrap_err(),
            LinkPasswordProblem::WrongCode
        );
    }
    for _ in 0..2 {
        assert_eq!(
            verify_second_factor(&env, &user, "000000").await.unwrap(),
            Err(SecondFactorProblem::Wrong)
        );
    }
    insert_reset_link(user.id, RAW_TOKEN, &db).await;

    let right = code_at(&secret, env.now().unix_timestamp());
    assert_eq!(
        reset(&env, RAW_TOKEN, &right).await.unwrap_err(),
        LinkPasswordProblem::TooManyCodeAttempts
    );
    assert!(reset_is_valid(&env, RAW_TOKEN).await);
    assert_eq!(count(RESET_EVENTS, user.id, &db).await, 0);
    assert_eq!(
        verify_second_factor(&env, &user, &right).await.unwrap(),
        Err(SecondFactorProblem::TooManyAttempts)
    );
}

#[sqlx::test]
async fn concurrent_wrong_reset_codes_are_checked_at_most_five_times(db: PgPool) {
    let app = App::for_tests(db.clone());
    let (user, secret, _) = with_two_factor("reset-race@example.test", &db).await;
    let mut guesses = tokio::task::JoinSet::new();
    for attempt in 0..10u8 {
        let raw = format!("reset-race-{attempt}");
        insert_reset_link(user.id, &raw, &db).await;
        let app = app.clone();
        guesses.spawn(async move {
            reset(&app.env(from(attempt)), &raw, "000000")
                .await
                .unwrap_err()
        });
    }
    let mut answers = guesses.join_all().await;

    let env = app.env(from(20));
    for attempt in 0..6 {
        let raw = format!("reset-after-race-{attempt}");
        insert_reset_link(user.id, &raw, &db).await;
        answers.push(reset(&env, &raw, "000000").await.unwrap_err());
    }
    insert_reset_link(user.id, RAW_TOKEN, &db).await;

    let checked = answers
        .iter()
        .filter(|&&answer| answer == LinkPasswordProblem::WrongCode)
        .count();
    assert!((1..=5).contains(&checked), "{answers:?}");
    assert_eq!(
        answers.last(),
        Some(&LinkPasswordProblem::TooManyCodeAttempts)
    );
    let right = code_at(&secret, env.now().unix_timestamp());
    assert_eq!(
        reset(&env, RAW_TOKEN, &right).await.unwrap_err(),
        LinkPasswordProblem::TooManyCodeAttempts
    );
    assert!(reset_is_valid(&env, RAW_TOKEN).await);
}
