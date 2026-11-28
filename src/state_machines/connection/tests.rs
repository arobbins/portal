use std::sync::Arc;

use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::action::Caller;
use crate::admin::actions::{TargetUser, delete_user, disable_user, enable_user, sign_out_user};
use crate::api::scope::ApiScope;
use crate::env::App;
use crate::oauth::access::app_caller;
use crate::oauth::testing::test_client;
use crate::routes::OAUTH_AUTHORIZE_PATH;
use crate::state_machines::testing::oauth::{
    REVOKE_PATH, TOKEN_PATH, approve_form, code_form, param_of, refresh_form, revoke_form,
    start_path,
};
use crate::state_machines::testing::{
    Browser, admin_caller, assets, json_of, location, reset_link, subjects_to,
};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::testing::{TEST_PASSWORD, active_user};

const SCOPES: &[ApiScope] = &[ApiScope::ProfileRead];
const NEW_PASSWORD: &str = "another-strong-password";

#[derive(Clone, Default)]
struct Tokens {
    access: String,
    refresh: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    connected: i64,
    disconnected: i64,
    emails: usize,
}

struct Linked {
    app: App,
    admin: Caller,
    browser: Browser,
    user: Arc<User>,
    client_id: String,
    tokens: Option<Tokens>,
    replaced: Option<Tokens>,
}

