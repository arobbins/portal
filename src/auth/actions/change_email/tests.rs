use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, Denied};
use crate::api::scope::ApiScope;
use crate::auth::actions::{ConfirmEmailProblem, confirm_email_change, pending_email_change};
use crate::auth::password::CurrentPasswordProblem;
use crate::auth::queries::{NewUserToken, query_insert_user_token};
use crate::auth::reauth::{PasskeyProof, ReauthProblem};
use crate::secret::hash_secret;
use crate::user::testing::{TEST_PASSWORD, active_user};

const NEW_EMAIL: &str = "new@example.test";
const RAW_TOKEN: &str = "a-raw-email-change-token";

fn change_to(new_email: &str, current_password: &str) -> ChangeEmail {
    ChangeEmail {
        new_email: new_email.to_string(),
        current_password: current_password.to_string(),
    }
}

async fn insert_link(user_id: i32, new_email: &str, db: &PgPool) {
    let token = NewUserToken {
        user_id,
        purpose: TokenPurpose::EmailChange,
        token_hash: &hash_secret(RAW_TOKEN),
        payload: Some(new_email),
        expires_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    query_insert_user_token(&token, db).await.unwrap();
}

async fn jobs(db: &PgPool) -> Vec<(String, serde_json::Value)> {
    sqlx::query!("SELECT kind, payload FROM jobs ORDER BY id")
        .fetch_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.kind, row.payload))
        .collect()
}

async fn events(user_id: i32, db: &PgPool) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT kind FROM account_events WHERE user_id = $1 ORDER BY id",
        user_id
    )
    .fetch_all(db)
    .await
    .unwrap()
}

async fn email_of(user_id: i32, db: &PgPool) -> String {
    sqlx::query_scalar!("SELECT email FROM users WHERE id = $1", user_id)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn an_api_token_can_never_change_the_email(db: PgPool) {
    let user = active_user("token@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let caller = Caller::api_token(user.clone(), 1, ApiScope::ALL.to_vec());

    let outcome = request_email_change(
        &app.env(None),
        &caller,
        &change_to(NEW_EMAIL, TEST_PASSWORD),
        None,
    )
    .await
    .unwrap();

    assert_eq!(outcome, Err(ActionError::Denied(Denied::BrowserOnly)));
    assert!(jobs(&db).await.is_empty());
    assert!(events(user.id, &db).await.is_empty());
}

#[sqlx::test]
async fn a_request_needs_the_current_password_and_a_new_valid_address(db: PgPool) {
    let user = active_user("checks@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let caller = Caller::browser(user.clone());
    let cases = [
        (
            change_to("not-an-email", TEST_PASSWORD),
            ChangeEmailProblem::InvalidEmail,
        ),
        (
            change_to(" Checks@Example.test ", TEST_PASSWORD),
            ChangeEmailProblem::SameEmail,
        ),
        (
            change_to(NEW_EMAIL, "not-my-password"),
            ChangeEmailProblem::Reauth(ReauthProblem::Password(CurrentPasswordProblem::Wrong)),
        ),
    ];

    for (input, problem) in cases {
        let outcome = request_email_change(&env, &caller, &input, None)
            .await
            .unwrap();
        assert_eq!(outcome, Err(ActionError::Problem(problem)));
    }
    assert!(jobs(&db).await.is_empty());
    assert!(events(user.id, &db).await.is_empty());
}

#[sqlx::test]
async fn a_request_queues_the_link_and_is_logged_without_changing_the_email(db: PgPool) {
    let user = active_user("before@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let caller = Caller::browser(user.clone());

    request_email_change(
        &app.env(None),
        &caller,
        &change_to(" New@Example.test ", TEST_PASSWORD),
        None,
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(
        jobs(&db).await,
        vec![(
            EmailChangeJob::KIND.to_string(),
            json!({ "user_id": user.id, "new_email": NEW_EMAIL })
        )]
    );
    assert_eq!(events(user.id, &db).await, vec!["email_change_requested"]);
    assert_eq!(email_of(user.id, &db).await, "before@example.test");
}

#[sqlx::test]
async fn the_job_sends_a_link_to_a_free_address(db: PgPool) {
    let user = active_user("job@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let job = EmailChangeJob {
        user_id: user.id,
        new_email: NEW_EMAIL.to_string(),
    };

    job.run(&app).await.unwrap();

    let link =
        sqlx::query!("SELECT user_id, payload FROM user_tokens WHERE purpose = 'email_change'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(
        (link.user_id, link.payload.as_deref()),
        (user.id, Some(NEW_EMAIL))
    );
    let sent = jobs(&db).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].1["to"], NEW_EMAIL);
    assert_eq!(sent[0].1["subject"], "Confirm your new email");
}

#[sqlx::test]
async fn the_job_only_warns_an_address_that_already_has_an_account(db: PgPool) {
    let user = active_user("mine@example.test", &db).await;
    active_user(NEW_EMAIL, &db).await;
    let app = App::for_tests(db.clone());
    let job = EmailChangeJob {
        user_id: user.id,
        new_email: NEW_EMAIL.to_string(),
    };

    job.run(&app).await.unwrap();

    let links = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM user_tokens"#)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(links, 0);
    let sent = jobs(&db).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].1["subject"], "This email already has an account");
}

#[sqlx::test]
async fn confirming_switches_the_email_once_and_tells_the_old_address(db: PgPool) {
    let user = active_user("old@example.test", &db).await;
    insert_link(user.id, NEW_EMAIL, &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);

    assert_eq!(
        pending_email_change(&env, RAW_TOKEN)
            .await
            .unwrap()
            .as_deref(),
        Some(NEW_EMAIL)
    );
    assert_eq!(email_of(user.id, &db).await, "old@example.test");
    let changed = confirm_email_change(&env, RAW_TOKEN)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(changed.email, NEW_EMAIL);
    assert_eq!(changed.session_version, user.session_version + 1);
    assert_eq!(email_of(user.id, &db).await, NEW_EMAIL);
    assert_eq!(events(user.id, &db).await, vec!["email_changed"]);
    let sent = jobs(&db).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].1["to"], "old@example.test");
    assert_eq!(sent[0].1["subject"], "Your email was changed");
    assert_eq!(
        confirm_email_change(&env, RAW_TOKEN).await.unwrap().err(),
        Some(ConfirmEmailProblem::ExpiredLink)
    );
    assert_eq!(pending_email_change(&env, RAW_TOKEN).await.unwrap(), None);
}

