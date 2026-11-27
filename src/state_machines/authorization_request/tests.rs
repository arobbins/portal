use axum::http::StatusCode;
use sqlx::PgPool;
use time::Duration;
use topcoat::asset::AssetBundle;

use super::*;
use crate::api::scope::ApiScope;
use crate::env::{App, Clock};
use crate::oauth::lifecycle::delete_expired;
use crate::oauth::secrets::REQUEST_IDLE_LIMIT;
use crate::oauth::testing::{REDIRECT_URI, test_client};
use crate::routes::OAUTH_AUTHORIZE_PATH;
use crate::secret::PublicIdKind;
use crate::state_machines::testing::oauth::{
    TOKEN_PATH, approve_form, code_form, deny_form, param_of, start_path,
};
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const SCOPES: &[ApiScope] = &[ApiScope::ProfileRead];

struct Owner {
    app: App,
    browser: Browser,
    client_id: String,
    request: Option<String>,
    code: Option<String>,
    answer: Option<String>,
}

#[sqlx::test]
async fn every_authorization_request_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in AUTHORIZATION_REQUEST.states.iter().enumerate() {
        for (column, &event) in AUTHORIZATION_REQUEST.events.iter().enumerate() {
            let cell = (row * 16 + column) as u8;
            let mut owner = Owner::arrange(&app, &assets, cell, from).await;
            let accepted = owner.apply(event).await;
            let actual = owner.state().await;
            let broken = owner.broken_rules(from, event, accepted, actual).await;
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

    write_results(&AUTHORIZATION_REQUEST, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Owner {
    async fn arrange(app: &App, assets: &AssetBundle, cell: u8, from: RequestState) -> Self {
        let email = format!("owner-{cell}@example.test");
        active_user(&email, &app.db).await;
        let mut owner = Self {
            app: app.clone(),
            browser: Browser::new(app, assets, [198, 51, 141, cell]),
            client_id: test_client(&app.db).await,
            request: None,
            code: None,
            answer: None,
        };
        assert!(owner.browser.sign_in(&email, TEST_PASSWORD).await);
        let steps: &[RequestEvent] = match from {
            S::New => &[],
            S::Pending => &[E::Start],
            S::Approved => &[E::Start, E::Approve],
            S::Dead => &[E::Start, E::WaitElevenMinutes],
            S::Gone => &[E::Start, E::Deny],
        };
        for &step in steps {
            assert!(owner.apply(step).await, "arranging {email}: {step:?}");
        }
        assert_eq!(owner.state().await, from, "arranging {email}");
        owner
    }

    async fn apply(&mut self, event: RequestEvent) -> bool {
        match event {
            E::Start => self.start().await,
            E::ReturnsAfterEightMinutes => {
                self.browser.minutes += 8;
                let path = format!("{OAUTH_AUTHORIZE_PATH}?request={}", self.request_id());
                let page = self.browser.get(&path).await;
                page.status() == StatusCode::OK && self.state().await == S::Pending
            }
            E::Approve => {
                let form = approve_form(&self.request_id(), SCOPES, TEST_PASSWORD);
                self.answer(&form).await;
                self.code = param_of(self.answer.as_deref(), "code");
                self.code.is_some()
            }
            E::Deny => {
                self.answer(&deny_form(&self.request_id())).await;
                param_of(self.answer.as_deref(), "error").as_deref() == Some("access_denied")
            }
            E::ExchangeCode => self.exchange().await,
            E::WaitElevenMinutes => {
                self.browser.minutes += 11;
                true
            }
            E::Cleanup => {
                let later = App {
                    clock: Clock::Fixed(self.browser.now()),
                    ..self.app.clone()
                };
                delete_expired(&later).await.unwrap() > 0
            }
        }
    }

    async fn start(&mut self) -> bool {
        let path = start_path(&self.app, &self.client_id, SCOPES);
        let response = self.browser.get(&path).await;
        let started = param_of(location(&response).as_deref(), "request");
        if self.request.is_none() {
            self.request = started.clone();
        }
        response.status() == StatusCode::SEE_OTHER && started.is_some()
    }

    async fn answer(&mut self, form: &str) {
        let response = self.browser.post(OAUTH_AUTHORIZE_PATH, form).await;
        self.answer = location(&response).filter(|to| to.starts_with(REDIRECT_URI));
    }

    async fn exchange(&mut self) -> bool {
        let code = self
            .code
            .clone()
            .unwrap_or_else(|| "wsc_never_issued".to_string());
        let form = code_form(&self.app, &self.client_id, &code);
        self.browser.post(TOKEN_PATH, &form).await.status() == StatusCode::OK
    }

    fn request_id(&self) -> String {
        self.request
            .clone()
            .unwrap_or_else(|| PublicIdKind::AuthorizationRequest.generate())
    }

    async fn state(&self) -> RequestState {
        let Some(request) = &self.request else {
            return S::New;
        };
        let row = sqlx::query!(
            "SELECT status, acted_at FROM oauth_requests WHERE public_id = $1",
            request
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap();
        match row {
            None => S::Gone,
            Some(row) if self.browser.now() - row.acted_at >= REQUEST_IDLE_LIMIT => S::Dead,
            Some(row) if row.status == "approved" => S::Approved,
            Some(_) => S::Pending,
        }
    }

    async fn broken_rules(
        &mut self,
        from: RequestState,
        event: RequestEvent,
        accepted: bool,
        actual: RequestState,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        if event == E::ReturnsAfterEightMinutes && from == S::Pending && !self.touched_now().await {
            broken.push(KEPT_ALIVE);
        }
        if matches!(event, E::Approve | E::Deny) && accepted && !self.answered_with_state() {
            broken.push(ANSWERS_GO_BACK_TO_THE_APP);
        }
        if !self.code_once_and_hashed(event, accepted, actual).await {
            broken.push(CODE_ONCE_AND_HASHED);
        }
        broken
    }

    async fn touched_now(&self) -> bool {
        let acted_at = sqlx::query_scalar!(
            "SELECT acted_at FROM oauth_requests WHERE public_id = $1",
            self.request_id()
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap();
        acted_at.is_some_and(|at| (self.browser.now() - at).abs() < Duration::seconds(1))
    }

    fn answered_with_state(&self) -> bool {
        param_of(self.answer.as_deref(), "state").as_deref() == Some("state-1")
    }

    async fn code_once_and_hashed(
        &mut self,
        event: RequestEvent,
        accepted: bool,
        actual: RequestState,
    ) -> bool {
        let Some(code) = self.code.clone() else {
            return true;
        };
        let stored_raw = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM oauth_requests WHERE code_hash = $1) AS "found!""#,
            code
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        let used = event == E::ExchangeCode && accepted;
        if used || actual == S::Dead {
            return !stored_raw && !self.exchange().await;
        }
        !stored_raw
    }
}
