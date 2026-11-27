use std::io::{Cursor, Read};

use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, Denied};
use crate::api::scope::ApiScope;
use crate::api_tokens::testing::issue_token;
use crate::auth::password::CurrentPasswordProblem;
use crate::env::{App, Clock};
use crate::oauth::testing::{connect_named_app, connect_test_app};
use crate::rate_limit::EXPORTS_PER_ACCOUNT_PER_HOUR;
use crate::user::testing::{TEST_PASSWORD, active_user};

fn with_password(password: &str) -> ExportData {
    ExportData {
        current_password: password.to_string(),
    }
}

async fn recorded(user_id: i32, db: &PgPool) -> (i64, i64) {
    let events = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM account_events WHERE user_id = $1 AND kind = 'data_exported'"#,
        user_id
    )
    .fetch_one(db)
    .await
    .unwrap();
    let emails = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM jobs WHERE kind = 'send_email' AND payload->>'subject' = 'Your account data was downloaded'"#
    )
    .fetch_one(db)
    .await
    .unwrap();
    (events, emails)
}

fn files_in(zip: &[u8]) -> Vec<(String, String)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            (file.name().to_string(), contents)
        })
        .collect()
}

#[sqlx::test]
async fn a_confirmed_export_is_recorded_and_emailed(db: PgPool) {
    let user = active_user("keeper@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let caller = Caller::browser(user.clone());

    let export = export_data(&app.env(None), &caller, &with_password(TEST_PASSWORD), None)
        .await
        .unwrap()
        .unwrap();

    let files = files_in(&export.zip);
    assert_eq!(files.len(), 11);
    let profile = &files
        .iter()
        .find(|(name, _)| name == "profile.csv")
        .unwrap()
        .1;
    assert!(profile.contains("keeper@example.test"));
    let readme = &files
        .iter()
        .find(|(name, _)| name == "README.txt")
        .unwrap()
        .1;
    for explained in [
        "connected_apps.csv",
        "returns_to, scopes",
        "billing.csv",
        "renews_on, ends_on, stripe_customer_id",
    ] {
        assert!(readme.contains(explained), "{explained}");
    }
    assert!(
        files
            .iter()
            .all(|(_, contents)| !contents.contains("$argon2"))
    );
    assert!(export.file_name.starts_with("webserver-data-"));
    assert_eq!(recorded(user.id, &db).await, (1, 1));
}

#[sqlx::test]
async fn an_api_token_can_never_export(db: PgPool) {
    let user = active_user("token@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let caller = Caller::api_token(user.clone(), 1, ApiScope::ALL.to_vec());

    let outcome = export_data(&app.env(None), &caller, &with_password(TEST_PASSWORD), None)
        .await
        .unwrap();

    assert_eq!(outcome, Err(ActionError::Denied(Denied::BrowserOnly)));
    assert_eq!(recorded(user.id, &db).await, (0, 0));
}

#[sqlx::test]
async fn a_wrong_password_exports_nothing(db: PgPool) {
    let user = active_user("guess@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let caller = Caller::browser(user.clone());

    let outcome = export_data(
        &app.env(None),
        &caller,
        &with_password("nope-nope-nope"),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        outcome,
        Err(ActionError::Problem(ExportProblem::Reauth(
            ReauthProblem::Password(CurrentPasswordProblem::Wrong)
        )))
    );
    assert_eq!(recorded(user.id, &db).await, (0, 0));
}

#[sqlx::test]
async fn an_account_exports_at_most_five_times_an_hour(db: PgPool) {
    let user = active_user("often@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let caller = Caller::browser(user.clone());
    let input = with_password(TEST_PASSWORD);

    for _ in 0..EXPORTS_PER_ACCOUNT_PER_HOUR {
        assert!(
            export_data(&env, &caller, &input, None)
                .await
                .unwrap()
                .is_ok()
        );
    }
    let outcome = export_data(&env, &caller, &input, None).await.unwrap();

    assert_eq!(
        outcome,
        Err(ActionError::Problem(ExportProblem::TooManyAttempts))
    );
    let allowed = i64::from(EXPORTS_PER_ACCOUNT_PER_HOUR);
    assert_eq!(recorded(user.id, &db).await, (allowed, allowed));
}

#[sqlx::test]
async fn no_secret_ever_reaches_the_export(db: PgPool) {
    let user = active_user("secrets@example.test", &db).await;
    let token = issue_token(user.id, &["profile:read"], time::Duration::days(30), &db).await;
    let token_hash =
        sqlx::query_scalar!("SELECT token_hash FROM api_tokens WHERE id = $1", token.id)
            .fetch_one(&db)
            .await
            .unwrap();
    let two_factor_secret = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";
    sqlx::query!(
        "INSERT INTO user_two_factor (user_id, secret) VALUES ($1, $2)",
        user.id,
        two_factor_secret
    )
    .execute(&db)
    .await
    .unwrap();
    let app = App::for_tests(db.clone());
    let (_, app_tokens) = connect_test_app(&app, &user, &[ApiScope::ProfileRead]).await;
    let app_token_hashes = sqlx::query_scalar!("SELECT token_hash FROM oauth_tokens")
        .fetch_all(&db)
        .await
        .unwrap();
    assert!(!app_token_hashes.is_empty());
    let caller = Caller::browser(user.clone());

    let export = export_data(&app.env(None), &caller, &with_password(TEST_PASSWORD), None)
        .await
        .unwrap()
        .unwrap();

    for (name, contents) in files_in(&export.zip) {
        let app_secrets = app_token_hashes.iter().map(String::as_str);
        for secret in [
            token.raw.as_str(),
            token_hash.as_str(),
            two_factor_secret,
            "$argon2",
            app_tokens.access_token.as_str(),
            app_tokens.refresh_token.as_str(),
        ]
        .into_iter()
        .chain(app_secrets)
        {
            assert!(!contents.contains(secret), "{name} contains a secret");
        }
    }
}

#[sqlx::test]
async fn every_file_has_its_header_and_dates_are_utc(db: PgPool) {
    let user = active_user("empty@example.test", &db).await;
    let mut app = App::for_tests(db.clone());
    let late_evening = time::OffsetDateTime::from_unix_timestamp(1_795_044_600)
        .unwrap()
        .to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap());
    app.clock = Clock::Fixed(late_evening);
    let caller = Caller::browser(user.clone());

    let export = export_data(&app.env(None), &caller, &with_password(TEST_PASSWORD), None)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(export.file_name, "webserver-data-2026-11-18.zip");
    for (name, contents) in files_in(&export.zip) {
        if name.ends_with(".csv") {
            assert!(
                !contents.lines().next().unwrap_or_default().is_empty(),
                "{name}"
            );
        }
    }
    let passkeys = files_in(&export.zip)
        .into_iter()
        .find(|(name, _)| name == "passkeys.csv")
        .unwrap()
        .1;
    assert_eq!(passkeys.trim(), "name,added_at,last_used_at,backed_up");
    let profile = files_in(&export.zip)
        .into_iter()
        .find(|(name, _)| name == "profile.csv")
        .unwrap()
        .1;
    assert!(profile.contains('Z'), "{profile}");
}

#[sqlx::test]
async fn a_connected_app_named_like_a_formula_is_exported_as_text(db: PgPool) {
    let user = active_user("formula@example.test", &db).await;
    let app = App::for_tests(db.clone());
    connect_named_app(&app, &user, "=HYPERLINK(\"x\")", &[ApiScope::ProfileRead]).await;
    let caller = Caller::browser(user.clone());

    let export = export_data(&app.env(None), &caller, &with_password(TEST_PASSWORD), None)
        .await
        .unwrap()
        .unwrap();

    let files = files_in(&export.zip);
    let connected = &files
        .iter()
        .find(|(name, _)| name == "connected_apps.csv")
        .unwrap()
        .1;
    let row = connected.lines().nth(1).unwrap();
    assert!(
        row.starts_with("\"'=HYPERLINK(\"\"x\"\")\",claude.ai,profile:read,"),
        "{row}"
    );
}
