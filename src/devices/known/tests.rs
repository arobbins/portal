use axum::http::HeaderValue;
use sqlx::PgPool;

use super::*;
use crate::action::Caller;
use crate::env::App;
use crate::preferences::actions::{UpdateNotifications, update_notifications};
use crate::user::testing::active_user;

const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0";

async fn alerts_to(email: &str, db: &PgPool) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT payload->>'text' AS "text!" FROM jobs WHERE kind = 'send_email' AND payload->>'to' = $1 AND payload->>'subject' = 'New sign-in to your account' ORDER BY id"#,
        email
    )
    .fetch_all(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn a_new_browser_gets_an_alert_and_a_known_one_does_not(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("known@example.test", &db).await;

    let token = recognize_device(&env, &user, None, SignInMethod::Password, FIREFOX)
        .await
        .unwrap();
    let alerts = alerts_to(&user.email, &db).await;
    assert_eq!(alerts.len(), 1);
    assert!(alerts[0].contains("Firefox on Linux"), "{}", alerts[0]);

    let again = recognize_device(&env, &user, Some(&token), SignInMethod::Passkey, FIREFOX)
        .await
        .unwrap();
    assert_eq!(again, token);
    assert_eq!(alerts_to(&user.email, &db).await.len(), 1);
}

#[sqlx::test]
async fn signing_in_from_an_emailed_link_marks_the_browser_known_without_an_alert(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("linked@example.test", &db).await;

    let token = recognize_device(&env, &user, None, SignInMethod::NewAccount, FIREFOX)
        .await
        .unwrap();
    recognize_device(&env, &user, Some(&token), SignInMethod::Password, FIREFOX)
        .await
        .unwrap();

    assert!(alerts_to(&user.email, &db).await.is_empty());
}

#[sqlx::test]
async fn turning_alerts_off_stops_them(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("quiet@example.test", &db).await;
    let off = UpdateNotifications {
        sign_in_alerts: false,
    };
    update_notifications(&env, &Caller::browser(user.clone()), &off)
        .await
        .unwrap()
        .unwrap();

    recognize_device(&env, &user, None, SignInMethod::Password, FIREFOX)
        .await
        .unwrap();

    assert!(alerts_to(&user.email, &db).await.is_empty());
}

#[test]
fn only_a_well_formed_cookie_is_read() {
    let token = "a".repeat(TOKEN_LENGTH);
    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!("theme=dark; known_device={token}")).unwrap(),
    );
    assert_eq!(known_device_token(&headers), Some(token.clone()));

    headers.insert(COOKIE, HeaderValue::from_static("known_device=short"));
    assert_eq!(known_device_token(&headers), None);
    assert!(known_device_cookie(&token, true).contains("HttpOnly"));
    assert!(known_device_cookie(&token, true).ends_with("; Secure"));
}
