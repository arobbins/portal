use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::action::Caller;
use crate::env::App;
use crate::routes::{ACCOUNT_PATH, LOGIN_PATH, LOGOUT_PATH};
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::two_factor::actions::{EnableTwoFactor, enable_two_factor};
use crate::two_factor::codes::{code_at, new_secret};
use crate::user::testing::{TEST_PASSWORD, active_user};

const CODE_PAGE: &str = "/login/two-factor";

struct Visitor {
    browser: Browser,
    email: String,
    secret: String,
    recovery: Vec<String>,
    last_code: Option<String>,
}

#[sqlx::test]
async fn every_second_factor_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in SECOND_FACTOR.states.iter().enumerate() {
        for (column, &event) in SECOND_FACTOR.events.iter().enumerate() {
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

    write_results(&SECOND_FACTOR, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn run_cell(
    app: &App,
    assets: &AssetBundle,
    [row, column]: [usize; 2],
    from: SecondFactorState,
    event: SecondFactorEvent,
) -> (SecondFactorState, bool, Vec<&'static str>) {
    let email = format!("second-factor-{row}-{column}@example.test");
    let user = active_user(&email, &app.db).await;
    let secret = new_secret();
    let now = app.clock.now().unix_timestamp();
    let input = EnableTwoFactor {
        secret: secret.clone(),
        code: code_at(&secret, now),
        current_password: TEST_PASSWORD.to_string(),
    };
    let ip = std::net::IpAddr::from([198, 51, 161, (row * 16 + column) as u8]);
    let recovery = enable_two_factor(&app.env(Some(ip)), &Caller::browser(user), &input, None)
        .await
        .unwrap()
        .unwrap();
    let mut visitor = Visitor {
        browser: Browser::new(app, assets, [198, 51, 160, (row * 16 + column) as u8]),
        email,
        secret,
        recovery,
        last_code: None,
    };
    visitor.browser.minutes += 1;
    visitor.arrange(from).await;

    let accepted = visitor.apply(event).await;
    let seen = visitor.look().await;
    let broken = visitor.broken_rules(from, event, accepted, seen).await;
    (seen, accepted, broken)
}

impl Visitor {
    async fn arrange(&mut self, from: SecondFactorState) {
        if from != S::SignedOut {
            assert!(self.enter_password().await, "arranging {}", self.email);
        }
        if from == S::SignedIn {
            let code = self.app_code();
            assert!(self.enter_code(&code).await, "arranging {}", self.email);
            self.browser.minutes += 1;
        }
        assert_eq!(self.look().await, from, "arranging {}", self.email);
    }

    async fn apply(&mut self, event: SecondFactorEvent) -> bool {
        match event {
            E::EnterPassword => self.enter_password().await,
            E::RightCode => {
                let code = self.app_code();
                self.enter_code(&code).await
            }
            E::RecoveryCode => {
                let code = self.recovery[0].clone();
                self.enter_code(&code).await
            }
            E::WrongCode => self.enter_code("000000").await,
            E::RightCodeLate => {
                self.browser.minutes += LATE_MINUTES;
                let code = self.app_code();
                let response = self.post_code(&code).await;
                location(&response).as_deref() == Some(LOGIN_PATH)
            }
            E::OpenAccount => self.browser.get(ACCOUNT_PATH).await.status() == StatusCode::OK,
            E::OpenSignIn => self.browser.get(LOGIN_PATH).await.status() == StatusCode::OK,
        }
    }

    fn app_code(&mut self) -> String {
        let code = code_at(&self.secret, self.browser.now().unix_timestamp());
        self.last_code = Some(code.clone());
        code
    }

    async fn enter_password(&mut self) -> bool {
        let form = format!(
            "email={}&password={TEST_PASSWORD}",
            self.email.replace('@', "%40")
        );
        let response = self.browser.post(LOGIN_PATH, &form).await;
        location(&response).as_deref() == Some(CODE_PAGE)
    }

    async fn post_code(&mut self, code: &str) -> axum::response::Response {
        self.browser.post(CODE_PAGE, &format!("code={code}")).await
    }

    async fn enter_code(&mut self, code: &str) -> bool {
        let response = self.post_code(code).await;
        response.status() == StatusCode::SEE_OTHER
            && !matches!(location(&response).as_deref(), Some(LOGIN_PATH | CODE_PAGE))
    }

    async fn look(&mut self) -> SecondFactorState {
        if self.browser.get(ACCOUNT_PATH).await.status() == StatusCode::OK {
            return S::SignedIn;
        }
        match self.browser.get(CODE_PAGE).await.status() {
            StatusCode::OK => S::Waiting,
            StatusCode::SEE_OTHER => S::SignedOut,
            status => panic!("unexpected {status} from {CODE_PAGE} for {}", self.email),
        }
    }

    async fn broken_rules(
        &mut self,
        from: SecondFactorState,
        event: SecondFactorEvent,
        accepted: bool,
        seen: SecondFactorState,
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        if seen == S::Waiting {
            let response = self.browser.get(ACCOUNT_PATH).await;
            if location(&response).as_deref() != Some(LOGIN_PATH) {
                broken.push(ACCOUNT_NEEDS_THE_CODE);
            }
        }
        if from == S::Waiting
            && event == E::RightCode
            && accepted
            && !self.used_code_refused().await
        {
            broken.push(CODE_WORKS_ONCE);
        }
        if from == S::Waiting && event == E::WrongCode && !self.locked_after_wrong_codes().await {
            broken.push(WRONG_CODES_LOCK);
        }
        broken
    }

    async fn used_code_refused(&mut self) -> bool {
        let code = self.last_code.clone().unwrap_or_default();
        self.browser.post(LOGOUT_PATH, "").await;
        self.enter_password().await && !self.enter_code(&code).await
    }

    async fn locked_after_wrong_codes(&mut self) -> bool {
        for _ in 0..4 {
            self.enter_code("000000").await;
            self.enter_password().await;
        }
        self.browser.minutes += 1;
        let code = self.app_code();
        !self.enter_code(&code).await
    }
}
