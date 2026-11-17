use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::admin::console::{Change, change_role};
use crate::env::App;
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const USERS_PAGE: &str = "/admin/users";
const CONFIRM_PAGE: &str = "/admin/confirm";
const NEW_PASSWORD: &str = "another-strong-password";

struct Admin {
    browser: Browser,
    db: PgPool,
    last_confirm_target: Option<String>,
}

struct Seen {
    state: UnlockState,
    location: Option<String>,
}

#[sqlx::test]
async fn every_admin_unlock_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in ADMIN_UNLOCK.states.iter().enumerate() {
        for (column, &event) in ADMIN_UNLOCK.events.iter().enumerate() {
            let expected = expect(from, event);
            let cell = match expected {
                Expect::Unreachable(_) => (None, false, Vec::new()),
                _ => run_cell(&app, &assets, [row, column], from, event).await,
            };
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: cell.0,
                accepted: cell.1,
                broken: cell.2,
            });
        }
    }

    write_results(&ADMIN_UNLOCK, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn run_cell(
    app: &App,
    assets: &AssetBundle,
    [row, column]: [usize; 2],
    from: UnlockState,
    event: UnlockEvent,
) -> (Option<UnlockState>, bool, Vec<&'static str>) {
    let email = format!("unlock-{row}-{column}@example.test");
    let mut admin = Admin {
        browser: Browser::new(app, assets, [198, 51, 100, (row * 16 + column) as u8]),
        db: app.db.clone(),
        last_confirm_target: None,
    };
    admin.arrange(from, &email).await;
    let accepted = admin.apply(event, &email).await;
    let seen = admin.look().await;
    let broken = admin.broken_rules(from, event, accepted, &seen).await;
    (Some(seen.state), accepted, broken)
}

impl Admin {
    async fn arrange(&mut self, from: UnlockState, email: &str) {
        let user = active_user(email, &self.db).await;
        change_role(&self.db, Change::Grant, &user.email)
            .await
            .unwrap();
        assert!(self.browser.sign_in(email, TEST_PASSWORD).await);
        assert_eq!(self.look().await.state, S::Locked, "arranging {email}");
        if from == S::Unlocked {
            assert!(self.confirm(TEST_PASSWORD).await, "unlocking {email}");
            assert_eq!(self.look().await.state, S::Unlocked, "arranging {email}");
        }
    }

    async fn apply(&mut self, event: UnlockEvent, email: &str) -> bool {
        let db = self.db.clone();
        match event {
            E::Confirm => self.confirm(TEST_PASSWORD).await,
            E::ConfirmWrongPassword => self.confirm("not-the-password").await,
            E::OpenSoon => {
                self.browser.minutes += SOON_MINUTES;
                true
            }
            E::OpenLate => {
                self.browser.minutes += LATE_MINUTES;
                true
            }
            E::ChangePassword => self.change_password().await,
            E::SignInAgain => {
                self.browser.post(crate::routes::LOGOUT_PATH, "").await;
                self.browser.sign_in(email, TEST_PASSWORD).await
            }
            E::RoleRevoked => change_role(&db, Change::Revoke, email).await.is_ok(),
            E::RoleRevokedAndGranted => {
                change_role(&db, Change::Revoke, email).await.unwrap();
                change_role(&db, Change::Grant, email).await.is_ok()
            }
        }
    }

    async fn broken_rules(
        &mut self,
        from: UnlockState,
        event: UnlockEvent,
        accepted: bool,
        seen: &Seen,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        let confirmed_from_locked = from == S::Locked && event == E::Confirm && accepted;
        if confirmed_from_locked
            && (seen.location.is_some() || self.last_confirm_target.as_deref() != Some(USERS_PAGE))
        {
            broken.push(RETURNS_TO_PAGE);
        }
        if seen.state == S::Unlocked {
            self.browser.minutes += SOON_MINUTES;
            if self.look().await.state != S::Unlocked {
                broken.push(TIMER_SLIDES);
            }
        }
        if seen.state == S::NoAccess
            && self.browser.get(CONFIRM_PAGE).await.status() != StatusCode::NOT_FOUND
        {
            broken.push(NON_ADMINS_SEE_NOTHING);
        }
        broken
    }

    async fn look(&mut self) -> Seen {
        let response = self.browser.get(USERS_PAGE).await;
        let location = location(&response);
        let state = match (response.status(), location.as_deref()) {
            (StatusCode::OK, _) => S::Unlocked,
            (StatusCode::SEE_OTHER, Some(CONFIRM_PAGE)) => S::Locked,
            (StatusCode::NOT_FOUND, _) => S::NoAccess,
            (status, location) => panic!("unexpected {status} to {location:?} from {USERS_PAGE}"),
        };
        Seen { state, location }
    }

    async fn confirm(&mut self, password: &str) -> bool {
        let response = self
            .browser
            .post(CONFIRM_PAGE, &format!("current_password={password}"))
            .await;
        let target = location(&response);
        let unlocked =
            response.status() == StatusCode::SEE_OTHER && target.as_deref() != Some(CONFIRM_PAGE);
        self.last_confirm_target = target;
        unlocked
    }

    async fn change_password(&mut self) -> bool {
        let form = format!(
            "current_password={TEST_PASSWORD}&password={NEW_PASSWORD}&confirm_password={NEW_PASSWORD}"
        );
        let response = self.browser.post("/account/password/change", &form).await;
        response.status() == StatusCode::SEE_OTHER
    }
}
