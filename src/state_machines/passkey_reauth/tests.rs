use axum::http::StatusCode;
use sqlx::PgPool;
use time::Duration;

use super::*;
use crate::auth::reauth::{PASSKEY_REAUTH as PROOF_KEY, grant_passkey_reauth};
use crate::env::App;
use crate::routes::LOGOUT_PATH;
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::user::testing::{TEST_PASSWORD, active_user};

const OTHER_PASSWORD: &str = "another-strong-password";
const CHANGE_EMAIL: &str = "/account/email/change";
const WINDOW_SECONDS: i64 = 5 * 60;

struct Holder {
    db: PgPool,
    browser: Browser,
    user: User,
    other: User,
    password: &'static str,
    changes: usize,
}

#[sqlx::test]
async fn every_passkey_reauth_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in PASSKEY_REAUTH.states.iter().enumerate() {
        for (column, &event) in PASSKEY_REAUTH.events.iter().enumerate() {
            let cell = (row * 16 + column) as u8;
            let user = active_user(&format!("proof-{cell}@example.test"), &db).await;
            let other = active_user(&format!("proof-other-{cell}@example.test"), &db).await;
            let mut holder = Holder {
                db: db.clone(),
                browser: Browser::new(&app, &assets, [198, 51, 109, cell]),
                user: fetch(user.id, &db).await,
                other: fetch(other.id, &db).await,
                password: TEST_PASSWORD,
                changes: 0,
            };
            assert!(
                holder
                    .browser
                    .sign_in(&holder.user.email, TEST_PASSWORD)
                    .await
            );
            holder.arrange(from).await;
            let (actual, broken) = holder.run(from, event).await;
            cells.push(CellResult {
                from,
                event,
                expected: expect(from, event),
                actual: Some(actual),
                accepted: true,
                broken,
            });
        }
    }

    write_results(&PASSKEY_REAUTH, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Holder {
    async fn arrange(&mut self, from: ProofState) {
        if from != S::None {
            self.confirm().await;
        }
        if from == S::Stale {
            self.browser.minutes += 6;
        }
        assert_eq!(self.state().await, from, "arranging {}", self.user.email);
    }

    async fn run(
        &mut self,
        from: ProofState,
        event: ProofEvent,
    ) -> (ProofState, Vec<&'static str>) {
        let mut broken = Vec::new();
        match event {
            E::ConfirmWithPasskey => self.confirm().await,
            E::UseIt => {
                let skipped_password = self.change_email_without_password().await;
                if skipped_password != (from == S::Fresh) {
                    broken.push(ONLY_FRESH_SKIPS_THE_PASSWORD);
                }
                if skipped_password && self.change_email_without_password().await {
                    broken.push(ONE_ACTION_PER_PROOF);
                }
            }
            E::UseItAndFail => {
                let email = self.user.email.clone();
                self.post_email_change(&email).await;
            }
            E::WaitSixMinutes => self.browser.minutes += 6,
            E::ChangePassword => self.change_password().await,
            E::RemoveAPasskey => self.remove_a_passkey().await,
            E::SignOutAndIn => {
                self.browser.post(LOGOUT_PATH, "").await;
                let email = self.user.email.clone();
                assert!(self.browser.sign_in(&email, self.password).await);
            }
            E::SignInAsSomeoneElse => {
                let email = self.other.email.clone();
                assert!(self.browser.sign_in(&email, TEST_PASSWORD).await);
                std::mem::swap(&mut self.user, &mut self.other);
                self.password = TEST_PASSWORD;
            }
        }
        (self.state().await, broken)
    }

    async fn confirm(&mut self) {
        let session = self.browser.session().expect("signed in");
        let user = fetch(self.user.id, &self.db).await;
        grant_passkey_reauth(&session, &user, self.browser.now())
            .await
            .unwrap();
        session.save().await.unwrap();
    }

    async fn change_email_without_password(&mut self) -> bool {
        self.changes += 1;
        let address = format!("moved-{}-{}", self.changes, self.user.email);
        let response = self.post_email_change(&address).await;
        response == StatusCode::SEE_OTHER
    }

    async fn post_email_change(&mut self, address: &str) -> StatusCode {
        let form = format!(
            "new_email={}&current_password=",
            address.replace('@', "%40")
        );
        let response = self.browser.post(CHANGE_EMAIL, &form).await;
        let status = response.status();
        let redirected_to_sign_in = location(&response).as_deref() == Some("/login");
        assert!(!redirected_to_sign_in, "signed out unexpectedly");
        status
    }

    async fn change_password(&mut self) {
        let next = if self.password == TEST_PASSWORD {
            OTHER_PASSWORD
        } else {
            TEST_PASSWORD
        };
        let form = format!(
            "current_password={}&password={next}&confirm_password={next}",
            self.password
        );
        let response = self.browser.post("/account/password/change", &form).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        self.password = next;
    }

    async fn remove_a_passkey(&mut self) {
        let id = sqlx::query_scalar!(
            r#"INSERT INTO user_passkeys (user_id, credential_id, passkey, name, backup_eligible, backed_up, public_id)
            VALUES ($1, convert_to(random_base62(16), 'UTF8'), '{}', 'Spare key', false, false, 'pk_' || random_base62(20))
            RETURNING public_id"#,
            self.user.id,
        )
        .fetch_one(&self.db)
        .await
        .unwrap();
        let response = self
            .browser
            .post(
                "/account/passkeys/remove",
                &format!("id={id}&current_password={}", self.password),
            )
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }

    async fn state(&self) -> ProofState {
        let Some(grant) = self.browser.session_value(PROOF_KEY).await else {
            return S::None;
        };
        let user = fetch(self.user.id, &self.db).await;
        let age = self.browser.now().unix_timestamp() - grant["confirmed_at"].as_i64().unwrap();
        let fresh = grant["user_id"].as_i64() == Some(i64::from(user.id))
            && grant["session_version"].as_i64() == Some(i64::from(user.session_version))
            && (0..=WINDOW_SECONDS).contains(&age);
        if fresh { S::Fresh } else { S::Stale }
    }
}

async fn fetch(user_id: i32, db: &PgPool) -> User {
    query_get_user_by_id(user_id, db).await.unwrap().unwrap()
}

#[test]
fn the_window_matches_the_limit() {
    assert_eq!(Duration::seconds(WINDOW_SECONDS), Duration::minutes(5));
}
