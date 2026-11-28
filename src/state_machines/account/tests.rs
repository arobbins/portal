use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::Caller;
use crate::admin::actions::{TargetUser, delete_user, disable_user, enable_user};
use crate::auth::actions::{
    ChangePassword, DeleteAccount, NewPassword, Register, change_password, delete_account,
    register, set_password_from_link,
};
use crate::auth::links::{TokenPurpose, delete_stale_sign_ups};
use crate::env::{App, Env};
use crate::secret::{hash_secret, random_base62};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::{AccountStatus, User};
use crate::user::queries::{NewUser, query_get_user_by_id, query_insert_user};
use crate::user::testing::{TEST_PASSWORD, active_user};

const NEW_PASSWORD: &str = "another-strong-password";

struct Bench<'a> {
    app: &'a App,
    env: Env<'a>,
    admin: Caller,
    db: &'a PgPool,
}

#[derive(Debug, Default)]
struct Snapshot {
    session_version: Option<i32>,
    events: i64,
    devices: i64,
    live_links: i64,
    credentials: i64,
}

struct Run {
    event: AccountEvent,
    state: AccountState,
    accepted: bool,
    before: Option<Snapshot>,
    after: Snapshot,
}

#[sqlx::test]
async fn every_account_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let bench = Bench {
        app: &app,
        env: app.env(None),
        admin: admin(&db).await,
        db: &db,
    };
    let mut cells = Vec::new();
    for (row, &from) in ACCOUNT.states.iter().enumerate() {
        for (column, &event) in ACCOUNT.events.iter().enumerate() {
            let expected = expect(from, event);
            let email = format!("cell-{row}-{column}@example.test");
            let (actual, accepted, broken) = match expected {
                Expect::Unreachable(_) => (None, false, Vec::new()),
                _ => {
                    let run = bench.run(from, event, &email).await;
                    let broken = broken_rules(&run, expected);
                    (Some(run.state), run.accepted, broken)
                }
            };
            cells.push(CellResult {
                from,
                event,
                expected,
                actual,
                accepted,
                broken,
            });
        }
    }

    write_results(&ACCOUNT, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

fn broken_rules(run: &Run, expected: Expect<AccountState>) -> Vec<&'static str> {
    let after = &run.after;
    let mut broken = Vec::new();
    let disabled = matches!(
        run.state,
        S::DisabledWithPassword | S::DisabledWithoutPassword
    );
    if disabled && (after.devices > 0 || after.live_links > 0) {
        broken.push(DISABLED_HAS_NO_ACCESS);
    }
    if run.state == S::AwaitingPassword && after.devices > 0 {
        broken.push(AWAITING_HAS_NO_DEVICES);
    }
    let leftovers = after.events + after.devices + after.live_links + after.credentials;
    if run.state == S::Deleted && leftovers > 0 {
        broken.push(DELETED_LEAVES_NOTHING);
    }
    match expected {
        Expect::Moves(to) if run.accepted && to != S::Deleted => {
            let before_events = run.before.as_ref().map_or(0, |before| before.events);
            if after.events <= before_events {
                broken.push(CHANGES_ARE_RECORDED);
            }
            if let Some(before) = &run.before {
                let signed_out = after.session_version != before.session_version;
                if signed_out != signs_out_everywhere(run.event) {
                    broken.push(SIGN_OUT_EVERYWHERE);
                }
            }
        }
        Expect::Refused => {
            if let Some(before) = &run.before
                && (after.events != before.events
                    || after.session_version != before.session_version)
            {
                broken.push(REFUSED_CHANGES_NOTHING);
            }
        }
        _ => {}
    }
    broken
}

