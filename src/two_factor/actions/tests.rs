use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::auth::password::CurrentPasswordProblem;
use crate::env::App;
use crate::two_factor::codes::{code_at, new_secret};
use crate::user::model::User;
use crate::user::testing::{TEST_PASSWORD, active_user};

async fn with_two_factor(email: &str, db: &PgPool) -> (Arc<User>, String) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user(email, db).await;
    let secret = new_secret();
    let now = env.now().unix_timestamp();
    enable_two_factor(
        &env,
        &Caller::browser(user.clone()),
        &enable(&secret, &code_at(&secret, now - 30), TEST_PASSWORD),
        None,
    )
    .await
    .unwrap()
    .unwrap();
    (user, secret)
}

fn from(last: u8) -> Option<IpAddr> {
    Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, last)))
}

fn enable(secret: &str, code: &str, password: &str) -> EnableTwoFactor {
    EnableTwoFactor {
        secret: secret.to_string(),
        code: code.to_string(),
        current_password: password.to_string(),
    }
}

fn confirm() -> ConfirmTwoFactorChange {
    ConfirmTwoFactorChange {
        current_password: TEST_PASSWORD.to_string(),
    }
}

async fn subjects(email: &str, db: &PgPool) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT payload->>'subject' AS "subject!" FROM jobs WHERE kind = 'send_email' AND payload->>'to' = $1 ORDER BY id"#,
        email
    )
    .fetch_all(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn turning_on_needs_the_password_and_a_code_from_the_app(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("totp-on@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let secret = new_secret();
    let now = env.now().unix_timestamp();

    let wrong_password = enable_two_factor(
        &env,
        &caller,
        &enable(&secret, &code_at(&secret, now), "nope"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        wrong_password.unwrap_err(),
        TwoFactorProblem::Reauth(ReauthProblem::Password(CurrentPasswordProblem::Wrong)).into()
    );
    let wrong_code = enable_two_factor(
        &env,
        &caller,
        &enable(&secret, "000000", TEST_PASSWORD),
        None,
    )
    .await
    .unwrap();
    assert_eq!(wrong_code.unwrap_err(), TwoFactorProblem::WrongCode.into());

    let codes = enable_two_factor(
        &env,
        &caller,
        &enable(&secret, &code_at(&secret, now), TEST_PASSWORD),
        None,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(codes.len(), 10);
    let summary = get_two_factor(&env, &caller).await.unwrap().unwrap();
    assert!(summary.enabled_at.is_some());
    assert_eq!(summary.recovery_codes_left, 10);
    assert!(needs_second_factor(&env, &user).await.unwrap());
    assert_eq!(
        subjects(&user.email, &db).await,
        ["Two-factor authentication is on"]
    );
    let again = enable_two_factor(
        &env,
        &caller,
        &enable(&secret, &code_at(&secret, now + 30), TEST_PASSWORD),
        None,
    )
    .await
    .unwrap();
    assert_eq!(again.unwrap_err(), TwoFactorProblem::AlreadyOn.into());
}

#[sqlx::test]
async fn each_app_code_and_recovery_code_works_once(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("totp-use@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let secret = new_secret();
    let now = env.now().unix_timestamp();
    let codes = enable_two_factor(
        &env,
        &caller,
        &enable(&secret, &code_at(&secret, now - 30), TEST_PASSWORD),
        None,
    )
    .await
    .unwrap()
    .unwrap();

    let current = code_at(&secret, now);
    assert_eq!(
        verify_second_factor(&env, &user, &current).await.unwrap(),
        Ok(())
    );
    assert_eq!(
        verify_second_factor(&env, &user, &current).await.unwrap(),
        Err(SecondFactorProblem::Wrong)
    );
    let recovery = codes[0].to_uppercase();
    assert_eq!(
        verify_second_factor(&env, &user, &recovery).await.unwrap(),
        Ok(())
    );
    assert_eq!(
        verify_second_factor(&env, &user, &recovery).await.unwrap(),
        Err(SecondFactorProblem::Wrong)
    );
    let summary = get_two_factor(&env, &caller).await.unwrap().unwrap();
    assert_eq!(summary.recovery_codes_left, 9);
}

#[sqlx::test]
async fn new_recovery_codes_replace_the_old_ones_and_turning_off_removes_everything(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("totp-off@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let secret = new_secret();
    let now = env.now().unix_timestamp();
    let first = enable_two_factor(
        &env,
        &caller,
        &enable(&secret, &code_at(&secret, now), TEST_PASSWORD),
        None,
    )
    .await
    .unwrap()
    .unwrap();

    let second = regenerate_recovery_codes(&env, &caller, &confirm(), None)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(first, second);
    assert_eq!(
        verify_second_factor(&env, &user, &first[0]).await.unwrap(),
        Err(SecondFactorProblem::Wrong)
    );

    disable_two_factor(&env, &caller, &confirm(), None)
        .await
        .unwrap()
        .unwrap();
    assert!(!needs_second_factor(&env, &user).await.unwrap());
    let summary = get_two_factor(&env, &caller).await.unwrap().unwrap();
    assert_eq!(summary.recovery_codes_left, 0);
    assert_eq!(
        disable_two_factor(&env, &caller, &confirm(), None)
            .await
            .unwrap()
            .unwrap_err(),
        TwoFactorProblem::NotOn.into()
    );
    assert_eq!(
        subjects(&user.email, &db).await.last().map(String::as_str),
        Some("Two-factor authentication is off")
    );
}

#[sqlx::test]
async fn wrong_codes_lock_the_second_step_even_after_a_correct_password(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("totp-guess@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let secret = new_secret();
    let now = env.now().unix_timestamp();
    enable_two_factor(
        &env,
        &caller,
        &enable(&secret, &code_at(&secret, now - 30), TEST_PASSWORD),
        None,
    )
    .await
    .unwrap()
    .unwrap();

    for _ in 0..5 {
        assert_eq!(
            verify_second_factor(&env, &user, "000000").await.unwrap(),
            Err(SecondFactorProblem::Wrong)
        );
        env.limits()
            .clear_failed_logins(&user.email, env.ip)
            .await
            .unwrap();
    }

    assert_eq!(
        verify_second_factor(&env, &user, &code_at(&secret, now))
            .await
            .unwrap(),
        Err(SecondFactorProblem::TooManyAttempts)
    );
}

#[sqlx::test]
async fn concurrent_wrong_codes_are_checked_at_most_five_times(db: PgPool) {
    let app = App::for_tests(db.clone());
    let (user, secret) = with_two_factor("totp-race@example.test", &db).await;
    let mut guesses = tokio::task::JoinSet::new();
    for _ in 0..10 {
        let app = app.clone();
        let user = user.clone();
        guesses.spawn(async move {
            verify_second_factor(&app.env(from(1)), &user, "000000")
                .await
                .unwrap()
                .unwrap_err()
        });
    }

    let answers = guesses.join_all().await;

    let wrong = answers
        .iter()
        .filter(|&&answer| answer == SecondFactorProblem::Wrong)
        .count();
    assert_eq!(wrong, 5, "{answers:?}");
    let env = app.env(from(2));
    assert_eq!(
        verify_second_factor(&env, &user, &code_at(&secret, env.now().unix_timestamp()))
            .await
            .unwrap(),
        Err(SecondFactorProblem::TooManyAttempts)
    );
}

#[sqlx::test]
async fn codes_from_one_address_are_limited_across_accounts(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(from(3));
    let mut users = Vec::new();
    for name in ["one", "two", "three"] {
        let (user, _) = with_two_factor(&format!("totp-ip-{name}@example.test"), &db).await;
        users.push(user);
    }

    for attempt in 0..10 {
        assert_eq!(
            verify_second_factor(&env, &users[attempt % 3], "000000")
                .await
                .unwrap(),
            Err(SecondFactorProblem::Wrong)
        );
    }

    assert_eq!(
        verify_second_factor(&env, &users[1], "000000")
            .await
            .unwrap(),
        Err(SecondFactorProblem::TooManyAttempts)
    );
}
