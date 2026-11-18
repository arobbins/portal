use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::env::App;
use crate::secret::hash_secret;
use crate::state_machines::testing::{Browser, assets, subjects_to};
use crate::state_machines::{CellResult, failures, write_results};
use crate::two_factor::codes::{code_at, new_secret};
use crate::user::testing::{TEST_PASSWORD, active_user};

const PAGE: &str = "/account/security/authentication";
const SHOWN_ONCE: &str = "see them again.";
const ON_EMAIL: &str = "Two-factor authentication is on";
const OFF_EMAIL: &str = "Two-factor authentication is off";

struct Person {
    browser: Browser,
    db: PgPool,
    email: String,
    user_id: i32,
    shown: Vec<String>,
}

#[sqlx::test]
async fn every_two_factor_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in TWO_FACTOR.states.iter().enumerate() {
        for (column, &event) in TWO_FACTOR.events.iter().enumerate() {
            let (actual, accepted, broken) =
                run_cell(&app, &assets, [row, column], from, event).await;
            cells.push(CellResult {
                from,
                event,
                expected: expect(from, event),
                actual: Some(actual),
                accepted,
                broken,
            });
        }
    }

    write_results(&TWO_FACTOR, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn run_cell(
    app: &App,
    assets: &AssetBundle,
    [row, column]: [usize; 2],
    from: TwoFactorState,
    event: TwoFactorEvent,
) -> (TwoFactorState, bool, Vec<&'static str>) {
    let email = format!("two-factor-{row}-{column}@example.test");
    let user = active_user(&email, &app.db).await;
    let mut person = Person {
        browser: Browser::new(app, assets, [198, 51, 150, (row * 16 + column) as u8]),
        db: app.db.clone(),
        email,
        user_id: user.id,
        shown: Vec::new(),
    };
    assert!(person.browser.sign_in(&person.email, TEST_PASSWORD).await);
    if from == S::On {
        assert!(
            person.turn_on(TEST_PASSWORD, true).await,
            "arranging {}",
            person.email
        );
    }
    assert_eq!(person.look().await, from, "arranging {}", person.email);
    let emails_before = subjects_to(&person.email, &person.db).await.len();
    person.shown.clear();

    let accepted = person.apply(event).await;
    let seen = person.look().await;
    let broken = person
        .broken_rules(event, accepted, seen, emails_before)
        .await;
    (seen, accepted, broken)
}

impl Person {
    async fn apply(&mut self, event: TwoFactorEvent) -> bool {
        self.browser.minutes += 1;
        match event {
            E::TurnOn => self.turn_on(TEST_PASSWORD, true).await,
            E::TurnOnWrongCode => self.turn_on(TEST_PASSWORD, false).await,
            E::TurnOnWrongPassword => self.turn_on("not-the-password", true).await,
            E::NewRecoveryCodes => self.new_codes().await,
            E::TurnOff => self.turn_off(TEST_PASSWORD).await,
            E::TurnOffWrongPassword => self.turn_off("not-the-password").await,
        }
    }

    async fn turn_on(&mut self, password: &str, right_code: bool) -> bool {
        let secret = new_secret();
        let code = if right_code {
            code_at(&secret, self.browser.now().unix_timestamp())
        } else {
            "000000".to_string()
        };
        let form = format!("secret={secret}&code={code}&current_password={password}");
        let html = self
            .page_after_post("/account/two-factor/enable", &form)
            .await;
        self.read_codes(&html)
    }

    async fn new_codes(&mut self) -> bool {
        let form = format!("current_password={TEST_PASSWORD}");
        let html = self
            .page_after_post("/account/two-factor/recovery-codes", &form)
            .await;
        self.read_codes(&html)
    }

    async fn turn_off(&mut self, password: &str) -> bool {
        let form = format!("current_password={password}");
        let response = self
            .browser
            .post("/account/two-factor/disable", &form)
            .await;
        response.status() == StatusCode::SEE_OTHER
    }

    async fn page_after_post(&mut self, path: &str, form: &str) -> String {
        let response = self.browser.post(path, form).await;
        body(response).await
    }

    fn read_codes(&mut self, html: &str) -> bool {
        if !html.contains(SHOWN_ONCE) {
            return false;
        }
        self.shown = recovery_codes_in(html);
        true
    }

    async fn look(&mut self) -> TwoFactorState {
        let html = body(self.browser.get(PAGE).await).await;
        if html.contains("On since") {
            S::On
        } else if html.contains("commandfor=\"set-up-two-factor\"") {
            S::Off
        } else {
            panic!("no two-factor card on {PAGE} for {}", self.email)
        }
    }

    async fn broken_rules(
        &mut self,
        event: TwoFactorEvent,
        accepted: bool,
        seen: TwoFactorState,
        emails_before: usize,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        let subjects = subjects_to(&self.email, &self.db).await;
        let new_subjects = &subjects[emails_before.min(subjects.len())..];
        let wanted = match (event, accepted) {
            (E::TurnOn, true) => Some(ON_EMAIL),
            (E::TurnOff, true) => Some(OFF_EMAIL),
            _ => None,
        };
        if let Some(wanted) = wanted
            && !new_subjects.iter().any(|subject| subject == wanted)
        {
            broken.push(CHANGES_ARE_EMAILED);
        }
        if accepted
            && matches!(event, E::TurnOn | E::NewRecoveryCodes)
            && !self.codes_shown_once().await
        {
            broken.push(CODES_SHOWN_ONCE);
        }
        if seen == S::Off && self.rows_kept().await > 0 {
            broken.push(OFF_KEEPS_NOTHING);
        }
        broken
    }

    async fn codes_shown_once(&mut self) -> bool {
        let stored = sqlx::query_scalar!(
            "SELECT code_hash FROM recovery_codes WHERE user_id = $1 AND used_at IS NULL",
            self.user_id
        )
        .fetch_all(&self.db)
        .await
        .unwrap();
        let hashed: Vec<String> = self
            .shown
            .iter()
            .map(|code| hash_secret(&code.replace('-', "")))
            .collect();
        let again = body(self.browser.get(PAGE).await).await;
        self.shown.len() == 10
            && stored.len() == 10
            && hashed.iter().all(|hash| stored.contains(hash))
            && self.shown.iter().all(|code| !stored.contains(code))
            && !again.contains(SHOWN_ONCE)
    }

    async fn rows_kept(&self) -> i64 {
        sqlx::query_scalar!(
            r#"SELECT (SELECT count(*) FROM user_two_factor WHERE user_id = $1) + (SELECT count(*) FROM recovery_codes WHERE user_id = $1) AS "count!""#,
            self.user_id
        )
        .fetch_one(&self.db)
        .await
        .unwrap()
    }
}

fn recovery_codes_in(html: &str) -> Vec<String> {
    let text: String = html
        .split('<')
        .filter_map(|part| part.split_once('>').map(|(_, text)| text))
        .collect::<Vec<_>>()
        .join("");
    let mut codes = Vec::new();
    for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
        let parts: Vec<&str> = word.split('-').collect();
        let looks_right = parts.len() == 2
            && parts.iter().all(|part| {
                part.len() == 5
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            });
        if looks_right && !codes.contains(&word.to_string()) {
            codes.push(word.to_string());
        }
    }
    codes
}

async fn body(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}
