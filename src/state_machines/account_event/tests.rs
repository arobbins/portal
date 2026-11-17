use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::Caller;
use crate::activity::actions::list_recent_activity;
use crate::auth::actions::{DeleteAccount, delete_account};
use crate::env::App;
use crate::profile::actions::{UpdateProfile, update_profile};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::events::delete_old_events;
use crate::user::queries::query_get_user_by_id;
use crate::user::testing::{TEST_PASSWORD, active_user};

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    kind: String,
    detail: serde_json::Value,
    channel: String,
    ip: Option<String>,
}

struct Owner {
    app: App,
    user_id: i32,
    entry: Option<i64>,
    renames: usize,
}

#[sqlx::test]
async fn every_account_event_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let mut cells = Vec::new();
    for (row, &from) in ACCOUNT_EVENT.states.iter().enumerate() {
        for (column, &event) in ACCOUNT_EVENT.events.iter().enumerate() {
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
            let user = active_user(&format!("eventful-{row}-{column}@example.test"), &db).await;
            let mut owner = Owner {
                app: app.clone(),
                user_id: user.id,
                entry: None,
                renames: 0,
            };
            owner.arrange(from).await;
            let before = owner.entry_row().await;
            let (name_before, count_before) = owner.profile_and_count().await;
            owner.apply(event).await;
            let actual = owner.state().await;
            let mut broken = Vec::new();
            let after = owner.entry_row().await;
            if before.is_some() && after.is_some() && before != after {
                broken.push(NEVER_EDITED);
            }
            if matches!(event, E::ChangeSucceeds | E::ChangeFails) {
                let (name_after, count_after) = owner.profile_and_count().await;
                let changed = name_after != name_before;
                if count_after - count_before != i64::from(changed) {
                    broken.push(EXACTLY_WITH_ITS_CHANGE);
                }
            }
            if matches!(actual, S::Recorded | S::PastRetention) && !owner.listed().await {
                broken.push(SHOWN_TO_THE_OWNER);
            }
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

    write_results(&ACCOUNT_EVENT, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Owner {
    async fn arrange(&mut self, from: EventState) {
        match from {
            S::None => {}
            S::Recorded => self.apply(E::ChangeSucceeds).await,
            S::PastRetention => {
                self.apply(E::ChangeSucceeds).await;
                self.apply(E::ThirteenMonthsPass).await;
            }
            S::Gone => unreachable!("Gone is terminal"),
        }
        assert_eq!(self.state().await, from, "arranging {}", self.user_id);
    }

    async fn apply(&mut self, event: EventEvent) {
        let db = &self.app.db;
        match event {
            E::ChangeSucceeds => {
                self.renames += 1;
                let name = format!("Renamed {}", self.renames);
                self.rename(&name).await;
                if self.entry.is_none() {
                    self.entry = sqlx::query_scalar!(
                        "SELECT id FROM account_events WHERE user_id = $1 AND kind = 'profile_updated' ORDER BY id LIMIT 1",
                        self.user_id
                    )
                    .fetch_optional(db)
                    .await
                    .unwrap();
                }
            }
            E::ChangeFails => self.rename("   ").await,
            E::OwnerReadsActivity => {
                self.listed().await;
            }
            E::ThirteenMonthsPass => {
                sqlx::query!(
                    "UPDATE account_events SET created_at = now() - interval '400 days' WHERE id = $1",
                    self.entry
                )
                .execute(db)
                .await
                .unwrap();
            }
            E::Cleanup => {
                delete_old_events(&self.app).await.unwrap();
            }
            E::AccountDeleted => {
                let input = DeleteAccount {
                    confirmation: "DELETE".to_string(),
                    current_password: TEST_PASSWORD.to_string(),
                };
                let caller = self.caller().await;
                delete_account(&self.app.env(None), &caller, &input, None)
                    .await
                    .unwrap()
                    .unwrap();
            }
        }
    }

    async fn rename(&self, name: &str) {
        let input = UpdateProfile {
            name: name.to_string(),
        };
        let caller = self.caller().await;
        let _ = update_profile(&self.app.env(None), &caller, &input)
            .await
            .unwrap();
    }

    async fn listed(&self) -> bool {
        let Some(user) = query_get_user_by_id(self.user_id, &self.app.db)
            .await
            .unwrap()
        else {
            return false;
        };
        let caller = Caller::browser(Arc::new(user));
        let activity = list_recent_activity(&self.app.env(None), &caller)
            .await
            .unwrap()
            .unwrap();
        activity
            .iter()
            .any(|entry| entry.summary == "Profile updated")
    }

    async fn caller(&self) -> Caller {
        let user = query_get_user_by_id(self.user_id, &self.app.db)
            .await
            .unwrap()
            .unwrap();
        Caller::browser(Arc::new(user))
    }

    async fn entry_row(&self) -> Option<Entry> {
        sqlx::query_as!(
            Entry,
            "SELECT kind, detail, channel, ip FROM account_events WHERE id = $1",
            self.entry
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap()
    }

    async fn profile_and_count(&self) -> (Option<String>, i64) {
        let row = sqlx::query!(
            r#"SELECT (SELECT name FROM users WHERE id = $1) AS name,
                (SELECT count(*) FROM account_events WHERE user_id = $1 AND kind = 'profile_updated') AS "count!""#,
            self.user_id
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        (row.name, row.count)
    }

    async fn state(&self) -> EventState {
        let Some(id) = self.entry else {
            return S::None;
        };
        let old = sqlx::query_scalar!(
            r#"SELECT created_at < now() - interval '395 days' AS "old!" FROM account_events WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap();
        match old {
            None => S::Gone,
            Some(true) => S::PastRetention,
            Some(false) => S::Recorded,
        }
    }
}
