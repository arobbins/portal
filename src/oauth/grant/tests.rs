use std::sync::Arc;

use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use super::*;
use crate::api::scope::ApiScope;
use crate::env::{App, Clock};
use crate::oauth::lifecycle::app_caller;
use crate::oauth::testing::{
    app_still_works, approve, code_exchange, connect_test_app, refresh_request, start_request,
    test_client,
};
use crate::rate_limit::TOKEN_REQUESTS_PER_CLIENT_PER_MINUTE;
use crate::user::model::User;
use crate::user::testing::active_user;

fn at(app: &App, moment: OffsetDateTime) -> App {
    App {
        clock: Clock::Fixed(moment),
        ..app.clone()
    }
}

async fn connections(db: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM oauth_connections"#)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_code_works_once_with_the_right_verifier_and_resource(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("code@example.test", &db).await;
    let client = test_client(&db).await;

    let request = start_request(&env, &client, "profile:read").await;
    let code = approve(&env, &user, &request, &[ApiScope::ProfileRead]).await;
    let wrong_verifier = TokenRequest {
        code_verifier: Some("not-the-verifier-not-the-verifier-not-the-verifier".to_string()),
        ..code_exchange(&env, &client, &code)
    };
    assert_eq!(
        issue_tokens(&env, &wrong_verifier).await.unwrap().err(),
        Some(TokenProblem::InvalidGrant)
    );
    assert_eq!(
        issue_tokens(&env, &code_exchange(&env, &client, &code))
            .await
            .unwrap()
            .err(),
        Some(TokenProblem::InvalidGrant),
        "a code is spent by the first attempt"
    );

    let request = start_request(&env, &client, "profile:read").await;
    let code = approve(&env, &user, &request, &[ApiScope::ProfileRead]).await;
    let wrong_resource = TokenRequest {
        resource: Some("https://elsewhere.test/mcp".to_string()),
        ..code_exchange(&env, &client, &code)
    };
    assert_eq!(
        issue_tokens(&env, &wrong_resource).await.unwrap().err(),
        Some(TokenProblem::InvalidGrant)
    );
    let tokens = issue_tokens(&env, &code_exchange(&env, &client, &code))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tokens.scope, "profile:read");
    assert_eq!(tokens.expires_in, 3600);
    assert!(
        issue_tokens(&env, &code_exchange(&env, &client, &code))
            .await
            .unwrap()
            .is_err()
    );
}

#[sqlx::test]
async fn a_code_needs_the_same_redirect_uri_it_was_issued_for(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("redirect@example.test", &db).await;
    let client = test_client(&db).await;

    for redirect_uri in [None, Some("https://claude.ai/elsewhere".to_string())] {
        let request = start_request(&env, &client, "profile:read").await;
        let code = approve(&env, &user, &request, &[ApiScope::ProfileRead]).await;
        let exchange = TokenRequest {
            redirect_uri: redirect_uri.clone(),
            ..code_exchange(&env, &client, &code)
        };
        assert_eq!(
            issue_tokens(&env, &exchange).await.unwrap().err(),
            Some(TokenProblem::InvalidGrant),
            "{redirect_uri:?}"
        );
    }
}

#[sqlx::test]
async fn codes_expire_after_ten_minutes(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("late@example.test", &db).await;
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read").await;
    let code = approve(&env, &user, &request, &[ApiScope::ProfileRead]).await;

    let later = at(
        &app,
        OffsetDateTime::now_utc() + CODE_LIFETIME + Duration::seconds(5),
    );
    let exchange = code_exchange(&later.env(None), &client, &code);
    assert_eq!(
        issue_tokens(&later.env(None), &exchange)
            .await
            .unwrap()
            .err(),
        Some(TokenProblem::InvalidGrant)
    );
}

