use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, Denied};
use crate::api::scope::ApiScope;
use crate::auth::password::CurrentPasswordProblem;
use crate::auth::reauth::{PasskeyProof, ReauthProblem};
use crate::env::App;
use crate::user::testing::{TEST_PASSWORD, active_user};

fn confirm(confirmation: &str, current_password: &str) -> DeleteAccount {
    DeleteAccount {
        confirmation: confirmation.to_string(),
        current_password: current_password.to_string(),
    }
}

async fn exists(user_id: i32, db: &PgPool) -> bool {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM users WHERE id = $1) AS "exists!""#,
        user_id
    )
    .fetch_one(db)
    .await
    .unwrap()
}

async fn sent_to(db: &PgPool) -> Vec<(String, String)> {
    sqlx::query!("SELECT payload FROM jobs WHERE kind = 'send_email' ORDER BY id")
        .fetch_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                row.payload["to"].as_str().unwrap().to_string(),
                row.payload["subject"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[sqlx::test]
async fn an_api_token_can_never_delete_the_account(db: PgPool) {
    let user = active_user("token@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let caller = Caller::api_token(user.clone(), 1, ApiScope::ALL.to_vec());

    let outcome = delete_account(
        &app.env(None),
        &caller,
        &confirm("DELETE", TEST_PASSWORD),
        None,
    )
    .await
    .unwrap();

    assert_eq!(outcome, Err(ActionError::Denied(Denied::BrowserOnly)));
    assert!(exists(user.id, &db).await);
}

#[sqlx::test]
async fn deleting_needs_the_typed_confirmation_and_proof_of_identity(db: PgPool) {
    let user = active_user("careful@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let caller = Caller::browser(user.clone());
    let cases = [
        (
            confirm("delete", TEST_PASSWORD),
            DeleteAccountProblem::Confirmation,
        ),
        (
            confirm("", TEST_PASSWORD),
            DeleteAccountProblem::Confirmation,
        ),
        (
            confirm("DELETE", ""),
            DeleteAccountProblem::Reauth(ReauthProblem::Missing),
        ),
        (
            confirm("DELETE", "not-my-password"),
            DeleteAccountProblem::Reauth(ReauthProblem::Password(CurrentPasswordProblem::Wrong)),
        ),
    ];

    for (input, problem) in cases {
        let outcome = delete_account(&env, &caller, &input, None).await.unwrap();
        assert_eq!(outcome, Err(ActionError::Problem(problem)));
    }
    assert!(exists(user.id, &db).await);
    assert!(sent_to(&db).await.is_empty());
}

#[sqlx::test]
async fn deleting_removes_the_account_and_everything_it_owns(db: PgPool) {
    let user = active_user("gone@example.test", &db).await;
    let other = active_user("stays@example.test", &db).await;
    crate::api_tokens::testing::issue_token(
        user.id,
        &["profile:read"],
        time::Duration::days(1),
        &db,
    )
    .await;
    let app = App::for_tests(db.clone());
    let caller = Caller::browser(user.clone());

    delete_account(
        &app.env(None),
        &caller,
        &confirm(" DELETE ", TEST_PASSWORD),
        None,
    )
    .await
    .unwrap()
    .unwrap();

    assert!(!exists(user.id, &db).await);
    assert!(exists(other.id, &db).await);
    let leftover_tokens = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM api_tokens WHERE user_id = $1"#,
        user.id
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(leftover_tokens, 0);
    assert_eq!(
        sent_to(&db).await,
        vec![(
            "gone@example.test".to_string(),
            "Your account was deleted".to_string()
        )]
    );
}

#[sqlx::test]
async fn a_passkey_confirmation_can_stand_in_for_the_password(db: PgPool) {
    let user = active_user("passkey@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let caller = Caller::browser(user.clone());
    let proof = Some(PasskeyProof::for_tests(user.id));

    delete_account(&app.env(None), &caller, &confirm("DELETE", ""), proof)
        .await
        .unwrap()
        .unwrap();

    assert!(!exists(user.id, &db).await);
}

#[test]
fn the_confirmation_problem_names_its_field_and_word() {
    let spec = DeleteAccountProblem::Confirmation.spec();
    assert_eq!(spec.field, Some(CONFIRMATION_FIELD));
    assert!(spec.message.contains(CONFIRMATION_WORD));
}
