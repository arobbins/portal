use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::{Caller, Denied};
use crate::api::scope::ApiScope;
use crate::permission::TokenAccess;
use crate::server::tools::tools;
use crate::state_machines::testing::admin_caller;
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::testing::active_user;

#[test]
fn every_permission_is_a_state() {
    for permission in Permission::ALL {
        assert!(
            PERMISSION.states.contains(&Asked(permission)),
            "{permission:?} isn't in the Permission machine"
        );
    }
}

#[sqlx::test]
async fn every_permission_is_decided_for_every_caller_as_declared(db: PgPool) {
    let subscriber = active_user("permission@example.test", &db).await;
    let admin = admin_caller("permission-admin@example.test", &db)
        .await
        .user;
    let mut cells = Vec::new();
    for &from in PERMISSION.states {
        for &kind in PERMISSION.events {
            let expected = expect(from, kind);
            let Asked(permission) = from else {
                cells.push(CellResult {
                    from,
                    event: kind,
                    expected,
                    actual: None,
                    accepted: false,
                    broken: Vec::new(),
                });
                continue;
            };
            let caller = caller_of(kind, &subscriber, &admin);
            cells.push(CellResult {
                from,
                event: kind,
                expected,
                actual: Some(Decided(decide(&caller, permission))),
                accepted: true,
                broken: broken_rules(permission),
            });
        }
    }

    write_results(&PERMISSION, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

fn caller_of(kind: CallerState, subscriber: &Arc<User>, admin: &Arc<User>) -> Caller {
    match kind {
        C::SubscriberBrowser => Caller::browser(subscriber.clone()),
        C::AdminBrowser => Caller::browser(admin.clone()),
        C::ReadToken => Caller::api_token(subscriber.clone(), 1, vec![ApiScope::ProfileRead]),
        C::WriteToken => Caller::api_token(subscriber.clone(), 2, vec![ApiScope::ProfileWrite]),
        C::AdminWriteToken => Caller::api_token(admin.clone(), 3, vec![ApiScope::ProfileWrite]),
        other => panic!("{other:?} isn't a caller"),
    }
}

fn decide(caller: &Caller, permission: Permission) -> CallerState {
    match caller.require(permission) {
        Ok(()) => C::Allowed,
        Err(Denied::BrowserOnly) => C::NeedsBrowser,
        Err(Denied::MissingScope(_)) => C::MissingScope,
        Err(Denied::NotPermitted(_)) => C::NotPermitted,
    }
}

fn broken_rules(permission: Permission) -> Vec<&'static str> {
    let mut broken = Vec::new();
    let browser_only = permission.token_access() == TokenAccess::BrowserOnly;
    if browser_only != SENSITIVE.contains(&permission) {
        broken.push(SENSITIVE_ARE_BROWSER_ONLY);
    }
    if permission.admins_only() != (permission == P::AdministerUsers) {
        broken.push(ONLY_ADMINISTERING_IS_ADMIN_ONLY);
    }
    let has_endpoint = tools().iter().any(|tool| tool.permission == permission);
    if !browser_only && !has_endpoint {
        broken.push(TOKEN_PERMISSIONS_HAVE_ENDPOINTS);
    }
    broken
}
