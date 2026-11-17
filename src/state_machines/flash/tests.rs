use axum::http::StatusCode;
use sqlx::PgPool;

use super::*;
use crate::account::tabs::{AccountPage, AccountTab};
use crate::env::App;
use crate::flash::{Flash, SESSION_FLASH};
use crate::state_machines::testing::{Browser, assets, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const OTHER_PASSWORD: &str = "another-strong-password";
const OPEN_TOAST: &str = "class=\"a-toaster a-toaster--is-open\"";

struct Visitor {
    browser: Browser,
    email: String,
    password: &'static str,
    last_location: Option<String>,
}

#[sqlx::test]
async fn every_flash_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in FLASH.states.iter().enumerate() {
        for (column, &event) in FLASH.events.iter().enumerate() {
            let email = format!("flash-{row}-{column}@example.test");
            active_user(&email, &db).await;
            let mut visitor = Visitor {
                browser: Browser::new(&app, &assets, [198, 51, 108, (row * 16 + column) as u8]),
                email,
                password: TEST_PASSWORD,
                last_location: None,
            };
            assert!(visitor.browser.sign_in(&visitor.email, TEST_PASSWORD).await);
            visitor.arrange(from).await;
            let (actual, broken) = visitor.run(from, event).await;
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

    write_results(&FLASH, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Visitor {
    async fn arrange(&mut self, from: FlashState) {
        self.browser.get(&AccountTab::Security.path()).await;
        match from {
            S::Empty => {}
            S::PasswordChanged => self.change_password().await,
            S::OthersSignedOut => self.sign_out_others().await,
        }
        assert_eq!(self.state().await, from, "arranging {}", self.email);
    }

    async fn run(
        &mut self,
        from: FlashState,
        event: FlashEvent,
    ) -> (FlashState, Vec<&'static str>) {
        let mut broken = Vec::new();
        let mut shown_page = None;
        match event {
            E::ChangePassword => self.change_password().await,
            E::SignOutOthers => self.sign_out_others().await,
            E::OpenSignIn => shown_page = Some(self.open(&AccountPage::SignIn.path()).await),
            E::OpenDevices => shown_page = Some(self.open(&AccountPage::Devices.path()).await),
            E::OpenProfile => shown_page = Some(self.open(&AccountTab::Profile.path()).await),
            E::SignOut => {
                self.browser.post(crate::routes::LOGOUT_PATH, "").await;
            }
        }
        if matches!(event, E::ChangePassword | E::SignOutOthers)
            && self
                .last_location
                .as_deref()
                .is_none_or(|location| location.contains('?'))
        {
            broken.push(NEVER_IN_THE_URL);
        }
        let waiting = flash_of(from);
        let accepts = matches!(
            (from, event),
            (S::PasswordChanged, E::OpenSignIn) | (S::OthersSignedOut, E::OpenDevices)
        );
        if let (true, Some(flash), Some(page)) = (accepts, waiting, &shown_page) {
            let path = match event {
                E::OpenDevices => AccountPage::Devices.path(),
                _ => AccountPage::SignIn.path(),
            };
            let again = self.open(&path).await;
            if !shows(page, flash) || shows(&again, flash) {
                broken.push(SHOWN_EXACTLY_ONCE);
            }
        }
        if let (false, Some(page)) = (accepts, &shown_page)
            && page.contains(OPEN_TOAST)
        {
            broken.push(OTHER_PAGES_LEAVE_IT);
        }
        (self.state().await, broken)
    }

    async fn change_password(&mut self) {
        let next = if self.password == TEST_PASSWORD {
            OTHER_PASSWORD
        } else {
            TEST_PASSWORD
        };
        let form = format!(
            "current_password={}&password={next}&confirm_password={next}",
            self.password
        );
        let response = self.browser.post("/account/password/change", &form).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        self.last_location = location(&response);
        self.password = next;
    }

    async fn sign_out_others(&mut self) {
        let response = self
            .browser
            .post("/account/devices/sign-out-others", "")
            .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        self.last_location = location(&response);
    }

    async fn open(&mut self, path: &str) -> String {
        let response = self.browser.get(path).await;
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&body).into_owned()
    }

    async fn state(&self) -> FlashState {
        let waiting = self.browser.session_value(SESSION_FLASH).await;
        let waiting: Option<Flash> = waiting.and_then(|value| serde_json::from_value(value).ok());
        match waiting {
            None => S::Empty,
            Some(Flash::PasswordChanged) => S::PasswordChanged,
            Some(Flash::OtherDevicesSignedOut) => S::OthersSignedOut,
            Some(other) => panic!("unexpected flash {other:?}"),
        }
    }
}

fn flash_of(state: FlashState) -> Option<Flash> {
    match state {
        S::Empty => None,
        S::PasswordChanged => Some(Flash::PasswordChanged),
        S::OthersSignedOut => Some(Flash::OtherDevicesSignedOut),
    }
}

fn shows(page: &str, flash: Flash) -> bool {
    open_toasts(page).iter().any(|text| text == flash.message())
}

fn open_toasts(page: &str) -> Vec<String> {
    page.split(OPEN_TOAST)
        .skip(1)
        .filter_map(|after| {
            let notice = after.find("<div class=\"a-notice")?;
            let text = &after[notice..];
            let start = text.find('>')? + 1;
            let end = text.find("</div>")?;
            Some(text[start..end].trim().to_string())
        })
        .collect()
}
