use axum::http::StatusCode;
use sqlx::PgPool;

use super::*;
use crate::components::form::FORM_ERROR_ID;
use crate::env::App;
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const LOGIN: &str = "/login";
const TOO_MANY: &str = "Too many attempts";

struct Answer {
    status: StatusCode,
    location: Option<String>,
    html: String,
}

impl Answer {
    fn state(&self) -> FailureState {
        match self.status {
            StatusCode::SEE_OTHER if self.location.as_deref() != Some(LOGIN) => S::SignedIn,
            _ if self.html.contains(TOO_MANY) => S::TooMany,
            _ if error_message(&self.html).is_some() => S::Problem,
            _ => S::Empty,
        }
    }
}

#[sqlx::test]
async fn every_form_failure_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in FORM_FAILURE.states.iter().enumerate() {
        for (column, &event) in FORM_FAILURE.events.iter().enumerate() {
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
            let email = format!("failing-{row}-{column}@example.test");
            active_user(&email, &db).await;
            let ip = [198, 51, 107, (row * 16 + column) as u8];
            let mut browser = Browser::new(&app, &assets, ip);
            let arranged = arrange(&mut browser, &email, from).await;
            assert_eq!(arranged.state(), from, "arranging {email}");
            let answer = apply(&mut browser, &email, event).await;
            let broken = broken_rules(&mut browser, &answer).await;
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(answer.state()),
                accepted: true,
                broken,
            });
        }
    }

    write_results(&FORM_FAILURE, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn arrange(browser: &mut Browser, email: &str, from: FailureState) -> Answer {
    match from {
        S::Empty | S::SignedIn => open(browser).await,
        S::Problem => apply(browser, email, E::WrongPassword).await,
        S::TooMany => apply(browser, email, E::FiveWrong).await,
    }
}

async fn apply(browser: &mut Browser, email: &str, event: FailureEvent) -> Answer {
    match event {
        E::WrongPassword => send(browser, email, "not-the-password").await,
        E::UnknownEmail => send(browser, "nobody-here@example.test", TEST_PASSWORD).await,
        E::RightPassword => send(browser, email, TEST_PASSWORD).await,
        E::Reload => open(browser).await,
        E::FiveWrong => {
            for _ in 0..4 {
                send(browser, email, "not-the-password").await;
            }
            send(browser, email, "not-the-password").await;
            send(browser, email, "not-the-password").await
        }
    }
}

async fn broken_rules(browser: &mut Browser, answer: &Answer) -> Vec<&'static str> {
    let mut broken = Vec::new();
    let state = answer.state();
    if state != S::SignedIn && open(browser).await.state() != S::Empty {
        broken.push(PROBLEMS_ARE_NOT_STICKY);
    }
    if state == S::Problem {
        let marked = answer.html.contains("aria-invalid=\"true\"")
            && answer
                .html
                .contains(&format!("aria-describedby=\"{FORM_ERROR_ID}\""));
        if !marked {
            broken.push(FIELDS_POINT_AT_THE_PROBLEM);
        }
    }
    let right_status = match state {
        S::Problem => answer.status == StatusCode::OK,
        S::TooMany => answer.status == StatusCode::TOO_MANY_REQUESTS,
        S::Empty | S::SignedIn => true,
    };
    if !right_status {
        broken.push(STATUS_MATCHES_THE_PROBLEM);
    }
    if state == S::Problem && error_message(&answer.html) != reference_message(browser).await {
        broken.push(NOTHING_IS_REVEALED);
    }
    broken
}

async fn reference_message(browser: &mut Browser) -> Option<String> {
    let answer = send(browser, "nobody-at-all@example.test", "whatever-password").await;
    error_message(&answer.html)
}

async fn send(browser: &mut Browser, email: &str, password: &str) -> Answer {
    let form = format!("email={}&password={password}", email.replace('@', "%40"));
    let response = browser.post(LOGIN, &form).await;
    answer(response).await
}

async fn open(browser: &mut Browser) -> Answer {
    let response = browser.get(LOGIN).await;
    answer(response).await
}

async fn answer(response: axum::http::Response<axum::body::Body>) -> Answer {
    let status = response.status();
    let location = location(&response);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    Answer {
        status,
        location,
        html: String::from_utf8_lossy(&body).into_owned(),
    }
}

fn error_message(html: &str) -> Option<String> {
    let start = html.find(&format!("id=\"{FORM_ERROR_ID}\""))?;
    let rest = &html[start..];
    let text: String = rest
        .split('>')
        .skip(1)
        .filter_map(|part| part.split('<').next())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .take(1)
        .collect();
    (!text.is_empty()).then_some(text)
}
