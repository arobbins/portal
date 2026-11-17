use std::net::IpAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::action::Caller;
use crate::admin::actions::{TargetUser, disable_user};
use crate::auth::actions::{
    ChangeEmail, ChangePassword, EmailChangeJob, NewPassword, change_password,
    pending_email_change, request_email_change, set_password_from_link,
};
use crate::auth::links::TokenPurpose;
use crate::env::{App, Env};
use crate::jobs::{Handler, Worker};
use crate::secret::{hash_secret, random_base62};
use crate::state_machines::testing::{Browser, admin_caller, assets, latest_link};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::queries::{query_get_user_by_email, query_get_user_by_id};
use crate::user::testing::{TEST_PASSWORD, active_user};

const NEW_PASSWORD: &str = "another-strong-password";
const PURPOSE: TokenPurpose = TokenPurpose::EmailChange;

struct Mover {
    app: App,
    admin: Caller,
    user: Arc<User>,
    browser: Browser,
    ip: [u8; 4],
    link: Option<String>,
    pending: Option<String>,
    requests: usize,
}

#[sqlx::test]
async fn every_email_change_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let admin = admin_caller("change-admin@example.test", &db).await;
    let mut cells = Vec::new();
    for (row, &from) in EMAIL_CHANGE.states.iter().enumerate() {
        for (column, &event) in EMAIL_CHANGE.events.iter().enumerate() {
            let expected = expect(from, event);
            let ip = [198, 51, 104, (row * 16 + column) as u8];
            let mut mover = Mover::arrange(&app, &assets, &admin, ip, from).await;
            let (state, accepted, broken) = mover.run(event).await;
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

    write_results(&EMAIL_CHANGE, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Mover {
    async fn arrange(
        app: &App,
        assets: &AssetBundle,
        admin: &Caller,
        ip: [u8; 4],
        from: ChangeState,
    ) -> Self {
        sqlx::query!("DELETE FROM jobs")
            .execute(&app.db)
            .await
            .unwrap();
        let email = format!("mover-{}@example.test", ip[3]);
        let mut mover = Self {
            app: app.clone(),
            admin: admin.clone(),
            user: active_user(&email, &app.db).await,
            browser: Browser::new(app, assets, ip),
            ip,
            link: None,
            pending: None,
            requests: 0,
        };
        if from != S::Unchanged {
            assert!(mover.apply(E::Request).await, "requesting for {email}");
        }
        if from == S::Changed {
            assert!(mover.apply(E::Confirm).await, "confirming for {email}");
        }
        assert_eq!(mover.state().await, from, "arranging {email}");
        mover
    }

    async fn run(&mut self, event: ChangeEvent) -> (ChangeState, bool, Vec<&'static str>) {
        let old_link = self.link.clone();
        let old_email = self.current().await.email;
        let version_before = self.current().await.session_version;
        let was_pending = self.state().await == S::Pending;
        let accepted = self.apply(event).await;
        let state = self.state().await;
        let mut broken = Vec::new();
        if state == S::Pending && !self.opening_keeps_the_link().await {
            broken.push(OPENING_KEEPS_THE_LINK);
        }
        if event == E::Request && was_pending && accepted {
            let old = old_link.unwrap_or_default();
            if pending_email_change(&self.env(), &old)
                .await
                .unwrap()
                .is_some()
            {
                broken.push(NEWEST_LINK_ONLY);
            }
        }
        if event == E::RequestTakenAddress && !accepted {
            broken.push(SAME_ANSWER_EITHER_WAY);
        }
        if event == E::Confirm && accepted {
            if !self.notice_sent_to(&old_email).await {
                broken.push(CONFIRMING_TELLS_THE_OLD_ADDRESS);
            }
            if self.current().await.session_version <= version_before {
                broken.push(CONFIRMING_SIGNS_OUT_EVERYWHERE);
            }
        }
        (state, accepted, broken)
    }

    async fn apply(&mut self, event: ChangeEvent) -> bool {
        match event {
            E::Request => {
                let address = self.fresh_address("new");
                self.request(&address).await
            }
            E::RequestTakenAddress => {
                let address = self.fresh_address("taken");
                active_user(&address, &self.app.db).await;
                self.request(&address).await
            }
            E::OpenLink => {
                let path = PURPOSE.path(&self.link_or_random());
                self.browser.get(&path).await.status() == StatusCode::OK
            }
            E::Confirm => self.confirm().await,
            E::AddressTakenFirst => {
                let address = self.pending.clone().unwrap_or_else(|| self.fresh_address("gone"));
                if query_get_user_by_email(&address, &self.app.db).await.unwrap().is_none() {
                    active_user(&address, &self.app.db).await;
                }
                self.confirm().await
            }
            E::LinkExpires => {
                sqlx::query!(
                    "UPDATE user_tokens SET expires_at = now() - interval '1 second' WHERE user_id = $1 AND purpose = $2",
                    self.user.id,
                    PURPOSE.as_str(),
                )
                .execute(&self.app.db)
                .await
                .unwrap()
                .rows_affected()
                    > 0
            }
            E::ChangePassword => self.change_password().await,
            E::PasswordReset => self.reset_password().await,
            E::AdminDisables => {
                let target = TargetUser {
                    id: self.user.public_id.clone(),
                };
                disable_user(&self.env(), &self.admin, &target).await.unwrap().is_ok()
            }
        }
    }

    async fn request(&mut self, address: &str) -> bool {
        self.requests += 1;
        let input = ChangeEmail {
            new_email: address.to_string(),
            current_password: TEST_PASSWORD.to_string(),
        };
        let caller = Caller::browser(Arc::new(self.current().await));
        let accepted = request_email_change(&self.env(), &caller, &input, None)
            .await
            .unwrap()
            .is_ok();
        let worker = Worker::new(self.app.clone(), vec![Handler::of::<EmailChangeJob>()]);
        while worker.run_next().await.unwrap() {}
        if let Some(link) = latest_link(PURPOSE, address, &self.app.db).await {
            self.link = Some(link);
            self.pending = Some(address.to_string());
        }
        accepted
    }

    async fn confirm(&mut self) -> bool {
        let path = PURPOSE.path(&self.link_or_random());
        self.browser.post(&path, "").await.status() == StatusCode::SEE_OTHER
    }

    async fn change_password(&self) -> bool {
        let input = ChangePassword {
            current_password: TEST_PASSWORD.to_string(),
            password: NEW_PASSWORD.to_string(),
            confirm_password: NEW_PASSWORD.to_string(),
        };
        let caller = Caller::browser(Arc::new(self.current().await));
        change_password(&self.env(), &caller, &input)
            .await
            .unwrap()
            .is_ok()
    }

    async fn reset_password(&self) -> bool {
        let raw = random_base62(32);
        sqlx::query!(
            "INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
            self.user.id,
            TokenPurpose::PasswordReset.as_str(),
            hash_secret(&raw),
        )
        .execute(&self.app.db)
        .await
        .unwrap();
        let input = NewPassword {
            password: NEW_PASSWORD.to_string(),
            confirm_password: NEW_PASSWORD.to_string(),
        };
        set_password_from_link(&self.env(), TokenPurpose::PasswordReset, &raw, &input)
            .await
            .unwrap()
            .is_ok()
    }

    async fn opening_keeps_the_link(&mut self) -> bool {
        let Some(link) = self.link.clone() else {
            return false;
        };
        let opened = self.browser.get(&PURPOSE.path(&link)).await.status() == StatusCode::OK;
        opened
            && pending_email_change(&self.env(), &link)
                .await
                .unwrap()
                .is_some()
    }

    async fn notice_sent_to(&self, address: &str) -> bool {
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM jobs WHERE kind = 'send_email' AND payload->>'to' = $1) AS "sent!""#,
            address,
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }

    async fn state(&self) -> ChangeState {
        let live = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM user_tokens WHERE user_id = $1 AND purpose = $2 AND expires_at > now()) AS "live!""#,
            self.user.id,
            PURPOSE.as_str(),
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        match (live, self.current().await.email == self.user.email) {
            (true, _) => S::Pending,
            (false, true) => S::Unchanged,
            (false, false) => S::Changed,
        }
    }

    async fn current(&self) -> User {
        query_get_user_by_id(self.user.id, &self.app.db)
            .await
            .unwrap()
            .unwrap()
    }

    fn fresh_address(&self, kind: &str) -> String {
        format!("{kind}-{}-{}@example.test", self.ip[3], self.requests)
    }

    fn link_or_random(&self) -> String {
        self.link.clone().unwrap_or_else(|| random_base62(32))
    }

    fn env(&self) -> Env<'_> {
        self.app.env(Some(IpAddr::from(self.ip)))
    }
}