#[sqlx::test]
async fn access_tokens_last_an_hour_and_refresh_tokens_thirty_idle_days(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("expiry@example.test", &db).await;
    let (client, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let now = OffsetDateTime::now_utc();

    let in_time = now + Duration::minutes(59);
    let too_late = now + Duration::minutes(61);
    assert!(
        app_caller(&tokens.access_token, in_time, &db)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        app_caller(&tokens.access_token, too_late, &db)
            .await
            .unwrap()
            .is_none()
    );

    let lapsed = at(&app, now + Duration::days(31));
    let refresh = refresh_request(&lapsed.env(None), &client, &tokens.refresh_token);
    assert_eq!(
        issue_tokens(&lapsed.env(None), &refresh)
            .await
            .unwrap()
            .err(),
        Some(TokenProblem::InvalidGrant)
    );
    let used = at(&app, now + Duration::days(29));
    let refresh = refresh_request(&used.env(None), &client, &tokens.refresh_token);
    assert!(
        issue_tokens(&used.env(None), &refresh)
            .await
            .unwrap()
            .is_ok()
    );
}

#[sqlx::test]
async fn reusing_a_replaced_refresh_token_disconnects_the_app(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("rotate@example.test", &db).await;
    let (client, first) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;

    let second = issue_tokens(&env, &refresh_request(&env, &client, &first.refresh_token))
        .await
        .unwrap()
        .unwrap();
    let now = OffsetDateTime::now_utc();
    assert!(
        app_caller(&first.access_token, now, &db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        app_caller(&second.access_token, now, &db)
            .await
            .unwrap()
            .is_some()
    );

    let reused = issue_tokens(&env, &refresh_request(&env, &client, &first.refresh_token)).await;
    assert_eq!(reused.unwrap().err(), Some(TokenProblem::InvalidGrant));
    assert!(
        app_caller(&second.access_token, now, &db)
            .await
            .unwrap()
            .is_none()
    );
    let again = issue_tokens(&env, &refresh_request(&env, &client, &second.refresh_token)).await;
    assert_eq!(again.unwrap().err(), Some(TokenProblem::InvalidGrant));
    assert_eq!(connections(&db).await, 0);
}

#[sqlx::test]
async fn a_refresh_token_only_works_for_its_own_client(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("owner@example.test", &db).await;
    let (_, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let other = test_client(&db).await;

    let stolen = issue_tokens(&env, &refresh_request(&env, &other, &tokens.refresh_token)).await;
    assert_eq!(stolen.unwrap().err(), Some(TokenProblem::InvalidGrant));
    revoke_token(&env, &tokens.refresh_token, &other)
        .await
        .unwrap();
    assert_eq!(connections(&db).await, 1);
}

#[sqlx::test]
async fn revoking_either_token_ends_the_connection(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("revoke@example.test", &db).await;
    let (client, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;

    revoke_token(&env, &tokens.access_token, &client)
        .await
        .unwrap();
    assert_eq!(connections(&db).await, 0);
    let refresh = issue_tokens(&env, &refresh_request(&env, &client, &tokens.refresh_token)).await;
    assert_eq!(refresh.unwrap().err(), Some(TokenProblem::InvalidGrant));
    revoke_token(&env, "wsr_garbage", &client).await.unwrap();
}

#[sqlx::test]
async fn the_token_endpoint_is_limited_per_client(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let client = test_client(&db).await;
    let request = refresh_request(&env, &client, "wsr_unknown");

    for _ in 0..TOKEN_REQUESTS_PER_CLIENT_PER_MINUTE {
        assert_eq!(
            issue_tokens(&env, &request).await.unwrap().err(),
            Some(TokenProblem::InvalidGrant)
        );
    }
    assert_eq!(
        issue_tokens(&env, &request).await.unwrap().err(),
        Some(TokenProblem::TooManyRequests)
    );
    assert_eq!(TOKEN_REQUESTS_PER_CLIENT_PER_MINUTE, 60);
}

#[test]
fn pkce_uses_s256_base64url_without_padding() {
    assert_eq!(
        challenge_of("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}

async fn connect_again(app: &App, user: &Arc<User>, client: &str) -> TokenResponse {
    let env = app.env(None);
    let request = start_request(&env, client, "profile:read profile:write").await;
    let scopes = [ApiScope::ProfileRead, ApiScope::ProfileWrite];
    let code = approve(&env, user, &request, &scopes).await;
    issue_tokens(&env, &code_exchange(&env, client, &code))
        .await
        .unwrap()
        .unwrap()
}

async fn refresh_with(app: &App, client: &str, token: &str) -> Option<TokenProblem> {
    let env = app.env(None);
    issue_tokens(&env, &refresh_request(&env, client, token))
        .await
        .unwrap()
        .err()
}

#[sqlx::test]
async fn reconnecting_revokes_every_earlier_token_and_keeps_the_connection(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("again@example.test", &db).await;
    let (client, first) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let refreshed = issue_tokens(&env, &refresh_request(&env, &client, &first.refresh_token))
        .await
        .unwrap()
        .unwrap();

    let current = connect_again(&app, &user, &client).await;

    assert!(!app_still_works(&app, &first).await);
    assert!(!app_still_works(&app, &refreshed).await);
    for earlier in [&first.refresh_token, &refreshed.refresh_token] {
        assert_eq!(
            refresh_with(&app, &client, earlier).await,
            Some(TokenProblem::InvalidGrant)
        );
    }
    assert_eq!(connections(&db).await, 1);
    assert!(app_still_works(&app, &current).await);
    assert_eq!(current.scope, "profile:read profile:write");
    assert_eq!(
        refresh_with(&app, &client, &current.refresh_token).await,
        None
    );
}

#[sqlx::test]
async fn reconnecting_revokes_an_earlier_approvals_unused_code(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("unused-code@example.test", &db).await;
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read").await;
    let earlier = approve(&env, &user, &request, &[ApiScope::ProfileRead]).await;

    let current = connect_again(&app, &user, &client).await;

    assert_eq!(
        issue_tokens(&env, &code_exchange(&env, &client, &earlier))
            .await
            .unwrap()
            .err(),
        Some(TokenProblem::InvalidGrant)
    );
    assert!(app_still_works(&app, &current).await);
    assert_eq!(connections(&db).await, 1);
}

async fn block_new_refresh_tokens(db: &PgPool) {
    sqlx::query!(
        "ALTER TABLE oauth_tokens ADD CONSTRAINT test_no_new_refresh CHECK (kind <> 'refresh' OR replaced_at IS NOT NULL) NOT VALID"
    )
    .execute(db)
    .await
    .unwrap();
}

async fn allow_new_refresh_tokens(db: &PgPool) {
    sqlx::query!("ALTER TABLE oauth_tokens DROP CONSTRAINT test_no_new_refresh")
        .execute(db)
        .await
        .unwrap();
}

#[sqlx::test]
async fn a_code_exchange_that_fails_part_way_changes_nothing(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("atomic-code@example.test", &db).await;
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read").await;
    let code = approve(&env, &user, &request, &[ApiScope::ProfileRead]).await;

    block_new_refresh_tokens(&db).await;
    let failed = issue_tokens(&env, &code_exchange(&env, &client, &code)).await;
    assert!(failed.is_err());
    allow_new_refresh_tokens(&db).await;

    let tokens = issue_tokens(&env, &code_exchange(&env, &client, &code))
        .await
        .unwrap()
        .unwrap();
    assert!(app_still_works(&app, &tokens).await);
}

#[sqlx::test]
async fn a_refresh_that_fails_part_way_changes_nothing(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("atomic-refresh@example.test", &db).await;
    let (client, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;

    block_new_refresh_tokens(&db).await;
    let failed = issue_tokens(&env, &refresh_request(&env, &client, &tokens.refresh_token)).await;
    assert!(failed.is_err());
    allow_new_refresh_tokens(&db).await;

    assert!(app_still_works(&app, &tokens).await);
    assert_eq!(
        refresh_with(&app, &client, &tokens.refresh_token).await,
        None
    );
    assert_eq!(connections(&db).await, 1);
}

async fn set_disabled(db: &PgPool, user: &User, disabled: bool) {
    sqlx::query!(
        "UPDATE users SET disabled_at = CASE WHEN $2 THEN now() END WHERE id = $1",
        user.id,
        disabled,
    )
    .execute(db)
    .await
    .unwrap();
}

#[sqlx::test]
async fn refreshing_while_disabled_is_refused_and_keeps_the_connection(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("paused-refresh@example.test", &db).await;
    let (client, tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;

    set_disabled(&db, &user, true).await;
    assert_eq!(
        refresh_with(&app, &client, &tokens.refresh_token).await,
        Some(TokenProblem::InvalidGrant)
    );
    assert_eq!(connections(&db).await, 1);

    set_disabled(&db, &user, false).await;
    assert!(app_still_works(&app, &tokens).await);
    assert_eq!(
        refresh_with(&app, &client, &tokens.refresh_token).await,
        None
    );
    assert_eq!(connections(&db).await, 1);
}
