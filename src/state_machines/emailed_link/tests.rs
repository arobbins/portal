use std::net::IpAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::action::Caller;
use crate::admin::actions::{TargetUser, disable_user};
use crate::auth::actions::{
    PasswordResetJob, RequestPasswordReset, confirm_email_change, request_password_reset,
};
use crate::auth::links::{TokenPurpose, delete_stale_sign_ups};
use crate::env::{App, Env};
use crate::jobs::{Handler, Worker};
use crate::secret::{hash_secret, random_base62};
use crate::state_machines::testing::{Browser, admin_caller, assets, latest_link};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::testing::active_user;

const PURPOSE: TokenPurpose = TokenPurpose::PasswordReset;
const NEW_PASSWORD: &str = "another-strong-password";
const DEAD_END: &str = "invalid or has already been used";

struct Holder {
    app: App,
    admin: Caller,
    user: Arc<User>,
    browser: Browser,
    ip: [u8; 4],
    raw: String,
    issued: bool,
}

#[sqlx::test]
async fn every_emailed_link_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let admin = admin_caller("link-admin@example.test", &db).await;
    let mut cells = Vec::new();
    for (row, &from) in EMAILED_LINK.states.iter().enumerate() {
        for (column, &event) in EMAILED_LINK.events.iter().enumerate() {
            let expected = expect(from, event);
            let cell = match expected {
                Expect::Unreachable(_) => (None, false, Vec::new()),
                _ => {
                    let ip = [198, 51, 105, (row * 16 + column) as u8];
                    let mut holder = Holder::arrange(&app, &assets, &admin, ip, from).await;
                    holder.run(event).await
                }
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

    write_results(&EMAILED_LINK, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Holder {
    async fn arrange(
        app: &App,
        assets: &AssetBundle,
        admin: &Caller,
        ip: [u8; 4],
        from: LinkState,
    ) -> Self {
        sqlx::query!("DELETE FROM jobs")
            .execute(&app.db)
            .await
            .unwrap();
        let email = format!("linked-{}@example.test", ip[3]);
        let mut holder = Self {
            app: app.clone(),
            admin: admin.clone(),
            user: active_user(&email, &app.db).await,
            browser: Browser::new(app, assets, ip),
            ip,
            raw: random_base62(32),
            issued: false,
        };
        match from {
            S::New => {}
            S::Live => assert!(holder.apply(E::Issue).await),
            S::Expired => {
                assert!(holder.apply(E::Issue).await);
                assert!(holder.apply(E::Expires).await);
            }
            S::Dead => {
                assert!(holder.apply(E::Issue).await);
                assert!(holder.apply(E::Use).await);
            }
        }
        assert_eq!(holder.state().await, from, "arranging {email}");
        holder
    }

    async fn run(&mut self, event: LinkEvent) -> (Option<LinkState>, bool, Vec<&'static str>) {
        let accepted = self.apply(event).await;
        let state = self.state().await;
        let mut broken = Vec::new();
        if self.stored_raw().await {
            broken.push(ONLY_THE_HASH_IS_STORED);
        }
        if state == S::Live {
            let opened = self.open().await == StatusCode::OK;
            if !opened || self.state().await != S::Live {
                broken.push(OPENING_KEEPS_IT);
            }
        }
        if matches!(state, S::Expired | S::Dead) && !self.dead_end().await {
            broken.push(DEAD_ENDS_LOOK_THE_SAME);
        }
        if self.live_links().await > 1 {
            broken.push(ONE_LIVE_LINK);
        }
        (Some(state), accepted, broken)
    }

    async fn apply(&mut self, event: LinkEvent) -> bool {
        match event {
            E::Issue => self.issue().await,
            E::Open => self.open().await == StatusCode::OK,
            E::Use => {
                let form = format!("password={NEW_PASSWORD}&confirm_password={NEW_PASSWORD}");
                let path = PURPOSE.path(&self.raw);
                self.browser.post(&path, &form).await.status() == StatusCode::SEE_OTHER
            }
            E::Expires => {
                sqlx::query!(
                    "UPDATE user_tokens SET expires_at = now() - interval '1 second' WHERE token_hash = $1",
                    hash_secret(&self.raw),
                )
                .execute(&self.app.db)
                .await
                .unwrap()
                .rows_affected()
                    == 1
            }
            E::Cleanup => {
                delete_stale_sign_ups(&self.app).await.unwrap();
                true
            }
            E::EmailChanged => self.change_email().await,
            E::AdminDisables => {
                let target = TargetUser {
                    id: self.user.public_id.clone(),
                };
                disable_user(&self.env(), &self.admin, &target).await.unwrap().is_ok()
            }
        }
    }

    async fn issue(&mut self) -> bool {
        let input = RequestPasswordReset {
            email: self.user.email.clone(),
        };
        let accepted = request_password_reset(&self.env(), &input)
            .await
            .unwrap()
            .is_ok();
        let worker = Worker::new(self.app.clone(), vec![Handler::of::<PasswordResetJob>()]);
        while worker.run_next().await.unwrap() {}
        if let Some(raw) = latest_link(PURPOSE, &self.user.email, &self.app.db).await
            && raw != self.raw
        {
            if !self.issued {
                self.raw = raw;
            }
            self.issued = true;
        }
        accepted
    }

    async fn change_email(&self) -> bool {
        let raw = random_base62(32);
        let new_email = format!("moved-{}", self.user.email);
        sqlx::query!(
            "INSERT INTO user_tokens (user_id, purpose, token_hash, payload, expires_at) VALUES ($1, $2, $3, $4, now() + interval '1 hour')",
            self.user.id,
            TokenPurpose::EmailChange.as_str(),
            hash_secret(&raw),
            new_email,
        )
        .execute(&self.app.db)
        .await
        .unwrap();
        confirm_email_change(&self.env(), &raw)
            .await
            .unwrap()
            .is_ok()
    }

    async fn open(&mut self) -> StatusCode {
        let path = PURPOSE.path(&self.raw);
        self.browser.get(&path).await.status()
    }

    async fn dead_end(&mut self) -> bool {
        let path = PURPOSE.path(&self.raw);
        let response = self.browser.get(&path).await;
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        status == StatusCode::BAD_REQUEST && String::from_utf8_lossy(&body).contains(DEAD_END)
    }

    async fn state(&self) -> LinkState {
        let expired = sqlx::query_scalar!(
            r#"SELECT expires_at <= now() AS "expired!" FROM user_tokens WHERE token_hash = $1"#,
            hash_secret(&self.raw),
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap();
        match expired {
            Some(false) => S::Live,
            Some(true) => S::Expired,
            None if self.issued => S::Dead,
            None => S::New,
        }
    }

    async fn stored_raw(&self) -> bool {
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM user_tokens WHERE token_hash = $1 OR payload = $1) AS "found!""#,
            self.raw,
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }

    async fn live_links(&self) -> i64 {
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "live!" FROM user_tokens WHERE user_id = $1 AND purpose = $2 AND expires_at > now()"#,
            self.user.id,
            PURPOSE.as_str(),
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }

    fn env(&self) -> Env<'_> {
        self.app.env(Some(IpAddr::from(self.ip)))
    }
}
