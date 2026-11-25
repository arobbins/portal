use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Json;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::config::Config;
use crate::env::App;

pub const SECRET_KEY: &str = "sk_test_rust_tests";
pub const WEBHOOK_SECRET: &str = "whsec_rust_tests";

#[derive(Debug, Clone)]
pub struct Seen {
    pub method: String,
    pub path: String,
    pub idempotency_key: Option<String>,
    pub form: Vec<(String, String)>,
}

impl Seen {
    pub fn field(&self, name: &str) -> Option<&str> {
        self.form
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Default)]
struct Fake {
    seen: Mutex<Vec<Seen>>,
    sessions: AtomicUsize,
    customers: AtomicUsize,
    failing_deletes: AtomicUsize,
    subscriptions: Mutex<HashMap<String, Value>>,
}

pub struct FakeStripe {
    base: String,
    fake: Arc<Fake>,
}

pub fn config_for(base: &str) -> Config {
    Config::from_lookup(|key| match key {
        "APP_BASE_URL" => Some("http://localhost".to_string()),
        "MAIL_TRANSPORT" => Some("log".to_string()),
        "STRIPE_SECRET_KEY" => Some(SECRET_KEY.to_string()),
        "STRIPE_WEBHOOK_SECRET" => Some(WEBHOOK_SECRET.to_string()),
        "STRIPE_API_BASE" => Some(base.to_string()),
        _ => None,
    })
    .expect("the test configuration is valid")
}

pub fn app_with_stripe_at(base: &str, db: &PgPool) -> App {
    App {
        config: Arc::new(config_for(base)),
        ..App::for_tests(db.clone())
    }
}

pub fn unreachable_stripe(db: &PgPool) -> App {
    app_with_stripe_at("http://127.0.0.1:9", db)
}

impl FakeStripe {
    pub async fn start() -> Self {
        let fake = Arc::new(Fake::default());
        let router = Router::new().fallback(respond).with_state(fake.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, router).await });
        Self { base, fake }
    }

    pub fn app(&self, db: &PgPool) -> App {
        app_with_stripe_at(&self.base, db)
    }

    pub fn seen(&self) -> Vec<Seen> {
        self.fake.seen.lock().unwrap().clone()
    }

    pub fn seen_at(&self, method: &str, path: &str) -> Vec<Seen> {
        self.seen()
            .into_iter()
            .filter(|seen| seen.method == method && seen.path == path)
            .collect()
    }

    pub fn set_subscription(&self, id: &str, object: Value) {
        self.fake
            .subscriptions
            .lock()
            .unwrap()
            .insert(id.to_string(), object);
    }

    pub fn fail_next_deletes(&self, count: usize) {
        self.fake.failing_deletes.store(count, Ordering::SeqCst);
    }
}

fn price(id: &str, amount: i64, name: &str, key: &str) -> Value {
    json!({
        "id": id, "object": "price", "active": true, "type": "recurring", "currency": "usd",
        "unit_amount": amount, "recurring": { "interval": "month", "interval_count": 1 },
        "product": { "id": format!("prod_{key}"), "active": true, "name": name, "metadata": { "plan_key": key } },
    })
}

async fn respond(
    State(fake): State<Arc<Fake>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: String,
) -> (StatusCode, Json<Value>) {
    let path = uri.path().to_string();
    fake.seen.lock().unwrap().push(Seen {
        method: method.to_string(),
        path: path.clone(),
        idempotency_key: headers
            .get("idempotency-key")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
        form: url::form_urlencoded::parse(body.as_bytes())
            .into_owned()
            .collect(),
    });
    match (method.as_str(), path.as_str()) {
        ("GET", "/v1/prices") => (
            StatusCode::OK,
            Json(json!({ "object": "list", "data": [
                price("price_business", 4900, "Business", "business"),
                price("price_pro", 1200, "Pro", "pro"),
            ] })),
        ),
        ("POST", "/v1/customers") => {
            let n = fake.customers.fetch_add(1, Ordering::SeqCst) + 1;
            (
                StatusCode::OK,
                Json(json!({ "id": format!("cus_fake{n}") })),
            )
        }
        ("POST", "/v1/checkout/sessions") => {
            let n = fake.sessions.fetch_add(1, Ordering::SeqCst) + 1;
            let url = format!("https://checkout.test/cs_{n}");
            (
                StatusCode::OK,
                Json(json!({ "id": format!("cs_{n}"), "url": url })),
            )
        }
        ("GET", "/v1/invoices") => (
            StatusCode::OK,
            Json(json!({ "object": "list", "data": [
                { "id": "in_fake", "amount_paid": 1200, "payment_intent": "pi_fake" }
            ] })),
        ),
        ("POST", "/v1/refunds") => (StatusCode::OK, Json(json!({ "id": "re_fake" }))),
        ("GET", _) if path.starts_with("/v1/subscriptions/") => {
            let id = path.trim_start_matches("/v1/subscriptions/");
            match fake.subscriptions.lock().unwrap().get(id) {
                Some(object) => (StatusCode::OK, Json(object.clone())),
                None => (StatusCode::NOT_FOUND, Json(json!({ "error": {} }))),
            }
        }
        ("POST", _) if path.starts_with("/v1/customers/") => (
            StatusCode::OK,
            Json(json!({ "id": path.trim_start_matches("/v1/customers/") })),
        ),
        ("POST", "/v1/billing_portal/sessions") => (
            StatusCode::OK,
            Json(json!({ "url": "https://portal.test/bps_1" })),
        ),
        ("DELETE", _) if path.starts_with("/v1/subscriptions/") => {
            let failing = fake.failing_deletes.load(Ordering::SeqCst);
            if failing > 0 {
                fake.failing_deletes.store(failing - 1, Ordering::SeqCst);
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({ "error": {} })),
                );
            }
            (StatusCode::OK, Json(json!({ "status": "canceled" })))
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": { "message": "No such resource" } })),
        ),
    }
}
