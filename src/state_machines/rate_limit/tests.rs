use axum::http::StatusCode;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::env::App;
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const BUCKET: &str = "failed_logins_by_account_and_ip";

struct Attempts {
    app: App,
    assets: AssetBundle,
    email: String,
    other_email: String,
    cell: u8,
    browsers: u8,
}

#[sqlx::test]
async fn every_rate_limit_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in RATE_LIMIT.states.iter().enumerate() {
        for (column, &event) in RATE_LIMIT.events.iter().enumerate() {
            let cell = (row * 16 + column) as u8;
            let email = format!("limited-{cell}@example.test");
            let other_email = format!("limited-other-{cell}@example.test");
            active_user(&email, &db).await;
            active_user(&other_email, &db).await;
            let mut attempts = Attempts {
                app: app.clone(),
                assets: assets.clone(),
                email,
                other_email,
                cell,
                browsers: 0,
            };
            attempts.arrange(from).await;
            let answer = attempts.apply(event).await;
            let actual = attempts.state().await;
            let mut broken = Vec::new();
            let expected_answer = if from == S::Locked {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::SEE_OTHER
            };
            if event == E::RightPassword && answer != expected_answer {
                broken.push(LOCKED_EVEN_WITH_THE_RIGHT_PASSWORD);
            }
            if attempts.limiter_says_locked().await != (actual == S::Locked) {
                broken.push(LIMITER_AGREES);
            }
            if actual == S::Locked && !attempts.signs_in_from_another_ip().await {
                broken.push(ONLY_THIS_ADDRESS_IS_LOCKED);
            }
            cells.push(CellResult {
                from,
                event,
                expected: expect(from, event),
                actual: Some(actual),
                accepted: true,
                broken,
            });
        }
    }

    write_results(&RATE_LIMIT, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Attempts {
    async fn arrange(&mut self, from: LockState) {
        let failures = match from {
            S::Clear => 0,
            S::Failing => FAILURES_TO_LOCK - 1,
            S::Locked => FAILURES_TO_LOCK,
        };
        for _ in 0..failures {
            self.apply(E::WrongPassword).await;
        }
        assert_eq!(self.state().await, from, "arranging {}", self.email);
    }

    async fn apply(&mut self, event: LockEvent) -> StatusCode {
        let (email, password, ip) = match event {
            E::WrongPassword => (self.email.clone(), "not-the-password", self.ip()),
            E::RightPassword => (self.email.clone(), TEST_PASSWORD, self.ip()),
            E::WrongFromAnotherIp => (self.email.clone(), "not-the-password", self.other_ip()),
            E::WrongForAnotherAccount => (self.other_email.clone(), "not-the-password", self.ip()),
            E::WindowPasses => {
                sqlx::query!(
                    "UPDATE rate_limit_hits SET at = at - make_interval(mins => $2) WHERE bucket = $1",
                    BUCKET,
                    (WINDOW_MINUTES + 1) as i32,
                )
                .execute(&self.app.db)
                .await
                .unwrap();
                return StatusCode::OK;
            }
        };
        self.sign_in(&email, password, ip).await
    }

    async fn sign_in(&mut self, email: &str, password: &str, ip: [u8; 4]) -> StatusCode {
        let mut browser = Browser::new(&self.app, &self.assets, ip);
        let form = format!("email={}&password={password}", email.replace('@', "%40"));
        let response = browser.post("/login", &form).await;
        if response.status() == StatusCode::SEE_OTHER
            && location(&response).as_deref() != Some("/login")
        {
            return StatusCode::SEE_OTHER;
        }
        response.status()
    }

    async fn signs_in_from_another_ip(&mut self) -> bool {
        let (email, ip) = (self.email.clone(), self.fresh_ip());
        self.sign_in(&email, TEST_PASSWORD, ip).await == StatusCode::SEE_OTHER
    }

    async fn limiter_says_locked(&self) -> bool {
        let ip = std::net::IpAddr::from(self.ip());
        self.app
            .limits
            .login_blocked(&self.email, Some(ip))
            .await
            .unwrap()
    }

    fn ip(&self) -> [u8; 4] {
        [198, 51, 130, self.cell]
    }

    fn other_ip(&self) -> [u8; 4] {
        [198, 51, 131, self.cell]
    }

    fn fresh_ip(&mut self) -> [u8; 4] {
        self.browsers += 1;
        [198, 51, 132 + self.browsers, self.cell]
    }

    async fn state(&self) -> LockState {
        let key = format!("{} 198.51.130.{}", self.email, self.cell);
        let failures = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM rate_limit_hits WHERE bucket = $1 AND key = $2 AND at > now() - make_interval(mins => $3)"#,
            BUCKET,
            key,
            WINDOW_MINUTES as i32,
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap();
        match failures {
            0 => S::Clear,
            count if count < FAILURES_TO_LOCK => S::Failing,
            _ => S::Locked,
        }
    }
}