#[sqlx::test]
async fn confirming_an_address_taken_in_the_meantime_changes_nothing(db: PgPool) {
    let user = active_user("first@example.test", &db).await;
    insert_link(user.id, NEW_EMAIL, &db).await;
    active_user(NEW_EMAIL, &db).await;
    let app = App::for_tests(db.clone());

    let outcome = confirm_email_change(&app.env(None), RAW_TOKEN)
        .await
        .unwrap();

    assert_eq!(outcome.err(), Some(ConfirmEmailProblem::EmailTaken));
    assert_eq!(email_of(user.id, &db).await, "first@example.test");
    assert!(events(user.id, &db).await.is_empty());
    assert!(jobs(&db).await.is_empty());
}

#[test]
fn the_request_schema_is_strict() {
    let schema = serde_json::to_value(schemars::schema_for!(ChangeEmail)).unwrap();

    assert_eq!(schema["required"], json!(["new_email"]));
    assert_eq!(schema["additionalProperties"], json!(false));
}

#[test]
fn every_problem_names_a_real_field() {
    let fields = [NEW_EMAIL_FIELD, "current_password"];
    for problem in [
        ChangeEmailProblem::InvalidEmail,
        ChangeEmailProblem::SameEmail,
        ChangeEmailProblem::Reauth(ReauthProblem::Password(CurrentPasswordProblem::Wrong)),
    ] {
        assert!(fields.contains(&problem.field().unwrap()));
    }
}

#[sqlx::test]
async fn changing_or_resetting_the_password_cancels_a_pending_email_change(db: PgPool) {
    use crate::auth::actions::{
        ChangePassword, NewPassword, change_password, set_password_from_link,
    };

    let changer = active_user("changer@example.test", &db).await;
    insert_link(changer.id, NEW_EMAIL, &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let new_password = "a-brand-new-password".to_string();
    let input = ChangePassword {
        current_password: TEST_PASSWORD.to_string(),
        password: new_password.clone(),
        confirm_password: new_password.clone(),
    };
    change_password(&env, &Caller::browser(changer), &input)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending_email_change(&env, RAW_TOKEN).await.unwrap(), None);

    let resetter = active_user("resetter@example.test", &db).await;
    insert_link(resetter.id, NEW_EMAIL, &db).await;
    let reset = NewUserToken {
        user_id: resetter.id,
        purpose: TokenPurpose::PasswordReset,
        token_hash: &hash_secret("a-raw-reset-token"),
        payload: None,
        expires_at: time::OffsetDateTime::now_utc() + time::Duration::hours(1),
    };
    query_insert_user_token(&reset, &db).await.unwrap();
    let chosen = NewPassword {
        password: new_password.clone(),
        confirm_password: new_password,
        code: String::new(),
    };
    set_password_from_link(
        &env,
        TokenPurpose::PasswordReset,
        "a-raw-reset-token",
        &chosen,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(pending_email_change(&env, RAW_TOKEN).await.unwrap(), None);
}

#[sqlx::test]
async fn a_passkey_confirmation_stands_in_for_the_password(db: PgPool) {
    let user = active_user("passkey@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let caller = Caller::browser(user.clone());
    let mine = Some(PasskeyProof::for_tests(user.id));
    let someone_elses = Some(PasskeyProof::for_tests(user.id + 1));

    let refused = request_email_change(&env, &caller, &change_to(NEW_EMAIL, ""), someone_elses)
        .await
        .unwrap();
    assert_eq!(
        refused,
        Err(ActionError::Problem(ChangeEmailProblem::Reauth(
            ReauthProblem::Missing
        )))
    );
    request_email_change(&env, &caller, &change_to(NEW_EMAIL, ""), mine)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(events(user.id, &db).await, vec!["email_change_requested"]);
}

#[sqlx::test]
async fn a_disabled_accounts_links_and_queued_jobs_do_nothing(db: PgPool) {
    let user = active_user("frozen@example.test", &db).await;
    insert_link(user.id, NEW_EMAIL, &db).await;
    sqlx::query!(
        "UPDATE users SET disabled_at = now() WHERE id = $1",
        user.id
    )
    .execute(&db)
    .await
    .unwrap();
    let app = App::for_tests(db.clone());
    let env = app.env(None);

    assert_eq!(pending_email_change(&env, RAW_TOKEN).await.unwrap(), None);
    let confirmed = confirm_email_change(&env, RAW_TOKEN).await.unwrap();
    assert!(matches!(confirmed, Err(ConfirmEmailProblem::ExpiredLink)));
    let job = EmailChangeJob {
        user_id: user.id,
        new_email: "another@example.test".to_string(),
    };
    job.run(&app).await.unwrap();
    assert!(jobs(&db).await.is_empty());
}
