use std::sync::Arc;

use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::*;
use crate::action::Caller;
use crate::billing::actions::{StartCheckout, start_checkout};
use crate::billing::jobs::CancelStripeSubscription;
use crate::billing::testing::{FakeStripe, WEBHOOK_SECRET};
use crate::env::App;
use crate::jobs::Job;
use crate::state_machines::testing::{Browser, assets, deliver_webhook};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::testing::active_user;

const BASE_TIME: i64 = 1_800_000_000;
const PERIOD_END: i64 = 1_900_000_000;

struct Truth {
    status: &'static str,
    cancel: bool,
    price: &'static str,
}

struct Subscriber<'a> {
    stripe: &'a FakeStripe,
    db: PgPool,
    browser: Browser,
    user: Arc<User>,
    customer: String,
    cell: usize,
    truth: Truth,
    subscription: String,
    subscriptions: usize,
    sent: i64,
    last: Option<String>,
    status: StatusCode,
}

#[sqlx::test]
async fn every_subscription_state_and_event_behaves_as_declared(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let assets = assets();
    let mut cells = Vec::new();
    for (row, &from) in SUBSCRIPTION.states.iter().enumerate() {
        for (column, &event) in SUBSCRIPTION.events.iter().enumerate() {
            let expected = expect(from, event);
            if let Expect::Unreachable(_) = expected {
                cells.push(unreachable(from, event, expected));
                continue;
            }
            let cell = row * 16 + column;
            let browser = Browser::new(&app, &assets, [198, 51, 140, cell as u8]);
            let mut subscriber = Subscriber::arrange(&stripe, &app, browser, cell, from).await;
            let before = subscriber.billing_events().await;
            subscriber.apply(event).await;
            let actual = subscriber.state().await;
            let broken = subscriber.broken_rules(from, event, &before).await;
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: Some(actual),
                accepted: subscriber.status == StatusCode::OK,
                broken,
            });
        }
    }

    write_results(&SUBSCRIPTION, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

fn unreachable(
    from: PlanState,
    event: StripeChange,
    expected: Expect<PlanState>,
) -> CellResult<PlanState, StripeChange> {
    CellResult {
        from,
        event,
        expected,
        actual: None,
        accepted: false,
        broken: Vec::new(),
    }
}

fn recorded(from: PlanState, event: StripeChange) -> Option<&'static str> {
    let live = !matches!(from, S::Free | S::Ended);
    match (from, event) {
        (S::Free | S::Ended, E::Subscribes) => Some("subscribed"),
        (_, E::Subscribes) => Some("duplicate_subscription_refunded"),
        (S::Free, E::SchedulesCancel | E::Resumes | E::ChangesPlan) => Some("subscribed"),
        (S::Active | S::PastDue, E::SchedulesCancel) => Some("subscription_cancelled"),
        (S::Active | S::Cancelling, E::PaymentFails) => Some("payment_failed"),
        (_, E::ChangesPlan) if live => Some("plan_changed"),
        (_, E::Ends) if live => Some("subscription_ended"),
        _ => None,
    }
}

impl<'a> Subscriber<'a> {
    async fn arrange(
        stripe: &'a FakeStripe,
        app: &App,
        browser: Browser,
        cell: usize,
        from: PlanState,
    ) -> Self {
        let user = active_user(&format!("subscriber-{cell}@example.test"), &app.db).await;
        let customer = customer_of(app, &user).await;
        let mut subscriber = Self {
            stripe,
            db: app.db.clone(),
            browser,
            user,
            customer,
            cell,
            truth: Truth {
                status: "active",
                cancel: false,
                price: "price_pro",
            },
            subscription: format!("sub_{cell}_0"),
            subscriptions: 0,
            sent: 0,
            last: None,
            status: StatusCode::OK,
        };
        let steps: &[StripeChange] = match from {
            S::Free => &[],
            S::Active => &[E::Subscribes],
            S::Cancelling => &[E::Subscribes, E::SchedulesCancel],
            S::PastDue => &[E::Subscribes, E::PaymentFails],
            S::PastDueCancelling => &[E::Subscribes, E::SchedulesCancel, E::PaymentFails],
            S::Ended => &[E::Subscribes, E::Ends],
        };
        for &step in steps {
            subscriber.apply(step).await;
        }
        assert_eq!(subscriber.state().await, from, "arranging cell {cell}");
        subscriber
    }

    async fn apply(&mut self, event: StripeChange) {
        match event {
            E::Subscribes => {
                self.subscriptions += 1;
                self.subscription = format!("sub_{}_{}", self.cell, self.subscriptions);
                self.truth = Truth {
                    status: "active",
                    cancel: false,
                    price: "price_pro",
                };
                self.changed("customer.subscription.created").await;
            }
            E::SchedulesCancel => {
                self.truth.cancel = true;
                self.changed("customer.subscription.updated").await;
            }
            E::Resumes => {
                self.truth.cancel = false;
                self.changed("customer.subscription.updated").await;
            }
            E::ChangesPlan => {
                self.truth.price = match self.truth.price {
                    "price_pro" => "price_business",
                    _ => "price_pro",
                };
                self.changed("customer.subscription.updated").await;
            }
            E::PaymentFails => {
                self.truth.status = "past_due";
                self.invoice("invoice.payment_failed").await;
            }
            E::Pays => {
                self.truth.status = "active";
                self.invoice("invoice.paid").await;
            }
            E::Ends => {
                self.truth.status = "canceled";
                self.changed("customer.subscription.deleted").await;
            }
            E::Replayed => {
                let body = self.last.clone().unwrap_or_default();
                self.status = self.deliver(&body, WEBHOOK_SECRET).await;
            }
            E::Late => {
                let body = self.event("invoice.payment_failed", self.invoice_object(), 0);
                self.status = self.deliver(&body, WEBHOOK_SECRET).await;
            }
            E::Forged => {
                self.sent += 1;
                let body = self.event("customer.subscription.deleted", self.object(), self.sent);
                self.status = self.deliver(&body, "whsec_someone_else").await;
            }
        }
    }

