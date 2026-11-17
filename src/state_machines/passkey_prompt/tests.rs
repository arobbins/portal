use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::auth::links::TokenPurpose;
use crate::env::App;
use crate::secret::{hash_secret, random_base62};
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const PROMPT_PAGE: &str = "/passkey-setup";
const OTHER_PASSWORD: &str = "another-strong-password";

struct Person {
    app: App,
    assets: AssetBundle,
    browser: Browser,
    user_id: i32,
    email: String,
    password: &'static str,
    cell: u8,
    sign_ins: u8,
    reset_landing: Option<String>,
}

#[sqlx::test]
async fn every_passkey_prompt_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in PASSKEY_PROMPT.states.iter().enumerate() {
        for (column, &event) in PASSKEY_PROMPT.events.iter().enumerate() {
            let expected = expect(from, event);
            if let Expect::Unreachable(_) = expected {
                cells.push(CellResult {
                    from,
                    event,
                    expected,
                    actual: None,
                    accepted: false,
                    broken: Vec::new(),
                });
                continue;
            }
            let cell = (row * 16 + column) as u8;
            let mut person = Person::arrange(&app, &assets, cell, from).await;
            let events_before = person.prompt_events().await;
            person.apply(event).await;
            let actual = person.state().await;
            let broken = person
                .broken_rules(from, event, actual, events_before)
                .await;
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(actual),
                accepted: true,
                broken,
            });
        }
    }

    write_results(&PASSKEY_PROMPT, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Person {
    async fn arrange(app: &App, assets: &AssetBundle, cell: u8, from: PromptState) -> Self {
        let email = format!("prompted-{cell}@example.test");
        let user = active_user(&email, &app.db).await;
        let mut person = Self {
            app: app.clone(),
            assets: assets.clone(),
            browser: Browser::new(app, assets, [198, 51, 110, cell]),
            user_id: user.id,
            email,
            password: TEST_PASSWORD,
            cell,
            sign_ins: 0,
            reset_landing: None,
        };
        person.sign_in().await;
        match from {
            S::Due => {}
            S::Snoozed => person.apply(E::PickNotNow).await,
            S::Declined => person.apply(E::PickNeverAsk).await,
            S::HasPasskey => person.apply(E::AddPasskey).await,
        }
        assert_eq!(person.state().await, from, "arranging {}", person.email);
        person
    }

    async fn apply(&mut self, event: PromptEvent) {
        match event {
            E::SignIn => {
                self.sign_in().await;
            }
            E::PickNotNow => {
                self.browser.post("/passkey-setup/snooze", "").await;
            }
            E::PickNeverAsk => {
                self.browser.post("/passkey-setup/decline", "").await;
            }
            E::ThirtyDaysPass => {
                sqlx::query!(
                    "UPDATE users SET passkey_prompt_after = now() - interval '1 second' WHERE id = $1 AND passkey_prompt_after > now() AND passkey_prompt_after <> 'infinity'",
                    self.user_id
                )
                .execute(&self.app.db)
                .await
                .unwrap();
            }
            E::AddPasskey => {
                sqlx::query!(
                    r#"INSERT INTO user_passkeys (user_id, credential_id, passkey, name, backup_eligible, backed_up, public_id)
                    VALUES ($1, convert_to(random_base62(16), 'UTF8'), '{}', 'Laptop', false, false, 'pk_' || random_base62(20))"#,
                    self.user_id,
                )
                .execute(&self.app.db)
                .await
                .unwrap();
            }
            E::RemovePasskey => self.remove_passkey().await,
            E::ResetPassword => self.reset_password().await,
        }
    }

    async fn broken_rules(
        &mut self,
        from: PromptState,
        event: PromptEvent,
        state: PromptState,
        events_before: i64,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        let page_opens = self.browser.get(PROMPT_PAGE).await.status() == StatusCode::OK;
        if page_opens && state != S::Due {
            broken.push(PAGE_ONLY_WHEN_DUE);
        }
        let prompted = self.sign_in().await == PROMPT_PAGE;
        if prompted != (state == S::Due) {
            broken.push(PROMPT_ONLY_WHEN_DUE);
        }
        if event == E::ResetPassword && self.reset_landing.as_deref() == Some(PROMPT_PAGE) {
            broken.push(RESETS_NEVER_PROMPT);
        }
        let answered = matches!(event, E::PickNotNow | E::PickNeverAsk);
        let recorded = self.prompt_events().await - events_before;
        let counted = state != from;
        if answered && recorded != i64::from(counted) {
            broken.push(ANSWERS_ARE_RECORDED);
        }
        broken
    }

    async fn sign_in(&mut self) -> String {
        self.sign_ins += 1;
        let ip = [198, 51, 111 + self.sign_ins, self.cell];
        self.browser = Browser::new(&self.app, &self.assets, ip);
        let form = format!(
            "email={}&password={}",
            self.email.replace('@', "%40"),
            self.password
        );
        let response = self.browser.post("/login", &form).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        location(&response).unwrap_or_default()
    }

    async fn remove_passkey(&mut self) {
        let id = sqlx::query_scalar!(
            "SELECT public_id FROM user_passkeys WHERE user_id = $1",
            self.user_id
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        let form = format!("id={id}&current_password={}", self.password);
        let response = self.browser.post("/account/passkeys/remove", &form).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }

    async fn reset_password(&mut self) {
        let raw = random_base62(32);
        sqlx::query!(
            "INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
            self.user_id,
            TokenPurpose::PasswordReset.as_str(),
            hash_secret(&raw),
        )
        .execute(&self.app.db)
        .await
        .unwrap();
        let next = if self.password == TEST_PASSWORD {
            OTHER_PASSWORD
        } else {
            TEST_PASSWORD
        };
        let form = format!("password={next}&confirm_password={next}");
        let path = TokenPurpose::PasswordReset.path(&raw);
        let response = self.browser.post(&path, &form).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        self.reset_landing = location(&response);
        self.password = next;
    }

    async fn prompt_events(&self) -> i64 {
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM account_events WHERE user_id = $1 AND kind IN ('passkey_prompt_snoozed', 'passkey_prompt_declined')"#,
            self.user_id
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }

    async fn state(&self) -> PromptState {
        let row = sqlx::query!(
            r#"SELECT
                EXISTS (SELECT 1 FROM user_passkeys WHERE user_id = $1) AS "has_passkey!",
                COALESCE(passkey_prompt_after = 'infinity', false) AS "declined!",
                COALESCE(passkey_prompt_after > now(), false) AS "snoozed!"
            FROM users WHERE id = $1"#,
            self.user_id
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        match (row.has_passkey, row.declined, row.snoozed) {
            (true, _, _) => S::HasPasskey,
            (false, true, _) => S::Declined,
            (false, false, true) => S::Snoozed,
            (false, false, false) => S::Due,
        }
    }
}
