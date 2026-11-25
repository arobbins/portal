use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::Caller;
use crate::admin::console::{Change, RoleOutcome, change_role};
use crate::api::scope::ApiScope;
use crate::auth::actions::{Register, register};
use crate::env::App;
use crate::permission::Permission;
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::{Role, User};
use crate::user::queries::query_get_user_by_email;
use crate::user::testing::active_user;

#[derive(Debug, Default)]
struct Snapshot {
    session_version: Option<i32>,
    events: i64,
    logged: i64,
}

struct Run {
    event: RoleEvent,
    state: RoleState,
    accepted: bool,
    before: Snapshot,
    after: Snapshot,
    user: Option<Arc<User>>,
}

#[sqlx::test]
async fn every_role_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let mut cells = Vec::new();
    for (row, &from) in ROLE.states.iter().enumerate() {
        for (column, &event) in ROLE.events.iter().enumerate() {
            let email = format!("role-{row}-{column}@example.test");
            let expected = expect(from, event);
            let run = run(&app, from, event, &email).await;
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(run.state),
                accepted: run.accepted,
                broken: broken_rules(&run, expected),
            });
        }
    }

    write_results(&ROLE, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

fn broken_rules(run: &Run, expected: Expect<RoleState>) -> Vec<&'static str> {
    let mut broken = Vec::new();
    if let Some(user) = &run.user
        && !administers_only_as_admin(user)
    {
        broken.push(ONLY_ADMINS_ADMINISTER);
    }
    let (before, after) = (&run.before, &run.after);
    let role_change = matches!(run.event, E::Grant | E::Revoke);
    match expected {
        Expect::Moves(_) if run.accepted && role_change => {
            if after.events != before.events + 1 || after.logged != before.logged + 1 {
                broken.push(CHANGES_ARE_RECORDED);
            }
            if after.session_version != before.session_version {
                broken.push(STAYS_SIGNED_IN);
            }
        }
        Expect::Refused if after.events != before.events || after.logged != before.logged => {
            broken.push(REPEATS_CHANGE_NOTHING);
        }
        _ => {}
    }
    broken
}

fn administers_only_as_admin(user: &Arc<User>) -> bool {
    let in_browser = Caller::browser(user.clone()).require(Permission::AdministerUsers);
    let with_token = Caller::api_token(user.clone(), 1, ApiScope::ALL.to_vec())
        .require(Permission::AdministerUsers);
    in_browser.is_ok() == (user.role == Role::Admin) && with_token.is_err()
}

async fn run(app: &App, from: RoleState, event: RoleEvent, email: &str) -> Run {
    let db = &app.db;
    arrange(from, email, db).await;
    let before = snapshot(email, db).await;
    let accepted = match event {
        E::Register => {
            let input = Register {
                price: None,
                name: "Role".to_string(),
                email: email.to_string(),
                time_zone: None,
            };
            register(&app.env(None), &input).await.unwrap().is_ok()
        }
        E::Grant => change_role(db, Change::Grant, email).await.unwrap() == RoleOutcome::Changed,
        E::Revoke => change_role(db, Change::Revoke, email).await.unwrap() == RoleOutcome::Changed,
    };
    let user = query_get_user_by_email(email, db)
        .await
        .unwrap()
        .map(Arc::new);
    Run {
        event,
        state: user.as_ref().map_or(S::New, |user| state(user.role)),
        accepted,
        before,
        after: snapshot(email, db).await,
        user,
    }
}

async fn arrange(from: RoleState, email: &str, db: &PgPool) {
    if from == S::New {
        return;
    }
    let user = active_user(email, db).await;
    if from == S::Admin {
        sqlx::query!(
            "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
            user.id
        )
        .execute(db)
        .await
        .unwrap();
    }
}

async fn snapshot(email: &str, db: &PgPool) -> Snapshot {
    sqlx::query_as!(
        Snapshot,
        r#"SELECT
            (SELECT session_version FROM users WHERE email = $1) AS session_version,
            (SELECT count(*) FROM account_events e JOIN users u ON u.id = e.user_id WHERE u.email = $1) AS "events!",
            (SELECT count(*) FROM admin_log WHERE target_email = $1) AS "logged!""#,
        email,
    )
    .fetch_one(db)
    .await
    .unwrap()
}

fn state(role: Role) -> RoleState {
    match role {
        Role::Subscriber => S::Subscriber,
        Role::Admin => S::Admin,
    }
}
