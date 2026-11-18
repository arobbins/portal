use axum::http::StatusCode;
use sqlx::PgPool;
use time::Duration;
use topcoat::asset::AssetBundle;

use super::*;
use crate::action::Caller;
use crate::devices::known::{KNOWN_DEVICE_COOKIE, forget_unseen_devices};
use crate::env::{App, Clock};
use crate::preferences::actions::{UpdateNotifications, update_notifications};
use crate::routes::{LOGIN_PATH, LOGOUT_PATH};
use crate::secret::hash_secret;
use crate::state_machines::testing::{Browser, assets, location, reset_link, subjects_to};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::testing::{TEST_PASSWORD, active_user};

const ALERT: &str = "New sign-in to your account";
const NEW_PASSWORD: &str = "another-strong-password";

struct Visitor {
    app: App,
    browser: Browser,
    user: std::sync::Arc<User>,
    password: &'static str,
}

#[sqlx::test]
async fn every_known_device_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in KNOWN_DEVICE.states.iter().enumerate() {
        for (column, &event) in KNOWN_DEVICE.events.iter().enumerate() {
            let (actual, accepted, broken) =
                run_cell(&app, &assets, [row, column], from, event).await;
            cells.push(CellResult {
                from,
                event,
                expected: expect(from, event),
                actual: Some(actual),
                accepted,
                broken,
            });
        }
    }

    write_results(&KNOWN_DEVICE, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn run_cell(
    app: &App,
    assets: &AssetBundle,
    [row, column]: [usize; 2],
    from: KnownState,
    event: KnownEvent,
) -> (KnownState, bool, Vec<&'static str>) {
    let email = format!("known-{row}-{column}@example.test");
    let mut visitor = Visitor {
        app: app.clone(),
        browser: Browser::new(app, assets, [198, 51, 170, (row * 16 + column) as u8]),
        user: active_user(&email, &app.db).await,
        password: TEST_PASSWORD,
    };
    if from == S::Known {
        assert!(visitor.sign_in().await, "arranging {email}");
        visitor.browser.post(LOGOUT_PATH, "").await;
    }
    assert_eq!(visitor.look().await, from, "arranging {email}");
    let alerts_before = visitor.alerts().await;

    let accepted = visitor.apply(event).await;
    let seen = visitor.look().await;
    let mut broken = Vec::new();
    let should_alert = from == S::Unknown && event == E::SignIn && accepted;
    let new_alerts = visitor.alerts().await - alerts_before;
    if new_alerts != usize::from(should_alert) {
        broken.push(ALERT_ONLY_WHEN_NEW);
    }
    if seen == S::Known && visitor.raw_cookie_stored().await {
        broken.push(ONLY_THE_HASH);
    }
    (seen, accepted, broken)
}

impl Visitor {
    async fn apply(&mut self, event: KnownEvent) -> bool {
        match event {
            E::SignIn => self.sign_in().await,
            E::SignInWithAlertsOff => {
                let off = UpdateNotifications {
                    sign_in_alerts: false,
                };
                update_notifications(
                    &self.app.env(None),
                    &Caller::browser(self.user.clone()),
                    &off,
                )
                .await
                .unwrap()
                .unwrap();
                self.sign_in().await
            }
            E::ResetFromEmail => self.reset_from_email().await,
            E::ClearCookies => {
                self.browser.forget_cookie(KNOWN_DEVICE_COOKIE);
                true
            }
            E::UnusedFor400Days => {
                let later = App {
                    clock: Clock::Fixed(self.browser.now() + Duration::days(UNUSED_DAYS)),
                    ..self.app.clone()
                };
                forget_unseen_devices(&later).await.is_ok()
            }
        }
    }

    async fn sign_in(&mut self) -> bool {
        let password = self.password;
        self.browser.sign_in(&self.user.email, password).await
    }

    async fn reset_from_email(&mut self) -> bool {
        let Some(link) = reset_link(&self.app, &self.user.email).await else {
            return false;
        };
        let form = format!("password={NEW_PASSWORD}&confirm_password={NEW_PASSWORD}");
        let response = self.browser.post(&link, &form).await;
        self.password = NEW_PASSWORD;
        response.status() == StatusCode::SEE_OTHER
            && location(&response).as_deref() != Some(LOGIN_PATH)
    }

    async fn look(&self) -> KnownState {
        let Some(token) = self.browser.cookie_value(KNOWN_DEVICE_COOKIE) else {
            return S::Unknown;
        };
        let known = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM known_devices WHERE user_id = $1 AND token_hash = $2) AS "known!""#,
            self.user.id,
            hash_secret(&token),
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        if known { S::Known } else { S::Unknown }
    }

    async fn alerts(&self) -> usize {
        subjects_to(&self.user.email, &self.app.db)
            .await
            .iter()
            .filter(|subject| *subject == ALERT)
            .count()
    }

    async fn raw_cookie_stored(&self) -> bool {
        let token = self
            .browser
            .cookie_value(KNOWN_DEVICE_COOKIE)
            .unwrap_or_default();
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM known_devices WHERE token_hash = $1) AS "stored!""#,
            token
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }
}
