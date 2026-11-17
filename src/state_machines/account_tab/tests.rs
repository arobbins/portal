use axum::http::StatusCode;
use sqlx::PgPool;

use super::*;
use crate::env::App;
use crate::routes::ACCOUNT_PATH;
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const UNKNOWN_TAB: &str = "/account/no-such-tab";

struct Seen {
    state: TabState,
    html: String,
}

#[test]
fn every_tab_is_a_state_and_an_event() {
    for tab in AccountTab::ALL {
        assert!(
            ACCOUNT_TAB.states.contains(&S::Shows(tab)),
            "{tab:?} isn't a state"
        );
        assert!(
            ACCOUNT_TAB.events.contains(&E::Open(tab)),
            "{tab:?} has no event"
        );
    }
}

#[sqlx::test]
async fn every_account_tab_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let email = "tabs@example.test";
    active_user(email, &db).await;
    let mut sign_ins = 0_u8;
    let mut browser = Browser::new(&app, &assets, [198, 51, 106, 0]);
    let mut cells = Vec::new();
    for &from in ACCOUNT_TAB.states {
        for &event in ACCOUNT_TAB.events {
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
            if open(&mut browser, ACCOUNT_PATH).await.state == S::SignedOut {
                sign_ins += 1;
                browser = Browser::new(&app, &assets, [198, 51, 106, sign_ins]);
                assert!(browser.sign_in(email, TEST_PASSWORD).await);
            }
            let mut path = arrange(&mut browser, from).await;
            let seen = apply(&mut browser, event, &mut path).await;
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(seen.state),
                accepted: true,
                broken: broken_rules(&seen),
            });
        }
    }

    write_results(&ACCOUNT_TAB, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn arrange(browser: &mut Browser, from: TabState) -> String {
    let path = match from {
        S::Shows(tab) => tab.path(),
        S::NotFound | S::SignedOut => UNKNOWN_TAB.to_string(),
    };
    if from == S::SignedOut {
        browser.post(crate::routes::LOGOUT_PATH, "").await;
    }
    let seen = open(browser, &path).await;
    assert_eq!(seen.state, from, "arranging {path}");
    path
}

async fn apply(browser: &mut Browser, event: TabEvent, path: &mut String) -> Seen {
    *path = match event {
        E::Open(tab) => tab.path(),
        E::OpenAccount => ACCOUNT_PATH.to_string(),
        E::OpenUnknown => UNKNOWN_TAB.to_string(),
        E::SignOut => {
            browser.post(crate::routes::LOGOUT_PATH, "").await;
            path.clone()
        }
    };
    open(browser, path).await
}

async fn open(browser: &mut Browser, path: &str) -> Seen {
    let response = browser.get(path).await;
    let status = response.status();
    let location = location(&response);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body).into_owned();
    let state = match (status, location.as_deref()) {
        (StatusCode::SEE_OTHER, Some("/login")) => S::SignedOut,
        (StatusCode::NOT_FOUND, _) => S::NotFound,
        (StatusCode::OK, _) => {
            S::Shows(current_tab(&html).unwrap_or_else(|| panic!("no current tab on {path}")))
        }
        (status, location) => panic!("unexpected {status} to {location:?} from {path}"),
    };
    Seen { state, html }
}

fn tab_links(html: &str) -> Vec<(AccountTab, &str)> {
    html.split("<a ")
        .skip(1)
        .filter_map(|tag| tag.split_once('>').map(|(attributes, _)| attributes))
        .filter_map(|attributes| {
            AccountTab::ALL
                .into_iter()
                .find(|tab| attributes.contains(&format!("href=\"{}\"", tab.path())))
                .map(|tab| (tab, attributes))
        })
        .collect()
}

fn current_tab(html: &str) -> Option<AccountTab> {
    tab_links(html)
        .into_iter()
        .find(|(_, attributes)| attributes.contains("l-nav__link--active"))
        .map(|(tab, _)| tab)
}

fn marked_current(html: &str) -> Vec<AccountTab> {
    tab_links(html)
        .into_iter()
        .filter(|(_, attributes)| attributes.contains("aria-current=\"page\""))
        .map(|(tab, _)| tab)
        .collect()
}

fn broken_rules(seen: &Seen) -> Vec<&'static str> {
    let S::Shows(tab) = seen.state else {
        return Vec::new();
    };
    let mut broken = Vec::new();
    if marked_current(&seen.html) != [tab] {
        broken.push(ONE_CURRENT_TAB);
    }
    let title = seen
        .html
        .split_once("<title>")
        .and_then(|(_, rest)| rest.split_once("</title>"))
        .map_or("", |(title, _)| title);
    if !title.starts_with(tab.heading()) {
        broken.push(TITLE_NAMES_THE_TAB);
    }
    let anchors_present = tab
        .sections()
        .iter()
        .all(|section| seen.html.contains(&format!("id=\"{}\"", section.anchor())));
    if !anchors_present {
        broken.push(SECTIONS_ARE_ON_THEIR_TAB);
    }
    broken
}
