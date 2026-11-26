use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use super::*;
use crate::env::{App, Clock};
use crate::oauth::testing::{REDIRECT_URI, authorize_params, start_request, test_client};

fn returned_error(started: Started) -> String {
    let Started::ReturnError(url) = started else {
        panic!("expected a redirect with an error, got {started:?}");
    };
    let url = url::Url::parse(&url).unwrap();
    assert!(url.as_str().starts_with(REDIRECT_URI));
    url.query_pairs()
        .find(|(key, _)| key == "error")
        .map(|(_, error)| error.into_owned())
        .unwrap()
}

#[test]
fn unknown_and_admin_only_scopes_are_never_requested() {
    assert_eq!(
        requested_scopes("profile:read test:admin nonsense profile:read billing:write"),
        vec![ApiScope::ProfileRead, ApiScope::BillingWrite]
    );
    assert!(!ApiScope::TestAdmin.offered_by_sign_in());
    assert_eq!(requested_scopes(""), Vec::<ApiScope>::new());
}

#[sqlx::test]
async fn an_unknown_client_or_redirect_is_refused_without_redirecting(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let client = test_client(&db).await;

    let unknown = authorize_params(&env, &PublicIdKind::OAuthClient.generate(), "profile:read");
    assert_eq!(
        start_authorization(&env, &unknown).await.unwrap(),
        Started::Refused(CantConnect::UnknownClient)
    );
    let elsewhere = AuthorizeParams {
        redirect_uri: Some("https://claude.ai/other".to_string()),
        ..authorize_params(&env, &client, "profile:read")
    };
    assert_eq!(
        start_authorization(&env, &elsewhere).await.unwrap(),
        Started::Refused(CantConnect::UnregisteredRedirect)
    );
}

#[sqlx::test]
async fn pkce_s256_and_this_resource_are_required(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let client = test_client(&db).await;
    let valid = authorize_params(&env, &client, "profile:read");

    let plain = AuthorizeParams {
        code_challenge_method: Some("plain".to_string()),
        ..valid.clone()
    };
    let no_challenge = AuthorizeParams {
        code_challenge: None,
        ..valid.clone()
    };
    let other_resource = AuthorizeParams {
        resource: Some("https://elsewhere.test/mcp".to_string()),
        ..valid.clone()
    };
    let implicit = AuthorizeParams {
        response_type: Some("token".to_string()),
        ..valid.clone()
    };
    let start = async |params| start_authorization(&env, &params).await.unwrap();

    assert_eq!(returned_error(start(plain).await), "invalid_request");
    assert_eq!(returned_error(start(no_challenge).await), "invalid_request");
    assert_eq!(
        returned_error(start(other_resource).await),
        "invalid_target"
    );
    assert_eq!(
        returned_error(start(implicit).await),
        "unsupported_response_type"
    );
    assert!(matches!(start(valid).await, Started::Request(_)));
}

#[sqlx::test]
async fn a_request_expires_ten_minutes_after_it_was_last_used(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let client = test_client(&db).await;
    let request = start_request(&env, &client, "profile:read").await;
    let moved = |minutes| App {
        clock: Clock::Fixed(OffsetDateTime::now_utc() + Duration::minutes(minutes)),
        ..app.clone()
    };

    let eight = moved(8);
    assert!(
        live_request(&eight.env(None), &request)
            .await
            .unwrap()
            .is_some()
    );
    keep_request_alive(&eight.env(None), &request)
        .await
        .unwrap();
    let sixteen = moved(16);
    assert!(
        live_request(&sixteen.env(None), &request)
            .await
            .unwrap()
            .is_some()
    );
    let thirty = moved(30);
    assert!(
        live_request(&thirty.env(None), &request)
            .await
            .unwrap()
            .is_none()
    );
    keep_request_alive(&thirty.env(None), &request)
        .await
        .unwrap();
    assert!(
        live_request(&thirty.env(None), &request)
            .await
            .unwrap()
            .is_none()
    );
}
