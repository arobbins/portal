use sqlx::PgPool;
use time::Duration;

use super::*;
use crate::activity::actions::activity_of;
use crate::api::scope::ApiScope;
use crate::oauth::testing::{connect_test_app, test_client};
use crate::profile::actions::{UpdateProfile, update_profile};
use crate::user::testing::active_user;

async fn connections(db: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM oauth_connections"#)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_disabled_accounts_apps_stop_working_until_it_is_enabled(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("paused@example.test", &db).await;
    let (_, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let now = app.clock.now();

    sqlx::query!(
        "UPDATE users SET disabled_at = now() WHERE id = $1",
        user.id
    )
    .execute(&db)
    .await
    .unwrap();
    assert!(
        app_caller(&tokens.access_token, now, &db)
            .await
            .unwrap()
            .is_none()
    );

    sqlx::query!("UPDATE users SET disabled_at = NULL WHERE id = $1", user.id)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        app_caller(&tokens.access_token, now, &db)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn deleting_the_account_deletes_its_apps(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("gone@example.test", &db).await;
    connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    assert_eq!(connections(&db).await, 1);

    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&db)
        .await
        .unwrap();

    assert_eq!(connections(&db).await, 0);
    let tokens = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM oauth_tokens"#)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(tokens, 0);
}

#[sqlx::test]
async fn a_sign_in_token_names_its_app_in_recent_activity(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("named@example.test", &db).await;
    let (_, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileWrite]).await;
    let caller = app_caller(&tokens.access_token, app.clock.now(), &db)
        .await
        .unwrap()
        .unwrap();

    let input = UpdateProfile {
        name: "Renamed".to_string(),
    };
    update_profile(&env, &caller, &input)
        .await
        .unwrap()
        .unwrap();

    let activity = activity_of(&env, user.id).await.unwrap();
    assert_eq!(activity[0].summary, "Profile updated");
    assert_eq!(activity[0].by.as_deref(), Some("By Claude (claude.ai)"));
    assert_eq!(activity[1].summary, "Connected Claude (claude.ai)");
    assert_eq!(activity[1].by, None);
}

#[sqlx::test]
async fn sign_in_tokens_never_reach_browser_only_actions(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("limits@example.test", &db).await;
    let (_, tokens) = connect_test_app(&app, &user, &ApiScope::ALL).await;
    let caller = app_caller(&tokens.access_token, app.clock.now(), &db)
        .await
        .unwrap()
        .unwrap();

    for permission in crate::permission::Permission::ALL {
        if permission.token_access() == crate::permission::TokenAccess::BrowserOnly
            && !permission.admins_only()
        {
            assert_eq!(
                caller.require(permission),
                Err(crate::action::Denied::BrowserOnly),
                "{permission:?}"
            );
        }
    }
    assert_eq!(caller.api_token_id(), None);
}

#[sqlx::test]
async fn cleanup_forgets_unapproved_clients_after_a_day(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("kept@example.test", &db).await;
    let (approved, _) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let abandoned = test_client(&db).await;
    let recent = test_client(&db).await;
    sqlx::query!(
        "UPDATE oauth_clients SET created_at = now() - interval '25 hours' WHERE client_id = ANY($1)",
        &[approved.clone(), abandoned.clone()]
    )
    .execute(&db)
    .await
    .unwrap();

    delete_expired(&app).await.unwrap();

    let left = sqlx::query_scalar!("SELECT client_id FROM oauth_clients ORDER BY id")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(left, vec![approved, recent]);
    assert_eq!(UNAPPROVED_CLIENT_LIFETIME, Duration::hours(24));
}

#[sqlx::test]
async fn disconnecting_everything_ends_every_app(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("all@example.test", &db).await;
    let (_, first) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    connect_test_app(&app, &user, &[ApiScope::BillingRead]).await;

    assert_eq!(disconnect_all_for_user(user.id, &db).await.unwrap(), 2);
    let now = app.clock.now();
    assert!(
        app_caller(&first.access_token, now, &db)
            .await
            .unwrap()
            .is_none()
    );
}
