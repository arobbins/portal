use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, Caller, Denied};
use crate::env::App;
use crate::feedback::queries::query_list_recent_api_feedback;
use crate::rate_limit::FEEDBACK_PER_TOKEN_PER_HOUR;
use crate::secret::PublicIdKind;
use crate::user::model::User;
use crate::user::testing::active_user;

fn feedback(message: &str) -> SendApiFeedback {
    SendApiFeedback {
        category: FeedbackCategory::ConfusingDocs,
        endpoint: None,
        problem_code: None,
        request_id: None,
        message: message.to_string(),
    }
}

fn bare_token(user: &Arc<User>) -> Caller {
    Caller::api_token(user.clone(), 1, Vec::new())
}

async fn event_kinds(user_id: i32, db: &PgPool) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT kind FROM account_events WHERE user_id = $1 ORDER BY id",
        user_id
    )
    .fetch_all(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn a_token_without_scopes_sends_feedback_with_its_secrets_removed(db: PgPool) {
    let user = active_user("agent@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let input = SendApiFeedback {
        endpoint: Some("update_profile".to_string()),
        problem_code: Some("name_too_long".to_string()),
        request_id: Some("6F9619FF-8B86-D011-B42D-00CF4FC964FF".to_string()),
        ..feedback("I sent Bearer wsk_secret123 and got name_too_long.")
    };

    let receipt = send_api_feedback(&app.env(None), &bare_token(&user), &input)
        .await
        .unwrap()
        .unwrap();

    assert!(PublicIdKind::Feedback.is_valid(&receipt.id));
    let rows = query_list_recent_api_feedback(10, &db).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].public_id, receipt.id);
    assert_eq!(
        rows[0].message,
        "I sent Bearer [redacted] and got name_too_long."
    );
    assert_eq!(rows[0].endpoint.as_deref(), Some("update-profile"));
    assert_eq!(rows[0].problem_code.as_deref(), Some("name_too_long"));
    assert_eq!(
        rows[0].request_id.as_deref(),
        Some("6f9619ff-8b86-d011-b42d-00cf4fc964ff")
    );
    assert_eq!(event_kinds(user.id, &db).await, vec!["api_feedback_sent"]);
}

#[sqlx::test]
async fn feedback_that_names_something_unknown_is_refused_and_not_stored(db: PgPool) {
    let user = active_user("unknown@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let caller = bare_token(&user);
    let cases = [
        (
            SendApiFeedback {
                endpoint: Some("delete-everything".to_string()),
                ..feedback("Hi")
            },
            FeedbackProblem::UnknownEndpoint,
        ),
        (
            SendApiFeedback {
                problem_code: Some("made_up".to_string()),
                ..feedback("Hi")
            },
            FeedbackProblem::UnknownProblemCode,
        ),
        (
            SendApiFeedback {
                request_id: Some("not-a-uuid".to_string()),
                ..feedback("Hi")
            },
            FeedbackProblem::InvalidRequestId,
        ),
        (feedback("   "), FeedbackProblem::MessageMissing),
        (
            feedback(&"x".repeat(MAX_FEEDBACK_LENGTH + 1)),
            FeedbackProblem::MessageTooLong,
        ),
    ];

    for (input, problem) in cases {
        let result = send_api_feedback(&env, &caller, &input).await.unwrap();
        assert_eq!(result, Err(ActionError::Problem(problem)));
    }
    assert!(
        query_list_recent_api_feedback(10, &db)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(event_kinds(user.id, &db).await.is_empty());
}

#[sqlx::test]
async fn a_token_can_only_send_so_much_feedback_an_hour(db: PgPool) {
    let user = active_user("chatty@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let caller = bare_token(&user);

    for _ in 0..FEEDBACK_PER_TOKEN_PER_HOUR {
        assert!(
            send_api_feedback(&env, &caller, &feedback("Again"))
                .await
                .unwrap()
                .is_ok()
        );
    }
    let refused = send_api_feedback(&env, &caller, &feedback("Again"))
        .await
        .unwrap();

    assert_eq!(
        refused,
        Err(ActionError::Problem(FeedbackProblem::TooManyAttempts))
    );
}

#[sqlx::test]
async fn only_admins_can_read_the_feedback(db: PgPool) {
    let user = active_user("reader@example.test", &db).await;
    let app = App::for_tests(db.clone());

    let listed = list_api_feedback(&app.env(None), &Caller::browser(user))
        .await
        .unwrap();

    assert!(matches!(
        listed,
        Err(ActionError::Denied(Denied::NotPermitted(_)))
    ));
}

#[test]
fn the_input_schema_is_strict_and_documents_its_limits() {
    let schema = serde_json::to_value(schemars::schema_for!(SendApiFeedback)).unwrap();

    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["required"],
        serde_json::json!(["category", "message"])
    );
    assert_eq!(
        schema["properties"]["message"]["maxLength"],
        MAX_FEEDBACK_LENGTH
    );
}

#[sqlx::test]
async fn a_message_that_redaction_makes_too_long_is_refused_not_a_server_error(db: PgPool) {
    let user = active_user("padded@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let message = "wsk_ ".repeat(MAX_FEEDBACK_LENGTH / 5);

    let result = send_api_feedback(&app.env(None), &bare_token(&user), &feedback(&message))
        .await
        .unwrap();

    assert_eq!(
        result,
        Err(ActionError::Problem(FeedbackProblem::MessageTooLong))
    );
}

#[sqlx::test]
async fn an_account_can_only_send_so_much_feedback_a_day_even_from_the_browser(db: PgPool) {
    let user = active_user("daily@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let caller = Caller::browser(user);

    for _ in 0..crate::rate_limit::FEEDBACK_PER_ACCOUNT_PER_DAY {
        assert!(
            send_api_feedback(&env, &caller, &feedback("Again"))
                .await
                .unwrap()
                .is_ok()
        );
    }
    let refused = send_api_feedback(&env, &caller, &feedback("Again"))
        .await
        .unwrap();

    assert_eq!(
        refused,
        Err(ActionError::Problem(FeedbackProblem::TooManyAttempts))
    );
}

#[sqlx::test]
async fn feedback_is_deleted_once_it_is_older_than_the_retention(db: PgPool) {
    let user = active_user("old@example.test", &db).await;
    let mut app = App::for_tests(db.clone());
    send_api_feedback(&app.env(None), &bare_token(&user), &feedback("Old news"))
        .await
        .unwrap()
        .unwrap();

    app.clock = crate::env::Clock::Fixed(
        time::OffsetDateTime::now_utc() + crate::feedback::FEEDBACK_RETENTION
            - time::Duration::hours(1),
    );
    assert_eq!(crate::feedback::delete_old_feedback(&app).await.unwrap(), 0);
    app.clock = crate::env::Clock::Fixed(
        time::OffsetDateTime::now_utc()
            + crate::feedback::FEEDBACK_RETENTION
            + time::Duration::hours(1),
    );
    assert_eq!(crate::feedback::delete_old_feedback(&app).await.unwrap(), 1);
}
