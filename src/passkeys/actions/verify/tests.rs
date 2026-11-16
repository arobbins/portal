use std::sync::Arc;

use sqlx::PgPool;
use tower_sessions::{MemoryStore, Session};

use super::*;
use crate::action::{ActionError, Denied};
use crate::api::scope::ApiScope;
use crate::auth::reauth::take_passkey_proof;
use crate::env::App;
use crate::user::testing::active_user;

fn session() -> Session {
    Session::new(None, Arc::new(MemoryStore::default()), None)
}

fn a_credential() -> PublicKeyCredential {
    serde_json::from_value(json!({
        "id": "AAAA",
        "rawId": "AAAA",
        "type": "public-key",
        "response": {
            "authenticatorData": "AAAA",
            "clientDataJSON": "AAAA",
            "signature": "AAAA",
            "userHandle": null
        },
        "extensions": {}
    }))
    .unwrap()
}

#[sqlx::test]
async fn an_api_token_can_never_confirm_with_a_passkey(db: PgPool) {
    let user = active_user("verifier@example.test", &db).await;
    let app = App::for_tests(db);
    let env = app.env(None);
    let caller = Caller::api_token(user, 1, ApiScope::ALL.to_vec());
    let session = session();

    let started = start_passkey_verification(&env, &session, &caller)
        .await
        .unwrap();
    let finished = finish_passkey_verification(&env, &session, &caller, &a_credential())
        .await
        .unwrap();

    assert_eq!(started, Err(ActionError::Denied(Denied::BrowserOnly)));
    assert_eq!(finished, Err(ActionError::Denied(Denied::BrowserOnly)));
}

#[sqlx::test]
async fn an_account_without_passkeys_is_told_to_use_its_password(db: PgPool) {
    let user = active_user("verifier@example.test", &db).await;
    let app = App::for_tests(db);
    let env = app.env(None);
    let caller = Caller::browser(user.clone());

    assert!(!has_passkey(&env, user.id).await.unwrap());
    assert_eq!(
        start_passkey_verification(&env, &session(), &caller)
            .await
            .unwrap(),
        Err(ActionError::Problem(VerifyPasskeyProblem::NoPasskeys))
    );
}

#[sqlx::test]
async fn finishing_without_a_started_request_grants_nothing(db: PgPool) {
    let user = active_user("verifier@example.test", &db).await;
    let app = App::for_tests(db);
    let env = app.env(None);
    let caller = Caller::browser(user.clone());
    let session = session();

    let outcome = finish_passkey_verification(&env, &session, &caller, &a_credential())
        .await
        .unwrap();

    assert_eq!(
        outcome,
        Err(ActionError::Problem(VerifyPasskeyProblem::Ceremony(
            CeremonyProblem::Expired
        )))
    );
    assert_eq!(
        take_passkey_proof(&session, &user, env.now())
            .await
            .unwrap(),
        None
    );
}
