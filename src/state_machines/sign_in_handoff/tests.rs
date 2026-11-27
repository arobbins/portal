use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use topcoat::asset::AssetBundle;

use super::*;
use crate::account::tabs::AccountPage;
use crate::api::scope::ApiScope;
use crate::auth::links::TokenPurpose;
use crate::billing::testing::{FakeStripe, WEBHOOK_SECRET};
use crate::env::App;
use crate::oauth::testing::test_client;
use crate::routes::{LOGIN_PATH, LOGOUT_PATH, OAUTH_AUTHORIZE_PATH};
use crate::state_machines::testing::oauth::start_path;
use crate::state_machines::testing::{Browser, assets, deliver_webhook, latest_link, location};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::testing::{TEST_PASSWORD, active_user};

const HOUR: i64 = 60 * 60;
const PAYMENT_RECEIVED: &str = "Payment received.";
const CHECKOUT_URL: &str = "https://checkout.test/";

struct Visitor {
    app: App,
    browser: Browser,
    user_id: i32,
    email: String,
    cell: u8,
    landed: Option<String>,
}

#[sqlx::test]
async fn every_sign_in_handoff_state_and_event_behaves_as_declared(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in SIGN_IN_HANDOFF.states.iter().enumerate() {
        for (column, &event) in SIGN_IN_HANDOFF.events.iter().enumerate() {
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
            let cell = (row * 16 + column) as u8;
            let mut visitor = Visitor::arrange(&app, &assets, cell, from).await;
            visitor.landed = None;
            let accepted = visitor.apply(event).await;
            let (actual, page) = visitor.observe().await;
            let mut broken = Vec::new();
            if !visitor.went_where_chosen(from, event) {
                broken.push(GOES_WHERE_CHOSEN);
            }
            let notice = page.is_some_and(|body| body.contains(PAYMENT_RECEIVED));
            if notice != (actual == S::AwaitingPayment) {
                broken.push(NOTICE_ONLY_WHILE_AWAITING);
            }
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(actual),
                accepted,
                broken,
            });
        }
    }

    write_results(&SIGN_IN_HANDOFF, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

impl Visitor {
    async fn arrange(app: &App, assets: &AssetBundle, cell: u8, from: HandoffState) -> Self {
        let email = format!("handoff-{cell}@example.test");
        let user = active_user(&email, &app.db).await;
        let mut visitor = Self {
            app: app.clone(),
            browser: Browser::new(app, assets, [198, 51, 143, cell]),
            user_id: user.id,
            email,
            cell,
            landed: None,
        };
        let steps: &[HandoffEvent] = match from {
            S::SignedOut => &[],
            S::ChoseCheckout => &[E::OpensSignInWithPrice],
            S::ChoseAuthorize => &[E::OpensAppRequest],
            S::SignedIn => &[E::SignsIn],
            S::AwaitingPayment => &[E::OpensSignInWithPrice, E::SignsIn, E::ReturnsFromCheckout],
        };
        for &step in steps {
            assert!(visitor.apply(step).await, "arranging cell {cell}: {step:?}");
        }
        assert_eq!(visitor.observe().await.0, from, "arranging cell {cell}");
        visitor
    }

    async fn apply(&mut self, event: HandoffEvent) -> bool {
        match event {
            E::OpensSignInWithPrice => {
                let path = format!("{LOGIN_PATH}?price=price_pro");
                self.browser.get(&path).await.status() == StatusCode::OK
            }
            E::OpensAppRequest => self.open_app_request().await,
            E::SignsIn => {
                let form = format!("email={}&password={TEST_PASSWORD}", encoded(&self.email));
                self.land(LOGIN_PATH, &form).await
            }
            E::RegistersAndConfirms => self.register().await,
            E::WaitsAnHour => {
                self.browser.minutes += 61;
                true
            }
            E::ReturnsFromCheckout => {
                let response = self.browser.get("/account/checkout/paid").await;
                location(&response) == Some(AccountPage::Plan.path())
            }
            E::SubscriptionArrives => self.subscription_arrives().await,
            E::SignsOut => {
                let response = self.browser.post(LOGOUT_PATH, "").await;
                response.status() == StatusCode::SEE_OTHER
            }
        }
    }

    async fn open_app_request(&mut self) -> bool {
        let client_id = test_client(&self.app.db).await;
        let mut path = start_path(&self.app, &client_id, &[ApiScope::ProfileRead]);
        for _ in 0..3 {
            let response = self.browser.get(&path).await;
            match location(&response) {
                Some(next) if next.starts_with('/') => path = next,
                _ => return response.status() == StatusCode::OK,
            }
        }
        false
    }

    async fn register(&mut self) -> bool {
        let email = format!("registrant-{}@example.test", self.cell);
        let mut form = format!("name=Registrant&email={}", encoded(&email));
        if let Some((param, value)) = self.chosen().await {
            form.push_str(&format!("&{param}={value}"));
        }
        self.browser.post("/register", &form).await;
        let purpose = TokenPurpose::EmailVerification;
        let Some(raw) = latest_link(purpose, &email, &self.app.db).await else {
            return false;
        };
        let password = format!("password={TEST_PASSWORD}&confirm_password={TEST_PASSWORD}");
        self.land(&purpose.path(&raw), &password).await
    }

    async fn land(&mut self, path: &str, form: &str) -> bool {
        let response = self.browser.post(path, form).await;
        self.landed = location(&response);
        response.status() == StatusCode::SEE_OTHER && self.landed.as_deref() != Some(LOGIN_PATH)
    }

    async fn subscription_arrives(&mut self) -> bool {
        let customer = sqlx::query_scalar!(
            "SELECT stripe_customer_id FROM billing_customers WHERE user_id = $1",
            self.user_id
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap()
        .unwrap_or_else(|| "cus_nobody".to_string());
        let body = created_event(self.cell, &customer);
        let delivered = deliver_webhook(&mut self.browser, &body, WEBHOOK_SECRET).await;
        let page = self.browser.get(&AccountPage::Plan.path()).await;
        delivered == StatusCode::OK && page.status() == StatusCode::OK
    }

    async fn chosen(&self) -> Option<(&'static str, String)> {
        let stored = self.fresh("after_sign_in", "at").await?;
        let handoff = &stored["handoff"];
        if let Some(price) = handoff["Checkout"].as_str() {
            return Some(("price", price.to_string()));
        }
        handoff["Authorize"]
            .as_str()
            .map(|request| ("authorize", request.to_string()))
    }

    async fn fresh(&self, key: &str, field: &str) -> Option<Value> {
        let value = self.browser.session_value(key).await?;
        let at = if field.is_empty() {
            value.as_i64()?
        } else {
            value[field].as_i64()?
        };
        (self.browser.now().unix_timestamp() - at < HOUR).then_some(value)
    }

    async fn observe(&mut self) -> (HandoffState, Option<String>) {
        let response = self.browser.get(&AccountPage::Plan.path()).await;
        if response.status() == StatusCode::OK {
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let page = String::from_utf8_lossy(&body).into_owned();
            let state = match self.fresh("checkout_pending", "").await {
                Some(_) => S::AwaitingPayment,
                None => S::SignedIn,
            };
            return (state, Some(page));
        }
        let state = match self.chosen().await {
            Some(("price", _)) => S::ChoseCheckout,
            Some(_) => S::ChoseAuthorize,
            None => S::SignedOut,
        };
        (state, None)
    }

    fn went_where_chosen(&self, from: HandoffState, event: HandoffEvent) -> bool {
        if !matches!(event, E::SignsIn | E::RegistersAndConfirms) {
            return true;
        }
        let landed = self.landed.as_deref().unwrap_or_default();
        match from {
            S::ChoseCheckout => landed.starts_with(CHECKOUT_URL),
            S::ChoseAuthorize => landed.starts_with(&format!("{OAUTH_AUTHORIZE_PATH}?request=")),
            S::SignedOut => landed == "/passkey-setup",
            S::SignedIn | S::AwaitingPayment => true,
        }
    }
}

fn encoded(value: &str) -> String {
    value.replace('@', "%40")
}

fn created_event(cell: u8, customer: &str) -> String {
    json!({
        "id": format!("evt_handoff_{cell}"), "object": "event", "type": "customer.subscription.created",
        "created": 1_800_000_000,
        "data": { "object": {
            "id": format!("sub_handoff_{cell}"), "object": "subscription", "customer": customer,
            "status": "active", "cancel_at_period_end": false, "cancel_at": null,
            "current_period_end": 1_900_000_000,
            "items": { "data": [{ "price": { "id": "price_pro" }, "current_period_end": 1_900_000_000 }] },
        } },
    })
    .to_string()
}
