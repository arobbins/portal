use sqlx::PgPool;

use super::*;
use crate::env::App;
use crate::routes::ACCOUNT_PATH;
use crate::state_machines::testing::{Browser, assets};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

#[test]
fn every_section_is_a_state() {
    for section in AccountSection::ALL {
        assert!(
            ACCOUNT_TAB_SECTIONS.states.contains(&Section(section)),
            "{section:?} isn't in the machine"
        );
    }
}

#[sqlx::test]
async fn every_section_lands_on_its_tab(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    active_user("sections@example.test", &db).await;
    let mut browser = Browser::new(&app, &assets, [198, 51, 140, 1]);
    assert!(
        browser
            .sign_in("sections@example.test", TEST_PASSWORD)
            .await
    );
    let sidebar = page(&mut browser, ACCOUNT_PATH).await;
    let mut cells = Vec::new();
    for &from in ACCOUNT_TAB_SECTIONS.states {
        for &event in ACCOUNT_TAB_SECTIONS.events {
            let expected = expect(from, event);
            let Section(section) = from else {
                cells.push(CellResult {
                    from,
                    event,
                    expected,
                    actual: None,
                    accepted: false,
                    broken: Vec::new(),
                });
                continue;
            };
            let target = match event {
                E::FollowSidebarLink => sidebar_link(&sidebar, section),
                E::OpenItsPath => Some(section.path()),
            };
            let (actual, broken) = match target {
                Some(target) => land(&mut browser, section, &target).await,
                None => (None, Vec::new()),
            };
            cells.push(CellResult {
                from,
                event,
                expected,
                actual,
                accepted: actual.is_some(),
                broken,
            });
        }
    }

    write_results(&ACCOUNT_TAB_SECTIONS, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn land(
    browser: &mut Browser,
    section: AccountSection,
    target: &str,
) -> (Option<SectionState>, Vec<&'static str>) {
    let (path, anchor) = target.split_once('#').unwrap_or((target, ""));
    let html = page(browser, path).await;
    let tab = AccountTab::ALL
        .into_iter()
        .find(|tab| html.contains(&format!("href=\"{}\" aria-current=\"page\"", tab.path())));
    let mut broken = Vec::new();
    if anchor != section.anchor() || !html.contains(&format!("id=\"{anchor}\"")) {
        broken.push(LANDS_AT_ITS_ANCHOR);
    }
    let homes = AccountTab::ALL
        .into_iter()
        .filter(|tab| tab.sections().contains(&section))
        .count();
    if homes != 1 {
        broken.push(ONE_HOME_EACH);
    }
    (tab.map(OnTab), broken)
}

fn sidebar_link(html: &str, section: AccountSection) -> Option<String> {
    let href = section.path();
    let tag = format!("href=\"{href}\"");
    html.match_indices(&tag)
        .filter_map(|(at, _)| html[at..].split_once('>')?.1.split_once('<'))
        .any(|(text, _)| text.trim() == section.label())
        .then_some(href)
}

async fn page(browser: &mut Browser, path: &str) -> String {
    let response = browser.get(path).await;
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8_lossy(&body).into_owned()
}
