use std::sync::Arc;

use axum::http::StatusCode;
use sqlx::PgPool;
use time::Duration;
use topcoat::asset::AssetBundle;

use super::*;
use crate::action::Caller;
use crate::admin::actions::{TargetUser, disable_user, sign_out_user};
use crate::auth::actions::{NewPassword, confirm_email_change, set_password_from_link};
use crate::auth::links::TokenPurpose;
use crate::devices::lifecycle::delete_stale_devices;
use crate::env::{App, Clock};
use crate::secret::{hash_secret, random_base62};
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::user::testing::{TEST_PASSWORD, active_user};

const ACCOUNT_PAGE: &str = "/account";
const NEW_PASSWORD: &str = "another-strong-password";

struct Pair {
    app: App,
    admin: Caller,
    user: Arc<User>,
    here: Browser,
    other: Browser,
    other_device: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Snapshot {
    devices: i64,
    sign_ins: i64,
}

#[sqlx::test]
async fn every_signed_in_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let admin = admin(&db).await;
    let mut cells = Vec::new();
    for (row, &from) in SIGNED_IN.states.iter().enumerate() {
        for (column, &event) in SIGNED_IN.events.iter().enumerate() {
            let expected = expect(from, event);
            let mut pair = Pair::arrange(&app, &assets, &admin, [row, column], from).await;
            let before = pair.snapshot().await;
            let old_cookie = pair.here.cookie.clone();
            let accepted = pair.apply(event).await;
            let state = signed_in(&mut pair.here).await;
            let broken = pair
                .broken_rules(event, accepted, state, &before, old_cookie)
                .await;
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(state),
                accepted,
                broken,
            });
        }
    }

    write_results(&SIGNED_IN, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Pair {
    async fn arrange(
        app: &App,
        assets: &AssetBundle,
        admin: &Caller,
        [row, column]: [usize; 2],
        from: SignedInState,
    ) -> Self {
        let cell = (row * 16 + column) as u8;
        let email = format!("signed-in-{row}-{column}@example.test");
        let user = active_user(&email, &app.db).await;
        let mut other = Browser::new(app, assets, [198, 51, 102, cell]);
        assert!(other.sign_in(&email, TEST_PASSWORD).await);
        let other_device = devices_of(user.id, &app.db).await.remove(0);
        let mut here = Browser::new(app, assets, [198, 51, 101, cell]);
        if from == S::SignedIn {
            assert!(here.sign_in(&email, TEST_PASSWORD).await);
        }
        assert_eq!(signed_in(&mut here).await, from, "arranging {email}");
        Self {
            app: app.clone(),
            admin: admin.clone(),
            user,
            here,
            other,
            other_device,
        }
    }

    async fn apply(&mut self, event: SignedInEvent) -> bool {
        let email = self.user.email.clone();
        match event {
            E::SignIn => self.here.sign_in(&email, TEST_PASSWORD).await,
            E::SignInWrongPassword => self.here.sign_in(&email, "not-the-password").await,
            E::SignOut => redirected(self.here.post(crate::routes::LOGOUT_PATH, "").await),
            E::ChangePassword => change_password(&mut self.here).await,
            E::OtherChangesPassword => change_password(&mut self.other).await,
            E::OtherSignsThisOut => self.sign_out_this_device().await,
            E::OtherSignsOutOthers => {
                let response = self
                    .other
                    .post("/account/devices/sign-out-others", "")
                    .await;
                redirected(response)
            }
            E::PasswordReset => self.reset_password().await,
            E::EmailChangeConfirmed => self.confirm_new_email().await,
            E::AdminSignsOutEverywhere => {
                let target = self.target();
                sign_out_user(&self.env(), &self.admin, &target)
                    .await
                    .unwrap()
                    .is_ok()
            }
            E::AdminDisables => {
                let target = self.target();
                disable_user(&self.env(), &self.admin, &target)
                    .await
                    .unwrap()
                    .is_ok()
            }
            E::IdleADay => self.idle_a_day().await,
        }
    }

    async fn broken_rules(
        &mut self,
        event: SignedInEvent,
        accepted: bool,
        state: SignedInState,
        before: &Snapshot,
        old_cookie: Option<String>,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        let other_state = signed_in(&mut self.other).await;
        let after = self.snapshot().await;
        let browsers = [state, other_state]
            .iter()
            .filter(|state| **state == S::SignedIn)
            .count() as i64;
        if after.devices != browsers {
            broken.push(ONE_ROW_PER_BROWSER);
        }
        if state == S::SignedOut && old_cookie.is_some() {
            let current = std::mem::replace(&mut self.here.cookie, old_cookie);
            if signed_in(&mut self.here).await == S::SignedIn {
                broken.push(OLD_COOKIE_IS_DEAD);
            }
            self.here.cookie = current;
        }
        if (other_state == S::SignedIn) != (!accepted || other_device_stays(event)) {
            broken.push(OTHER_DEVICE_ONLY_WHEN_MEANT);
        }
        let signed_in_now = i64::from(event == E::SignIn && accepted);
        if after.sign_ins - before.sign_ins != signed_in_now {
            broken.push(SIGN_INS_ARE_RECORDED);
        }
        broken
    }

    async fn sign_out_this_device(&mut self) -> bool {
        let devices = devices_of(self.user.id, &self.app.db).await;
        let Some(this) = devices.iter().find(|id| **id != self.other_device) else {
            return false;
        };
        let response = self
            .other
            .post("/account/devices/sign-out", &format!("id={this}"))
            .await;
        redirected(response)
    }

    async fn reset_password(&self) -> bool {
        let raw = plant_link(
            self.user.id,
            TokenPurpose::PasswordReset,
            None,
            &self.app.db,
        )
        .await;
        let input = NewPassword {
            password: NEW_PASSWORD.to_string(),
            confirm_password: NEW_PASSWORD.to_string(),
            code: String::new(),
        };
        set_password_from_link(&self.env(), TokenPurpose::PasswordReset, &raw, &input)
            .await
            .unwrap()
            .is_ok()
    }

    async fn confirm_new_email(&self) -> bool {
        let new_email = format!("moved-{}", self.user.email);
        let purpose = TokenPurpose::EmailChange;
        let raw = plant_link(self.user.id, purpose, Some(&new_email), &self.app.db).await;
        confirm_email_change(&self.env(), &raw)
            .await
            .unwrap()
            .is_ok()
    }

    async fn idle_a_day(&mut self) -> bool {
        let day = Duration::days(1).whole_minutes() + 60;
        self.here.minutes += day;
        self.other.minutes += day;
        let later = App {
            clock: Clock::Fixed(self.here.now()),
            ..self.app.clone()
        };
        delete_stale_devices(&later).await.unwrap();
        true
    }

    fn env(&self) -> crate::env::Env<'_> {
        self.app.env(None)
    }

    fn target(&self) -> TargetUser {
        TargetUser {
            id: self.user.public_id.clone(),
        }
    }

    async fn snapshot(&self) -> Snapshot {
        sqlx::query_as!(
            Snapshot,
            r#"SELECT
                (SELECT count(*) FROM user_devices d JOIN users u ON u.id = d.user_id WHERE d.user_id = $1 AND d.session_version = u.session_version) AS "devices!",
                (SELECT count(*) FROM account_events WHERE user_id = $1 AND kind = 'signed_in') AS "sign_ins!""#,
            self.user.id,
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }
}

async fn signed_in(browser: &mut Browser) -> SignedInState {
    let response = browser.get(ACCOUNT_PAGE).await;
    match (response.status(), location(&response).as_deref()) {
        (StatusCode::OK, _) => S::SignedIn,
        (StatusCode::SEE_OTHER, Some("/login")) => S::SignedOut,
        (status, location) => panic!("unexpected {status} to {location:?} from {ACCOUNT_PAGE}"),
    }
}

async fn change_password(browser: &mut Browser) -> bool {
    let form = format!(
        "current_password={TEST_PASSWORD}&password={NEW_PASSWORD}&confirm_password={NEW_PASSWORD}"
    );
    let response = browser.post("/account/password/change", &form).await;
    response.status() == StatusCode::SEE_OTHER && location(&response).as_deref() != Some("/login")
}

fn redirected(response: axum::http::Response<axum::body::Body>) -> bool {
    response.status() == StatusCode::SEE_OTHER
}

async fn devices_of(user_id: i32, db: &PgPool) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT public_id FROM user_devices WHERE user_id = $1 ORDER BY id",
        user_id
    )
    .fetch_all(db)
    .await
    .unwrap()
}

async fn plant_link(
    user_id: i32,
    purpose: TokenPurpose,
    payload: Option<&str>,
    db: &PgPool,
) -> String {
    let raw = random_base62(32);
    sqlx::query!(
        "INSERT INTO user_tokens (user_id, purpose, token_hash, payload, expires_at) VALUES ($1, $2, $3, $4, now() + interval '1 hour')",
        user_id,
        purpose.as_str(),
        hash_secret(&raw),
        payload,
    )
    .execute(db)
    .await
    .unwrap();
    raw
}

async fn admin(db: &PgPool) -> Caller {
    let user = active_user("signed-in-admin@example.test", db).await;
    sqlx::query!(
        "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
        user.id
    )
    .execute(db)
    .await
    .unwrap();
    let user = query_get_user_by_id(user.id, db).await.unwrap().unwrap();
    Caller::browser(Arc::new(user))
}
