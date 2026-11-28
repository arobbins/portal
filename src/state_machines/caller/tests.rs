use std::sync::Arc;

use sqlx::PgPool;
use time::Duration;

use super::*;
use crate::action::{Caller, Denied};
use crate::api::scope::ApiScope;
use crate::api_tokens::testing::issue_token;
use crate::env::App;
use crate::server::tools::tools;
use crate::state_machines::testing::{Browser, admin_caller, assets};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::testing::active_user;

struct Kind {
    caller: Caller,
    token: Option<String>,
}

#[sqlx::test]
async fn every_caller_and_permission_is_decided_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let subscriber = active_user("caller@example.test", &db).await;
    let admin = admin_caller("caller-admin@example.test", &db).await.user;
    let mut browser = Browser::new(&app, &assets, [198, 51, 121, 1]);
    let mut cells = Vec::new();
    for &from in CALLER.states {
        let kind = kind(from, &subscriber, &admin, &db).await;
        for &permission in CALLER.events {
            let expected = expect(from, permission);
            let Some(kind) = &kind else {
                cells.push(CellResult {
                    from,
                    event: permission,
                    expected,
                    actual: None,
                    accepted: false,
                    broken: Vec::new(),
                });
                continue;
            };
            let decision = decide(&kind.caller, permission);
            let broken = broken_rules(kind, permission, decision, &mut browser).await;
            cells.push(CellResult {
                from,
                event: permission,
                expected,
                actual: Some(decision),
                accepted: true,
                broken,
            });
        }
    }

    write_results(&CALLER, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn kind(
    from: CallerState,
    subscriber: &Arc<User>,
    admin: &Arc<User>,
    db: &PgPool,
) -> Option<Kind> {
    let token = async |user: &Arc<User>, scope: ApiScope| {
        let issued = issue_token(user.id, &[scope.as_str()], Duration::days(30), db).await;
        Kind {
            caller: Caller::api_token(user.clone(), issued.id, vec![scope]),
            token: Some(issued.raw),
        }
    };
    let browser = |user: &Arc<User>| Kind {
        caller: Caller::browser(user.clone()),
        token: None,
    };
    Some(match from {
        S::SubscriberBrowser => browser(subscriber),
        S::AdminBrowser => browser(admin),
        S::ReadToken => token(subscriber, ApiScope::ProfileRead).await,
        S::WriteToken => token(subscriber, ApiScope::ProfileWrite).await,
        S::AdminWriteToken => token(admin, ApiScope::ProfileWrite).await,
        S::Allowed | S::NeedsBrowser | S::MissingScope | S::NotPermitted => return None,
    })
}

fn decide(caller: &Caller, permission: Permission) -> CallerState {
    match caller.require(permission) {
        Ok(()) => S::Allowed,
        Err(Denied::BrowserOnly) => S::NeedsBrowser,
        Err(Denied::MissingScope(_)) => S::MissingScope,
        Err(Denied::NotPermitted(_)) => S::NotPermitted,
    }
}

async fn broken_rules(
    kind: &Kind,
    permission: Permission,
    decision: CallerState,
    browser: &mut Browser,
) -> Vec<&'static str> {
    let mut broken = Vec::new();
    let tools: Vec<_> = tools()
        .into_iter()
        .filter(|tool| tool.permission == permission)
        .collect();
    if decision == S::NeedsBrowser && !tools.is_empty() {
        broken.push(BROWSER_ONLY_HAS_NO_ENDPOINT);
    }
    let Some(token) = &kind.token else {
        return broken;
    };
    for tool in tools {
        let endpoint = &tool.endpoint;
        let body = endpoint
            .example_request
            .as_ref()
            .map(|json| json.to_string());
        let response = browser
            .call_api(endpoint.method.as_str(), endpoint.path, token, body)
            .await;
        let served = response.status().is_success();
        let forbidden = response.status() == axum::http::StatusCode::FORBIDDEN;
        let agrees = if decision == S::Allowed {
            served
        } else {
            forbidden
        };
        if !agrees {
            broken.push(ENDPOINTS_AGREE);
        }
    }
    broken
}