impl Bench<'_> {
    async fn run(&self, from: AccountState, event: AccountEvent, email: &str) -> Run {
        if from == S::New {
            return self.run_registration(event, email).await;
        }
        let user = self.arrange(from, email).await;
        let before = self.snapshot(user.id).await;
        let accepted = self.apply(event, &user).await;
        Run {
            event,
            state: self.state_of(user.id).await,
            accepted,
            before: Some(before),
            after: self.snapshot(user.id).await,
        }
    }

    async fn run_registration(&self, event: AccountEvent, email: &str) -> Run {
        let accepted = self.register(email).await;
        let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", email)
            .fetch_optional(self.db)
            .await
            .unwrap();
        let (state, after) = match user_id {
            Some(user_id) => (self.state_of(user_id).await, self.snapshot(user_id).await),
            None => (S::New, Snapshot::default()),
        };
        Run {
            event,
            state,
            accepted,
            before: None,
            after,
        }
    }

    async fn arrange(&self, from: AccountState, email: &str) -> User {
        let user = match from {
            S::Active | S::DisabledWithPassword => {
                let user = active_user(email, self.db).await;
                self.give_device(&user).await;
                user
            }
            _ => {
                let new_user = NewUser {
                    name: "Cell",
                    email,
                };
                let id = query_insert_user(&new_user, self.db)
                    .await
                    .unwrap()
                    .unwrap();
                self.plant_link(TokenPurpose::EmailVerification, id).await;
                Arc::new(self.user(id).await.unwrap())
            }
        };
        if matches!(from, S::DisabledWithPassword | S::DisabledWithoutPassword) {
            disable_user(&self.env, &self.admin, &target(&user))
                .await
                .unwrap()
                .unwrap();
        }
        let user = self.user(user.id).await.unwrap();
        assert_eq!(state(&user), from, "arranging {email}");
        user
    }

    async fn apply(&self, event: AccountEvent, user: &User) -> bool {
        let env = &self.env;
        let admin = &self.admin;
        match event {
            E::Register => self.register(&user.email).await,
            E::ConfirmEmail => self.use_link(TokenPurpose::EmailVerification, user).await,
            E::ResetPassword => self.use_link(TokenPurpose::PasswordReset, user).await,
            E::ChangePassword => self.change_password(user).await,
            E::Disable => disable_user(env, admin, &target(user))
                .await
                .unwrap()
                .is_ok(),
            E::Enable => enable_user(env, admin, &target(user))
                .await
                .unwrap()
                .is_ok(),
            E::DeleteByAdmin => delete_user(env, admin, &target(user))
                .await
                .unwrap()
                .is_ok(),
            E::DeleteSelf => self.delete_self(user).await,
            E::Cleanup => {
                self.a_week_later(user.id).await;
                delete_stale_sign_ups(self.app).await.unwrap();
                true
            }
        }
    }

    async fn register(&self, email: &str) -> bool {
        let input = Register {
            authorize: None,
            price: None,
            name: "Cell".to_string(),
            email: email.to_string(),
            time_zone: None,
        };
        register(&self.env, &input).await.unwrap().is_ok()
    }

    async fn use_link(&self, purpose: TokenPurpose, user: &User) -> bool {
        let raw = self.plant_link(purpose, user.id).await;
        let input = NewPassword {
            password: NEW_PASSWORD.to_string(),
            confirm_password: NEW_PASSWORD.to_string(),
            code: String::new(),
        };
        let accepted = set_password_from_link(&self.env, purpose, &raw, &input)
            .await
            .unwrap()
            .is_ok();
        sqlx::query!(
            "DELETE FROM user_tokens WHERE token_hash = $1",
            hash_secret(&raw)
        )
        .execute(self.db)
        .await
        .unwrap();
        accepted
    }

    async fn change_password(&self, user: &User) -> bool {
        let input = ChangePassword {
            current_password: TEST_PASSWORD.to_string(),
            password: NEW_PASSWORD.to_string(),
            confirm_password: NEW_PASSWORD.to_string(),
        };
        let caller = self.signed_in(user).await;
        change_password(&self.env, &caller, &input)
            .await
            .unwrap()
            .is_ok()
    }

    async fn delete_self(&self, user: &User) -> bool {
        let input = DeleteAccount {
            confirmation: "DELETE".to_string(),
            current_password: TEST_PASSWORD.to_string(),
        };
        let caller = self.signed_in(user).await;
        delete_account(&self.env, &caller, &input, None)
            .await
            .unwrap()
            .is_ok()
    }

    async fn signed_in(&self, user: &User) -> Caller {
        Caller::browser(Arc::new(self.user(user.id).await.unwrap()))
    }

    async fn give_device(&self, user: &User) {
        sqlx::query!(
            "INSERT INTO user_devices (public_id, user_id, session_version, user_agent) VALUES ('dev_' || random_base62(20), $1, $2, 'State machine test')",
            user.id,
            user.session_version,
        )
        .execute(self.db)
        .await
        .unwrap();
    }

    async fn plant_link(&self, purpose: TokenPurpose, user_id: i32) -> String {
        let raw = random_base62(32);
        sqlx::query!(
            "INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
            user_id,
            purpose.as_str(),
            hash_secret(&raw),
        )
        .execute(self.db)
        .await
        .unwrap();
        raw
    }

    async fn a_week_later(&self, user_id: i32) {
        sqlx::query!(
            "UPDATE users SET created_at = created_at - interval '8 days' WHERE id = $1",
            user_id
        )
        .execute(self.db)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE user_tokens SET expires_at = now() - interval '1 second' WHERE user_id = $1",
            user_id
        )
        .execute(self.db)
        .await
        .unwrap();
    }

    async fn snapshot(&self, user_id: i32) -> Snapshot {
        sqlx::query_as!(
            Snapshot,
            r#"SELECT
                (SELECT session_version FROM users WHERE id = $1) AS session_version,
                (SELECT count(*) FROM account_events WHERE user_id = $1) AS "events!",
                (SELECT count(*) FROM user_devices WHERE user_id = $1) AS "devices!",
                (SELECT count(*) FROM user_tokens WHERE user_id = $1 AND expires_at > now()) AS "live_links!",
                (SELECT count(*) FROM user_passkeys WHERE user_id = $1)
                    + (SELECT count(*) FROM api_tokens WHERE user_id = $1) AS "credentials!""#,
            user_id,
        )
        .fetch_one(self.db)
        .await
        .unwrap()
    }

    async fn user(&self, user_id: i32) -> Option<User> {
        query_get_user_by_id(user_id, self.db).await.unwrap()
    }

    async fn state_of(&self, user_id: i32) -> AccountState {
        self.user(user_id).await.as_ref().map_or(S::Deleted, state)
    }
}

fn state(user: &User) -> AccountState {
    match user.status() {
        AccountStatus::AwaitingPassword => S::AwaitingPassword,
        AccountStatus::Active { .. } => S::Active,
        AccountStatus::Disabled {
            password_hash: None,
        } => S::DisabledWithoutPassword,
        AccountStatus::Disabled {
            password_hash: Some(_),
        } => S::DisabledWithPassword,
    }
}

fn target(user: &User) -> TargetUser {
    TargetUser {
        id: user.public_id.clone(),
    }
}

async fn admin(db: &PgPool) -> Caller {
    let user = active_user("cell-admin@example.test", db).await;
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
