use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, Denied};
use crate::api::scope::ApiScope;
use crate::devices::lifecycle::{SignInMethod, end_device, keep_device, load_device, start_device};
use crate::env::App;
use crate::user::testing::active_user;

const CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0";

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
async fn a_signed_in_device_is_listed_until_it_signs_out(db: PgPool) {
    let user = active_user("listed@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env("203.0.113.7".parse().ok());
    let laptop = start_device(&env, &user, CHROME, SignInMethod::Password)
        .await
        .unwrap();
    let caller = Caller::browser(user.clone());

    let devices = list_devices(&env, &caller, Some(&laptop))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].name, "Chrome on macOS");
    assert_eq!(devices[0].ip.as_deref(), Some("203.0.113.7"));
    assert!(devices[0].current);
    assert!(load_device(&app, user.id, &laptop).await.unwrap().is_some());

    end_device(&app, user.id, &laptop).await.unwrap();
    assert!(load_device(&app, user.id, &laptop).await.unwrap().is_none());
    assert_eq!(event_kinds(user.id, &db).await, ["signed_in"]);
}

#[sqlx::test]
async fn signing_out_another_device_ends_only_that_session(db: PgPool) {
    let user = active_user("other@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let here = start_device(&env, &user, CHROME, SignInMethod::Password)
        .await
        .unwrap();
    let there = start_device(&env, &user, FIREFOX, SignInMethod::Passkey)
        .await
        .unwrap();
    let caller = Caller::browser(user.clone());
    let input = |id: &str| SignOutDevice { id: id.to_string() };

    let this_one = sign_out_device(&env, &caller, &input(&here), Some(&here))
        .await
        .unwrap();
    assert_eq!(
        this_one,
        Err(ActionError::Problem(SignOutDeviceProblem::ThisDevice))
    );

    sign_out_device(&env, &caller, &input(&there), Some(&here))
        .await
        .unwrap()
        .unwrap();
    assert!(load_device(&app, user.id, &there).await.unwrap().is_none());
    assert!(load_device(&app, user.id, &here).await.unwrap().is_some());

    let again = sign_out_device(&env, &caller, &input(&there), Some(&here))
        .await
        .unwrap();
    assert_eq!(
        again,
        Err(ActionError::Problem(SignOutDeviceProblem::NotFound))
    );
    let malformed = sign_out_device(&env, &caller, &input("dev_nope"), Some(&here))
        .await
        .unwrap();
    assert_eq!(
        malformed,
        Err(ActionError::Problem(SignOutDeviceProblem::NotFound))
    );
}

#[sqlx::test]
async fn nobody_can_sign_out_someone_elses_device(db: PgPool) {
    let owner = active_user("owner@example.test", &db).await;
    let intruder = active_user("intruder@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let device = start_device(&env, &owner, CHROME, SignInMethod::Password)
        .await
        .unwrap();

    let outcome = sign_out_device(
        &env,
        &Caller::browser(intruder),
        &SignOutDevice { id: device.clone() },
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        outcome,
        Err(ActionError::Problem(SignOutDeviceProblem::NotFound))
    );
    assert!(
        load_device(&app, owner.id, &device)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn signing_out_other_devices_keeps_this_one_and_bumps_the_session_version(db: PgPool) {
    let user = active_user("everywhere@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let here = start_device(&env, &user, CHROME, SignInMethod::Password)
        .await
        .unwrap();
    let phone = start_device(&env, &user, FIREFOX, SignInMethod::Passkey)
        .await
        .unwrap();
    let tablet = start_device(&env, &user, FIREFOX, SignInMethod::Passkey)
        .await
        .unwrap();

    let updated = sign_out_other_devices(&env, &Caller::browser(user.clone()), &here)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.session_version, user.session_version + 1);
    assert_eq!(
        load_device(&app, user.id, &here)
            .await
            .unwrap()
            .map(|u| u.session_version),
        Some(updated.session_version)
    );
    assert!(load_device(&app, user.id, &phone).await.unwrap().is_none());
    assert!(load_device(&app, user.id, &tablet).await.unwrap().is_none());
    let detail = sqlx::query_scalar!(
        "SELECT detail FROM account_events WHERE user_id = $1 AND kind = 'other_devices_signed_out'",
        user.id
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(detail, json!({ "devices": 2, "apps_disconnected": 0 }));
}

#[sqlx::test]
async fn a_session_version_bump_hides_devices_until_one_is_kept(db: PgPool) {
    let user = active_user("bumped@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let here = start_device(&env, &user, CHROME, SignInMethod::Password)
        .await
        .unwrap();
    start_device(&env, &user, FIREFOX, SignInMethod::Password)
        .await
        .unwrap();

    let bumped = query_bump_session_version(user.id, &db).await.unwrap();
    let caller = Caller::browser(std::sync::Arc::new(bumped));
    assert!(
        list_devices(&env, &caller, Some(&here))
            .await
            .unwrap()
            .unwrap()
            .is_empty()
    );

    assert!(keep_device(&app, &here, &caller.user).await.unwrap());
    let listed = list_devices(&env, &caller, Some(&here))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        listed.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
        [here.as_str()]
    );
}

#[sqlx::test]
async fn api_tokens_cannot_see_or_sign_out_devices(db: PgPool) {
    let user = active_user("token@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let token = Caller::api_token(user.clone(), 1, ApiScope::ALL.to_vec());

    assert_eq!(
        list_devices(&env, &token, None).await.unwrap(),
        Err(ActionError::Denied(Denied::BrowserOnly))
    );
    assert_eq!(
        sign_out_device(&env, &token, &SignOutDevice { id: "dev_x".into() }, None)
            .await
            .unwrap(),
        Err(ActionError::Denied(Denied::BrowserOnly))
    );
    assert_eq!(
        sign_out_other_devices(&env, &token, "dev_x")
            .await
            .unwrap()
            .map(|_| ()),
        Err(ActionError::Denied(Denied::BrowserOnly))
    );
    assert!(event_kinds(user.id, &db).await.is_empty());
}
