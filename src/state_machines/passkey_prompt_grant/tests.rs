use axum::http::StatusCode;
use sqlx::PgPool;

use super::*;
use crate::auth::links::TokenPurpose;
use crate::env::App;
use crate::passkeys::prompt_grant::{PASSKEY_PROMPT_GRANT as GRANT_KEY, PASSKEY_PROMPT_WINDOW};
use crate::routes::LOGOUT_PATH;
use crate::secret::{hash_secret, random_base62};
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const PROMPT_PAGE: &str = "/passkey-setup";
const OTHER_PASSWORD: &str = "another-strong-password";
const WINDOW_SECONDS: i64 = 5 * 60;

struct Visitor {
    db: PgPool,
    browser: Browser,
    user_id: i32,
    email: String,
    other_email: String,
    password: &'static str,
}

#[sqlx::test]
async fn every_passkey_prompt_grant_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in PASSKEY_PROMPT_GRANT.states.iter().enumerate() {
        for (column, &event) in PASSKEY_PROMPT_GRANT.events.iter().enumerate() {
            let cell = (row * 16 + column) as u8;
            let email = format!("granted-{cell}@example.test");
            let other_email = format!("granted-other-{cell}@example.test");
            let user = active_user(&email, &db).await;
            let other = active_user(&other_email, &db).await;
            sqlx::query!(
                "UPDATE users SET passkey_prompt_after = 'infinity' WHERE id = $1",
                other.id
            )
            .execute(&db)
            .await
            .unwrap();
            let mut visitor = Visitor {
                db: db.clone(),
                browser: Browser::new(&app, &assets, [198, 51, 120, cell]),
                user_id: user.id,
                email,
                other_email,
                password: TEST_PASSWORD,
            };
            visitor.arrange(from).await;
            let started = visitor.apply(event).await;
            let actual = visitor.state().await;
            let mut broken = Vec::new();
            if event == E::StartWithoutPassword && started != (from == S::Fresh) {
                broken.push(SKIPS_PASSWORD_ONLY_WHEN_FRESH);
            }
            let opens = visitor.browser.get(PROMPT_PAGE).await.status() == StatusCode::OK;
            if opens != (actual == S::Fresh && visitor.due().await) {
                broken.push(PAGE_ONLY_WITH_A_FRESH_GRANT);
            }
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

    write_results(&PASSKEY_PROMPT_GRANT, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Visitor {
    async fn arrange(&mut self, from: GrantState) {
        match from {
            S::None => {
                self.apply(E::SignInWhenNotDue).await;
                self.set_due(true).await;
            }
            S::Fresh => {
                self.apply(E::SignInWhenDue).await;
            }
            S::Stale => {
                self.apply(E::SignInWhenDue).await;
                self.apply(E::WaitSixMinutes).await;
            }
        }
        assert_eq!(self.state().await, from, "arranging {}", self.email);
    }

    async fn apply(&mut self, event: GrantEvent) -> bool {
        match event {
            E::SignInWhenDue => {
                self.set_due(true).await;
                self.sign_in().await
            }
            E::SignInWhenNotDue => {
                self.set_due(false).await;
                self.sign_in().await
            }
            E::PickNotNow => redirected(self.browser.post("/passkey-setup/snooze", "").await),
            E::StartWithoutPassword => {
                let response = self
                    .browser
                    .post_json("/passkeys/register/start", "{}")
                    .await;
                response.status() == StatusCode::OK
            }
            E::WaitSixMinutes => {
                self.browser.minutes += 6;
                true
            }
            E::ResetPassword => self.reset_password().await,
            E::SignOut => redirected(self.browser.post(LOGOUT_PATH, "").await),
            E::SignInAsSomeoneElse => {
                let form = format!(
                    "email={}&password={TEST_PASSWORD}",
                    self.other_email.replace('@', "%40")
                );
                let signed_in = redirected(self.browser.post("/login", &form).await);
                let other_id =
                    sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", self.other_email)
                        .fetch_one(&self.db)
                        .await
                        .unwrap();
                self.user_id = other_id;
                signed_in
            }
        }
    }

    async fn sign_in(&mut self) -> bool {
        let form = format!(
            "email={}&password={}",
            self.email.replace('@', "%40"),
            self.password
        );
        let response = self.browser.post("/login", &form).await;
        response.status() == StatusCode::SEE_OTHER
            && location(&response).as_deref() != Some("/login")
    }

    async fn reset_password(&mut self) -> bool {
        let raw = random_base62(32);
        sqlx::query!(
            "INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
            self.user_id,
            TokenPurpose::PasswordReset.as_str(),
            hash_secret(&raw),
        )
        .execute(&self.db)
        .await
        .unwrap();
        let next = if self.password == TEST_PASSWORD {
            OTHER_PASSWORD
        } else {
            TEST_PASSWORD
        };
        let form = format!("password={next}&confirm_password={next}");
        let response = self
            .browser
            .post(&TokenPurpose::PasswordReset.path(&raw), &form)
            .await;
        self.password = next;
        redirected(response)
    }

    async fn set_due(&self, due: bool) {
        let after = if due { None } else { Some("infinity") };
        sqlx::query!(
            "UPDATE users SET passkey_prompt_after = $2::text::timestamptz WHERE id = $1",
            self.user_id,
            after,
        )
        .execute(&self.db)
        .await
        .unwrap();
    }

    async fn due(&self) -> bool {
        sqlx::query_scalar!(
            r#"SELECT (passkey_prompt_after IS NULL OR passkey_prompt_after <= now()) AS "due!" FROM users WHERE id = $1"#,
            self.user_id
        )
        .fetch_one(&self.db)
        .await
        .unwrap()
    }

    async fn state(&self) -> GrantState {
        let Some(grant) = self.browser.session_value(GRANT_KEY).await else {
            return S::None;
        };
        let age = self.browser.now().unix_timestamp() - grant["granted_at"].as_i64().unwrap();
        let fresh = grant["user_id"].as_i64() == Some(i64::from(self.user_id))
            && (0..=WINDOW_SECONDS).contains(&age);
        if fresh { S::Fresh } else { S::Stale }
    }
}

fn redirected(response: axum::http::Response<axum::body::Body>) -> bool {
    response.status() == StatusCode::SEE_OTHER
}

#[test]
fn the_window_matches_the_limit() {
    assert_eq!(WINDOW_SECONDS, PASSKEY_PROMPT_WINDOW.whole_seconds());
}
