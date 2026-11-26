use sqlx::PgPool;

use super::*;
use crate::env::App;
use crate::rate_limit::CLIENT_REGISTRATIONS_PER_IP_PER_HOUR;

fn metadata(uris: &[&str]) -> ClientMetadata {
    ClientMetadata {
        client_name: Some("Claude".to_string()),
        redirect_uris: uris.iter().map(|uri| uri.to_string()).collect(),
        token_endpoint_auth_method: Some("none".to_string()),
        ..ClientMetadata::default()
    }
}

#[test]
fn only_public_clients_with_safe_redirects_may_register() {
    assert!(check_metadata(&metadata(&["https://claude.ai/cb"])).is_ok());
    assert_eq!(
        check_metadata(&metadata(&["http://claude.ai/cb"])).err(),
        Some(RegistrationProblem::InvalidRedirectUri)
    );
    assert_eq!(
        check_metadata(&metadata(&[])).err(),
        Some(RegistrationProblem::InvalidRedirectUri)
    );
    let confidential = ClientMetadata {
        token_endpoint_auth_method: Some("client_secret_basic".to_string()),
        ..metadata(&["https://claude.ai/cb"])
    };
    assert_eq!(
        check_metadata(&confidential).err(),
        Some(RegistrationProblem::InvalidMetadata)
    );
    let implicit = ClientMetadata {
        response_types: Some(vec!["token".to_string()]),
        ..metadata(&["https://claude.ai/cb"])
    };
    assert_eq!(
        check_metadata(&implicit).err(),
        Some(RegistrationProblem::InvalidMetadata)
    );
    let multiline = ClientMetadata {
        client_name: Some("Claude\r\nBcc: someone".to_string()),
        ..metadata(&["https://claude.ai/cb"])
    };
    assert_eq!(
        check_metadata(&multiline).err(),
        Some(RegistrationProblem::InvalidMetadata)
    );
}

#[test]
fn a_client_without_a_name_is_named_after_its_return_host() {
    let unnamed = ClientMetadata {
        client_name: None,
        ..metadata(&["http://127.0.0.1:4000/cb"])
    };

    assert_eq!(check_metadata(&unnamed).unwrap().name, "127.0.0.1");
}

#[sqlx::test]
async fn registration_is_limited_per_ip(db: PgPool) {
    let app = App::for_tests(db);
    let env = app.env(Some("203.0.113.9".parse().unwrap()));
    let input = metadata(&["https://claude.ai/cb"]);

    for _ in 0..CLIENT_REGISTRATIONS_PER_IP_PER_HOUR {
        let client = register_client(&env, &input).await.unwrap().unwrap();
        assert!(PublicIdKind::OAuthClient.is_valid(&client.client_id));
        let found = find_client(&env, &client.client_id).await.unwrap().unwrap();
        assert_eq!(found.redirect_uris, input.redirect_uris);
    }
    assert_eq!(
        register_client(&env, &input).await.unwrap().err(),
        Some(RegistrationProblem::TooManyRegistrations)
    );
    assert_eq!(CLIENT_REGISTRATIONS_PER_IP_PER_HOUR, 20);
}

#[sqlx::test]
async fn unknown_client_ids_find_nothing(db: PgPool) {
    let app = App::for_tests(db);
    let env = app.env(None);

    for unknown in [
        PublicIdKind::OAuthClient.generate(),
        "http://claude.ai/client.json".to_string(),
        "https://localhost/client.json".to_string(),
        "anything".to_string(),
    ] {
        assert_eq!(
            find_client(&env, &unknown).await.unwrap(),
            None,
            "{unknown}"
        );
    }
}

#[sqlx::test]
async fn metadata_documents_are_cached_for_an_hour(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let client_id = "https://client.invalid/oauth/client.json";
    let row = NewClient {
        client_id,
        name: "Cached",
        redirect_uris: &["https://client.invalid/cb".to_string()],
        fetched_at: Some(env.now()),
    };
    query_insert_client(&row, &db).await.unwrap();

    let cached = find_client(&env, client_id).await.unwrap().unwrap();
    assert_eq!(cached.name, "Cached");
    assert_eq!(METADATA_CACHE, time::Duration::hours(1));
}
