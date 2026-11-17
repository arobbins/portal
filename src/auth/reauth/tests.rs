use std::sync::Arc;

use sqlx::PgPool;
use time::Duration;
use tower_sessions::MemoryStore;

use super::*;
use crate::env::App;
use crate::user::testing::{TEST_PASSWORD, active_user};

fn session() -> Session {
    Session::new(None, Arc::new(MemoryStore::default()), None)
}

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH + Duration::days(20_000) + Duration::seconds(seconds)
}

#[sqlx::test]
async fn a_passkey_proof_lasts_five_minutes_for_one_user_and_sign_in(db: PgPool) {
    let user = active_user("proof@example.test", &db).await;
    let other = active_user("other@example.test", &db).await;
    let signed_in_again = Arc::new(User {
        id: user.id,
        name: user.name.clone(),
        email: user.email.clone(),
        password: user.password.clone(),
        role: user.role,
        verified_at: user.verified_at,
        session_version: user.session_version + 1,
        webauthn_id: user.webauthn_id,
        public_id: user.public_id.clone(),
        disabled_at: user.disabled_at,
        admin_granted_at: user.admin_granted_at,
    });
    let cases = [
        (&user, 300, true),
        (&user, 301, false),
        (&other, 1, false),
        (&signed_in_again, 1, false),
    ];

    for (taker, seconds, valid) in cases {
        let session = session();
        grant_passkey_reauth(&session, &user, at(0)).await.unwrap();
        let proof = take_passkey_proof(&session, taker, at(seconds))
            .await
            .unwrap();
        assert_eq!(proof.is_some(), valid, "{seconds}s");
    }
}

#[sqlx::test]
async fn a_proof_is_used_once_unless_it_is_given_back(db: PgPool) {
    let user = active_user("once@example.test", &db).await;
    let session = session();
    grant_passkey_reauth(&session, &user, at(0)).await.unwrap();

    let proof = take_passkey_proof(&session, &user, at(1)).await.unwrap();
    assert!(proof.is_some());
    assert!(
        take_passkey_proof(&session, &user, at(2))
            .await
            .unwrap()
            .is_none()
    );

    return_passkey_proof(&session, proof).await.unwrap();
    assert!(
        take_passkey_proof(&session, &user, at(3))
            .await
            .unwrap()
            .is_some()
    );

    grant_passkey_reauth(&session, &user, at(4)).await.unwrap();
    forget_passkey_proof(&session).await.unwrap();
    assert!(
        take_passkey_proof(&session, &user, at(5))
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn reauth_takes_a_proof_for_this_user_or_the_right_password(db: PgPool) {
    let user = active_user("reauth@example.test", &db).await;
    let other = active_user("someone@example.test", &db).await;
    let app = App::for_tests(db);
    let env = app.env(None);
    let mine = Some(PasskeyProof::for_tests(user.id));
    let theirs = Some(PasskeyProof::for_tests(other.id));

    assert_eq!(check_reauth(&env, &user, "", mine).await.unwrap(), Ok(()));
    assert_eq!(
        check_reauth(&env, &user, TEST_PASSWORD, None)
            .await
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        check_reauth(&env, &user, "", theirs).await.unwrap(),
        Err(ReauthProblem::Missing)
    );
    assert_eq!(
        check_reauth(&env, &user, "", None).await.unwrap(),
        Err(ReauthProblem::Missing)
    );
    assert_eq!(
        check_reauth(&env, &user, "not-my-password", theirs)
            .await
            .unwrap(),
        Err(ReauthProblem::Password(CurrentPasswordProblem::Wrong))
    );
}

#[test]
fn a_missing_confirmation_points_at_the_password_field() {
    assert_eq!(ReauthProblem::Missing.field(), Some(CURRENT_PASSWORD_FIELD));
}
