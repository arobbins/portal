use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, ConnectedApp, Denied};
use crate::auth::password::CurrentPasswordProblem;
use crate::env::App;
use crate::oauth::testing::{approve, start_request, test_client};
use crate::user::model::Role;
use crate::user::queries::query_get_user_by_id;
use crate::user::testing::active_user;

async fn granted(db: &PgPool, user_id: i32) -> Vec<Vec<String>> {
    sqlx::query_scalar!(
        "SELECT scopes FROM oauth_connections WHERE user_id = $1",
        user_id
    )
    .fetch_all(db)
    .await
    .unwrap()
}

fn ticking(request: &str, scopes: &[ApiScope], password: &str) -> ConnectApp {
    ConnectApp {
        request: request.to_string(),
        scopes: scopes.to_vec(),
        current_password: password.to_string(),
    }
}

#[sqlx::test]
async fn only_a_browser_can_connect_apps(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("apps@example.test", &db).await;
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read").await;
    let connected = ConnectedApp {
        connection_id: 1,
        scopes: ApiScope::ALL.to_vec(),
        label: "Claude (claude.ai)".to_string(),
    };
    let input = ticking(&request, &[ApiScope::ProfileRead], "");

    for caller in [
        Caller::connected_app(user.clone(), connected),
        Caller::api_token(user.clone(), 1, ApiScope::ALL.to_vec()),
    ] {
        assert_eq!(
            connect_app(&env, &caller, &input, None).await.unwrap(),
            Err(ActionError::Denied(Denied::BrowserOnly))
        );
    }
    assert!(granted(&db, user.id).await.is_empty());
}

#[sqlx::test]
async fn write_scopes_need_the_password_and_read_scopes_do_not(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("writer@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read billing:write").await;
    let both = [ApiScope::ProfileRead, ApiScope::BillingWrite];

    let wrong = connect_app(&env, &caller, &ticking(&request, &both, "wrong"), None).await;
    assert_eq!(
        wrong.unwrap(),
        Err(ActionError::Problem(ConnectProblem::Reauth(
            ReauthProblem::Password(CurrentPasswordProblem::Wrong)
        )))
    );
    assert!(granted(&db, user.id).await.is_empty());
    let failed_logins = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM rate_limit_hits WHERE bucket = 'failed_logins_by_account'"#
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(failed_logins, 1);

    let read_only = ticking(&request, &[ApiScope::ProfileRead], "");
    assert!(
        connect_app(&env, &caller, &read_only, None)
            .await
            .unwrap()
            .is_ok()
    );
    assert_eq!(
        granted(&db, user.id).await,
        vec![vec!["profile:read".to_string()]]
    );
}

#[sqlx::test]
async fn admin_scopes_and_unrequested_scopes_are_never_granted(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("boss@example.test", &db).await;
    sqlx::query!(
        "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
        user.id
    )
    .execute(&db)
    .await
    .unwrap();
    let admin = query_get_user_by_id(user.id, &db)
        .await
        .unwrap()
        .map(Arc::new)
        .unwrap();
    assert_eq!(admin.role, Role::Admin);
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read test:admin").await;

    let ticked = [
        ApiScope::ProfileRead,
        ApiScope::TestAdmin,
        ApiScope::BillingRead,
    ];
    approve(&env, &admin, &request, &ticked).await;

    assert_eq!(
        granted(&db, user.id).await,
        vec![vec!["profile:read".to_string()]]
    );
}

#[sqlx::test]
async fn connecting_the_same_client_again_updates_its_scopes(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("twice@example.test", &db).await;
    let client = test_client(&db).await;

    let first = start_request(&env, &client, "profile:read").await;
    approve(&env, &user, &first, &[ApiScope::ProfileRead]).await;
    let second = start_request(&env, &client, "billing:read").await;
    approve(&env, &user, &second, &[ApiScope::BillingRead]).await;

    assert_eq!(
        granted(&db, user.id).await,
        vec![vec!["billing:read".to_string()]]
    );
}

#[sqlx::test]
async fn an_account_has_at_most_fifty_connections(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("many@example.test", &db).await;
    let caller = Caller::browser(user.clone());

    for index in 0..MAX_CONNECTIONS_PER_USER {
        let client = test_client(&db).await;
        let from = app.env(Some([198, 51, 100, index as u8].into()));
        let request = start_request(&from, &client, "profile:read").await;
        approve(&env, &user, &request, &[ApiScope::ProfileRead]).await;
    }
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read").await;
    let input = ticking(&request, &[ApiScope::ProfileRead], "");

    assert_eq!(
        connect_app(&env, &caller, &input, None).await.unwrap(),
        Err(ActionError::Problem(ConnectProblem::TooManyApps))
    );
    assert_eq!(MAX_CONNECTIONS_PER_USER, 50);
}

#[sqlx::test]
async fn denying_connects_nothing_and_ends_the_request(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("no@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read").await;

    let redirect = deny_app(&env, &caller, &request).await.unwrap().unwrap();
    assert!(redirect.contains("error=access_denied"));
    assert!(redirect.contains("state=state-1"));
    let input = ticking(&request, &[ApiScope::ProfileRead], "");
    assert_eq!(
        connect_app(&env, &caller, &input, None).await.unwrap(),
        Err(ActionError::Problem(ConnectProblem::Expired))
    );
    assert!(granted(&db, user.id).await.is_empty());
}