#[sqlx::test]
async fn every_connection_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let admin = admin_caller("connection-admin@example.test", &db).await;
    let mut cells = Vec::new();
    for (row, &from) in CONNECTION.states.iter().enumerate() {
        for (column, &event) in CONNECTION.events.iter().enumerate() {
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
            let mut connected = Linked::arrange(&app, &assets, &admin, cell, from).await;
            let before = connected.counts().await;
            let old = connected.tokens.clone();
            let accepted = connected.apply(event).await;
            let actual = connected.state().await;
            let after = connected.counts().await;
            let broken = connected
                .broken_rules(from, event, actual, (before, after), old)
                .await;
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(actual),
                accepted,
                broken,
            });
        }
    }

    write_results(&CONNECTION, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Linked {
    async fn arrange(
        app: &App,
        assets: &AssetBundle,
        admin: &Caller,
        cell: u8,
        from: ConnectionState,
    ) -> Self {
        let email = format!("connector-{cell}@example.test");
        let user = active_user(&email, &app.db).await;
        let mut connected = Self {
            app: app.clone(),
            admin: admin.clone(),
            browser: Browser::new(app, assets, [198, 51, 142, cell]),
            user,
            client_id: test_client(&app.db).await,
            tokens: None,
            replaced: None,
        };
        assert!(connected.browser.sign_in(&email, TEST_PASSWORD).await);
        let steps: &[ConnectionEvent] = match from {
            S::None => &[],
            S::Connected => &[E::Connect],
            S::Suspended => &[E::Connect, E::Refresh, E::AdminDisables],
            S::Disconnected => &[E::Connect, E::OwnerDisconnects],
        };
        for &step in steps {
            assert!(connected.apply(step).await, "arranging {email}: {step:?}");
        }
        assert_eq!(connected.state().await, from, "arranging {email}");
        connected
    }

    async fn apply(&mut self, event: ConnectionEvent) -> bool {
        match event {
            E::Connect => self.connect().await,
            E::Refresh => self.refresh().await,
            E::ReuseRefreshToken => {
                self.refresh().await;
                let old = self.replaced.clone().unwrap_or_default();
                let form = refresh_form(&self.app, &self.client_id, &old.refresh);
                self.browser.post(TOKEN_PATH, &form).await.status() == StatusCode::BAD_REQUEST
            }
            E::AppRevokes => {
                let access = self.tokens.clone().unwrap_or_default().access;
                let form = revoke_form(&access, &self.client_id);
                self.browser.post(REVOKE_PATH, &form).await.status() == StatusCode::OK
            }
            E::OwnerDisconnects => {
                let id = self.public_id().await.unwrap_or_else(|| "app_none".into());
                let form = format!("id={id}");
                let path = "/account/connected-apps/disconnect";
                self.browser.post(path, &form).await.status() == StatusCode::SEE_OTHER
            }
            E::PasswordReset => self.reset_password().await,
            E::SignOutOtherDevices => {
                let path = "/account/devices/sign-out-others";
                self.browser.post(path, "").await.status() == StatusCode::SEE_OTHER
            }
            E::AdminSignsOutEverywhere | E::AdminDisables | E::AdminEnables | E::AccountDeleted => {
                self.admin_acts(event).await
            }
        }
    }

    async fn connect(&mut self) -> bool {
        let start = start_path(&self.app, &self.client_id, SCOPES);
        let started = self.browser.get(&start).await;
        let Some(request) = param_of(location(&started).as_deref(), "request") else {
            return false;
        };
        let form = approve_form(&request, SCOPES, TEST_PASSWORD);
        let approved = self.browser.post(OAUTH_AUTHORIZE_PATH, &form).await;
        let Some(code) = param_of(location(&approved).as_deref(), "code") else {
            return false;
        };
        let form = code_form(&self.app, &self.client_id, &code);
        self.take_tokens(&form).await
    }

    async fn refresh(&mut self) -> bool {
        let Some(tokens) = self.tokens.clone() else {
            return false;
        };
        let form = refresh_form(&self.app, &self.client_id, &tokens.refresh);
        let issued = self.take_tokens(&form).await;
        if issued {
            self.replaced = Some(tokens);
        }
        issued
    }

    async fn take_tokens(&mut self, form: &str) -> bool {
        let response = self.browser.post(TOKEN_PATH, form).await;
        if response.status() != StatusCode::OK {
            return false;
        }
        let body = json_of(response).await;
        self.tokens = Some(Tokens {
            access: body["access_token"].as_str().unwrap().to_string(),
            refresh: body["refresh_token"].as_str().unwrap().to_string(),
        });
        true
    }

    async fn reset_password(&mut self) -> bool {
        let Some(path) = reset_link(&self.app, &self.user.email).await else {
            return false;
        };
        let form = format!("password={NEW_PASSWORD}&confirm_password={NEW_PASSWORD}");
        self.browser.post(&path, &form).await.status() == StatusCode::SEE_OTHER
    }

    async fn admin_acts(&self, event: ConnectionEvent) -> bool {
        let env = self.app.env(None);
        let target = TargetUser {
            id: self.user.public_id.clone(),
        };
        let done = match event {
            E::AdminSignsOutEverywhere => sign_out_user(&env, &self.admin, &target).await,
            E::AdminDisables => disable_user(&env, &self.admin, &target).await,
            E::AdminEnables => enable_user(&env, &self.admin, &target).await,
            _ => delete_user(&env, &self.admin, &target).await,
        };
        done.unwrap().is_ok()
    }

    async fn public_id(&self) -> Option<String> {
        sqlx::query_scalar!(
            "SELECT c.public_id FROM oauth_connections c JOIN oauth_clients a ON a.id = c.client_id WHERE c.user_id = $1 AND a.client_id = $2",
            self.user.id,
            self.client_id,
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap()
    }

    async fn state(&self) -> ConnectionState {
        let disabled = sqlx::query_scalar!(
            r#"SELECT disabled_at IS NOT NULL AS "disabled!" FROM users WHERE id = $1"#,
            self.user.id
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap();
        match (self.public_id().await, disabled) {
            (Some(_), Some(true)) => S::Suspended,
            (Some(_), _) => S::Connected,
            (None, _) if self.tokens.is_some() => S::Disconnected,
            (None, _) => S::None,
        }
    }

    async fn counts(&self) -> Counts {
        let row = sqlx::query!(
            r#"SELECT
                (SELECT count(*) FROM account_events WHERE user_id = $1 AND kind = 'app_connected') AS "connected!",
                (SELECT count(*) FROM account_events WHERE user_id = $1 AND kind = 'app_disconnected') AS "disconnected!""#,
            self.user.id,
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        Counts {
            connected: row.connected,
            disconnected: row.disconnected,
            emails: subjects_to(&self.user.email, &self.app.db)
                .await
                .iter()
                .filter(|subject| subject.ends_with("was connected to your account"))
                .count(),
        }
    }

    async fn gets_in(&self, tokens: Option<&Tokens>) -> bool {
        let Some(tokens) = tokens else {
            return false;
        };
        app_caller(&tokens.access, self.browser.now(), &self.app.db)
            .await
            .unwrap()
            .is_some()
    }

    async fn broken_rules(
        &mut self,
        from: ConnectionState,
        event: ConnectionEvent,
        actual: ConnectionState,
        (before, after): (Counts, Counts),
        old: Option<Tokens>,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        if self.gets_in(self.tokens.as_ref()).await != (actual == S::Connected) {
            broken.push(APP_GETS_IN_ONLY_WHILE_CONNECTED);
        }
        let refreshed = event == E::Refresh && from == S::Connected;
        if refreshed && self.gets_in(old.as_ref()).await {
            broken.push(REFRESH_REPLACES_THE_ACCESS_TOKEN);
        }
        let reconnected = event == E::Connect && from == S::Connected;
        if reconnected && !self.earlier_tokens_are_unknown(old.as_ref()).await {
            broken.push(RECONNECTING_REVOKES_EARLIER_TOKENS);
        }
        let paused = event == E::Refresh && from == S::Suspended;
        if paused && !same_tokens(self.tokens.as_ref(), old.as_ref()) {
            broken.push(A_DISABLED_OWNERS_APP_CANT_REFRESH);
        }
        if event != E::AccountDeleted && !recorded_as_declared(from, event, actual, &before, &after)
        {
            broken.push(CHANGES_ARE_RECORDED);
        }
        if self.stored_raw().await {
            broken.push(ONLY_HASHES_ARE_STORED);
        }
        broken
    }

    async fn earlier_tokens_are_unknown(&mut self, old: Option<&Tokens>) -> bool {
        let Some(old) = old else {
            return false;
        };
        let form = refresh_form(&self.app, &self.client_id, &old.refresh);
        let response = self.browser.post(TOKEN_PATH, &form).await;
        let refused = json_of(response).await["error"] == "invalid_grant";
        refused && !self.gets_in(Some(old)).await && self.public_id().await.is_some()
    }

    async fn stored_raw(&self) -> bool {
        let raws: Vec<String> = [&self.tokens, &self.replaced]
            .into_iter()
            .flatten()
            .flat_map(|tokens| [tokens.access.clone(), tokens.refresh.clone()])
            .collect();
        let found = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM oauth_tokens WHERE token_hash = ANY($1)"#,
            &raws[..],
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        found > 0
    }
}

fn same_tokens(now: Option<&Tokens>, before: Option<&Tokens>) -> bool {
    match (now, before) {
        (Some(now), Some(before)) => now.access == before.access && now.refresh == before.refresh,
        (None, None) => true,
        _ => false,
    }
}

fn recorded_as_declared(
    from: ConnectionState,
    event: ConnectionEvent,
    actual: ConnectionState,
    before: &Counts,
    after: &Counts,
) -> bool {
    let connected = event == E::Connect && actual == S::Connected;
    let by_app_or_owner = matches!(
        event,
        E::ReuseRefreshToken | E::AppRevokes | E::OwnerDisconnects
    );
    let ended = by_app_or_owner && from != actual && actual == S::Disconnected;
    after.connected - before.connected == i64::from(connected)
        && after.disconnected - before.disconnected == i64::from(ended)
        && (after.emails > before.emails) == connected
}
