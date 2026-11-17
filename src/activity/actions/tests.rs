use serde_json::json;
use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, Denied};
use crate::api::scope::ApiScope;
use crate::devices::lifecycle::{SignInMethod, start_device};
use crate::env::App;
use crate::user::events::{EventSource, record_event};
use crate::user::testing::active_user;

const SAFARI: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1";

#[test]
fn every_event_kind_has_its_own_summary() {
    let detail = json!({ "name": "Laptop", "devices": 2, "method": "passkey" });

    for kind in AccountEventKind::ALL {
        assert_ne!(
            summarize(kind, &detail),
            "Account changed",
            "{}",
            kind.as_str()
        );
    }
    assert_eq!(
        summarize(AccountEventKind::SignedIn, &detail),
        "Signed in with a passkey"
    );
    assert_eq!(
        summarize(AccountEventKind::ApiTokenCreated, &detail),
        "API token “Laptop” created"
    );
    assert_eq!(
        summarize(
            AccountEventKind::OtherDevicesSignedOut,
            &json!({ "devices": 1 })
        ),
        "1 other device signed out"
    );
}

#[sqlx::test]
async fn recent_activity_is_newest_first_and_names_the_device(db: PgPool) {
    let user = active_user("history@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env("198.51.100.4".parse().ok());
    start_device(&env, &user, SAFARI, SignInMethod::Passkey)
        .await
        .unwrap();
    let source = EventSource::browser(user.id);
    record_event(
        &env,
        &source,
        AccountEventKind::PasswordChanged,
        json!({}),
        &db,
    )
    .await
    .unwrap();

    let activity = list_recent_activity(&env, &Caller::browser(user.clone()))
        .await
        .unwrap()
        .unwrap();

    let summaries: Vec<&str> = activity.iter().map(|item| item.summary.as_str()).collect();
    assert_eq!(summaries, ["Password changed", "Signed in with a passkey"]);
    assert_eq!(activity[1].device.as_deref(), Some("Safari on iOS"));
    assert_eq!(activity[1].ip.as_deref(), Some("198.51.100.4"));
    assert_eq!(activity[1].by, None);
}

#[sqlx::test]
async fn recent_activity_shows_only_the_latest_events(db: PgPool) {
    let user = active_user("busy@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let source = EventSource::browser(user.id);
    for _ in 0..RECENT_ACTIVITY_LIMIT + 5 {
        record_event(
            &env,
            &source,
            AccountEventKind::ProfileUpdated,
            json!({}),
            &db,
        )
        .await
        .unwrap();
    }

    let activity = list_recent_activity(&env, &Caller::browser(user))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(activity.len() as i64, RECENT_ACTIVITY_LIMIT);
}

#[sqlx::test]
async fn api_tokens_cannot_read_the_activity_log(db: PgPool) {
    let user = active_user("private@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let token = Caller::api_token(user, 1, ApiScope::ALL.to_vec());

    let outcome = list_recent_activity(&app.env(None), &token).await.unwrap();

    assert_eq!(outcome, Err(ActionError::Denied(Denied::BrowserOnly)));
}
