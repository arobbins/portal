use sqlx::PgPool;
use time::Duration;

use super::*;
use crate::action::Caller;
use crate::admin::actions::{TargetUser, delete_user, enable_user, list_admin_log, sign_out_user};
use crate::admin::console::{Change, change_role};
use crate::api_tokens::lifecycle::delete_long_expired;
use crate::auth::actions::{DeleteAccount, delete_account};
use crate::auth::links::delete_stale_sign_ups;
use crate::devices::lifecycle::delete_stale_devices;
use crate::env::{App, Clock};
use crate::jobs::delete_old_failed_jobs;
use crate::state_machines::testing::admin_caller;
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::events::delete_old_events;
use crate::user::testing::{TEST_PASSWORD, active_user};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    id: i64,
    admin_email: Option<String>,
    action: String,
    target_email: String,
}

struct Bench {
    app: App,
    admin: Caller,
    viewer: Caller,
    target: TargetUser,
    target_email: String,
}

#[sqlx::test]
async fn every_admin_log_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let viewer = admin_caller("log-viewer@example.test", &db).await;
    let mut cells = Vec::new();
    for (row, &from) in ADMIN_LOG.states.iter().enumerate() {
        for (column, &event) in ADMIN_LOG.events.iter().enumerate() {
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
            let target = active_user(&format!("logged-{row}-{column}@example.test"), &db).await;
            let bench = Bench {
                app: app.clone(),
                admin: admin_caller(&format!("acting-{row}-{column}@example.test"), &db).await,
                viewer: viewer.clone(),
                target: TargetUser {
                    id: target.public_id.clone(),
                },
                target_email: target.email.clone(),
            };
            bench.arrange(from).await;
            let before = bench.entries().await;
            bench.apply(event).await;
            let after = bench.entries().await;
            let actual = bench.state().await;
            let broken = bench.broken_rules(event, &before, &after, actual).await;
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

    write_results(&ADMIN_LOG, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Bench {
    async fn arrange(&self, from: LogState) {
        match from {
            S::Nothing => {}
            S::Logged => self.apply(E::AdminActs).await,
            S::AccountGone => self.apply(E::AdminDeletesAccount).await,
        }
        assert_eq!(self.state().await, from, "arranging {}", self.target_email);
    }

    async fn apply(&self, event: LogEvent) {
        let env = self.app.env(None);
        match event {
            E::AdminActs => {
                sign_out_user(&env, &self.admin, &self.target)
                    .await
                    .unwrap()
                    .unwrap();
            }
            E::AdminChangesNothing => {
                enable_user(&env, &self.admin, &self.target)
                    .await
                    .unwrap()
                    .unwrap();
            }
            E::ConsoleGrantsAdmin => {
                change_role(&self.app.db, Change::Grant, &self.target_email)
                    .await
                    .unwrap();
            }
            E::AdminDeletesAccount => {
                delete_user(&env, &self.admin, &self.target)
                    .await
                    .unwrap()
                    .unwrap();
            }
            E::ActingAdminLeaves => {
                let input = DeleteAccount {
                    confirmation: "DELETE".to_string(),
                    current_password: TEST_PASSWORD.to_string(),
                };
                delete_account(&env, &self.admin, &input, None)
                    .await
                    .unwrap()
                    .unwrap();
            }
            E::CleanupYearsLater => self.clean_up_years_later().await,
        }
    }

    async fn clean_up_years_later(&self) {
        sqlx::query!("UPDATE admin_log SET created_at = created_at - interval '5 years'")
            .execute(&self.app.db)
            .await
            .unwrap();
        let later = App {
            clock: Clock::Fixed(time::OffsetDateTime::now_utc() + Duration::days(5 * 365)),
            ..self.app.clone()
        };
        delete_stale_sign_ups(&later).await.unwrap();
        delete_long_expired(&later).await.unwrap();
        delete_old_events(&later).await.unwrap();
        delete_old_failed_jobs(&later).await.unwrap();
        delete_stale_devices(&later).await.unwrap();
    }

    async fn broken_rules(
        &self,
        event: LogEvent,
        before: &[Entry],
        after: &[Entry],
        state: LogState,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        let kept: Vec<&Entry> = after.iter().take(before.len()).collect();
        if kept.len() < before.len() {
            broken.push(NEVER_REMOVED);
        } else if kept.into_iter().ne(before.iter()) {
            broken.push(NEVER_EDITED);
        }
        let expected_admin = match event {
            E::ConsoleGrantsAdmin => None,
            _ => Some(self.admin.user.email.clone()),
        };
        if after
            .iter()
            .skip(before.len())
            .any(|entry| entry.admin_email != expected_admin)
        {
            broken.push(NAMES_WHO_DID_IT);
        }
        if !self.listed_correctly(after, state).await {
            broken.push(SHOWN_ON_THE_LOG_PAGE);
        }
        broken
    }

    async fn listed_correctly(&self, entries: &[Entry], state: LogState) -> bool {
        let shown = list_admin_log(&self.app.env(None), &self.viewer)
            .await
            .unwrap()
            .unwrap();
        let mine: Vec<_> = shown
            .iter()
            .filter(|entry| entry.target_email == self.target_email)
            .collect();
        mine.len() == entries.len()
            && mine
                .iter()
                .all(|entry| entry.target_exists == (state != S::AccountGone))
    }

    async fn entries(&self) -> Vec<Entry> {
        sqlx::query_as!(
            Entry,
            "SELECT id, admin_email, action, target_email FROM admin_log WHERE target_public_id = $1 ORDER BY id",
            self.target.id
        )
        .fetch_all(&self.app.db)
        .await
        .unwrap()
    }

    async fn state(&self) -> LogState {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM users WHERE public_id = $1) AS "exists!""#,
            self.target.id
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        match (self.entries().await.is_empty(), exists) {
            (true, _) => S::Nothing,
            (false, true) => S::Logged,
            (false, false) => S::AccountGone,
        }
    }
}