    async fn changed(&mut self, kind: &str) {
        let object = self.object();
        self.stripe
            .set_subscription(&self.subscription, object.clone());
        self.send(kind, object).await;
    }

    async fn invoice(&mut self, kind: &str) {
        self.stripe
            .set_subscription(&self.subscription, self.object());
        self.send(kind, self.invoice_object()).await;
    }

    async fn send(&mut self, kind: &str, object: Value) {
        self.sent += 1;
        let body = self.event(kind, object, self.sent);
        self.status = self.deliver(&body, WEBHOOK_SECRET).await;
        self.last = Some(body);
    }

    fn event(&self, kind: &str, object: Value, at: i64) -> String {
        let id = format!("evt_{}_{}_{at}", self.cell, self.sent);
        json!({ "id": id, "object": "event", "type": kind, "created": BASE_TIME + at, "data": { "object": object } })
            .to_string()
    }

    fn object(&self) -> Value {
        let cancel_at = if self.truth.cancel {
            json!(PERIOD_END)
        } else {
            Value::Null
        };
        json!({
            "id": self.subscription, "object": "subscription", "customer": self.customer,
            "status": self.truth.status, "cancel_at_period_end": self.truth.cancel, "cancel_at": cancel_at,
            "current_period_end": PERIOD_END,
            "items": { "data": [{ "price": { "id": self.truth.price }, "current_period_end": PERIOD_END }] },
        })
    }

    fn invoice_object(&self) -> Value {
        json!({
            "object": "invoice", "customer": self.customer,
            "parent": { "subscription_details": { "subscription": self.subscription } },
        })
    }

    async fn deliver(&mut self, body: &str, secret: &str) -> StatusCode {
        deliver_webhook(&mut self.browser, body, secret).await
    }

    async fn state(&self) -> PlanState {
        let rows = sqlx::query!(
            "SELECT status, ends_at IS NOT NULL AS \"scheduled!\" FROM subscriptions WHERE user_id = $1 ORDER BY status = 'ended', id DESC",
            self.user.id
        )
        .fetch_all(&self.db)
        .await
        .unwrap();
        let Some(row) = rows.first() else {
            return S::Free;
        };
        match (row.status.as_str(), row.scheduled) {
            ("active", _) => S::Active,
            ("cancelling", _) => S::Cancelling,
            ("past_due", false) => S::PastDue,
            ("past_due", true) => S::PastDueCancelling,
            ("ended", _) => S::Ended,
            (other, _) => panic!("unknown subscription status {other}"),
        }
    }

    async fn billing_events(&self) -> Vec<(String, String)> {
        sqlx::query!(
            "SELECT kind, channel FROM account_events WHERE user_id = $1 AND kind = ANY($2) ORDER BY id",
            self.user.id,
            &BILLING_KINDS.map(str::to_string)[..],
        )
        .fetch_all(&self.db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.kind, row.channel))
        .collect()
    }

    async fn broken_rules(
        &self,
        from: PlanState,
        event: StripeChange,
        before: &[(String, String)],
    ) -> Vec<&'static str> {
        let mut broken = Vec::new();
        let after = self.billing_events().await;
        let added: Vec<&(String, String)> = after.iter().skip(before.len()).collect();
        let wanted = recorded(from, event);
        let as_wanted = match wanted {
            None => added.is_empty(),
            Some(kind) => added.len() == 1 && added[0].0 == kind && added[0].1 == "stripe",
        };
        if !as_wanted {
            broken.push(EACH_CHANGE_IS_RECORDED);
        }
        if !self.one_live_subscription(from, event).await {
            broken.push(ONE_LIVE_SUBSCRIPTION);
        }
        let signed_answer = if event == E::Forged {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::OK
        };
        if self.status != signed_answer {
            broken.push(ONLY_SIGNED_EVENTS);
        }
        broken
    }

    async fn one_live_subscription(&self, from: PlanState, event: StripeChange) -> bool {
        let live = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM subscriptions WHERE user_id = $1 AND status <> 'ended'"#,
            self.user.id
        )
        .fetch_one(&self.db)
        .await
        .unwrap();
        let duplicate = event == E::Subscribes && !matches!(from, S::Free | S::Ended);
        if !duplicate {
            return live <= 1;
        }
        let refund = json!({ "subscription": self.subscription, "refund": true });
        let queued = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM jobs WHERE kind = $1 AND payload = $2"#,
            CancelStripeSubscription::KIND,
            refund,
        )
        .fetch_one(&self.db)
        .await
        .unwrap();
        let stored = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM subscriptions WHERE stripe_subscription_id = $1"#,
            self.subscription
        )
        .fetch_one(&self.db)
        .await
        .unwrap();
        live == 1 && queued == 1 && stored == 0
    }
}

const BILLING_KINDS: [&str; 6] = [
    "subscribed",
    "subscription_cancelled",
    "payment_failed",
    "plan_changed",
    "subscription_ended",
    "duplicate_subscription_refunded",
];

async fn customer_of(app: &App, user: &Arc<User>) -> String {
    let caller = Caller::browser(user.clone());
    let input = StartCheckout {
        price: "price_pro".to_string(),
    };
    start_checkout(&app.env(None), &caller, &input)
        .await
        .unwrap()
        .unwrap();
    sqlx::query_scalar!(
        "SELECT stripe_customer_id FROM billing_customers WHERE user_id = $1",
        user.id
    )
    .fetch_one(&app.db)
    .await
    .unwrap()
}
