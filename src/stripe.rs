mod signature;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::{Method, StatusCode};
use serde_json::Value;

#[cfg(test)]
pub use signature::sign;
pub use signature::verify_signature;

const STRIPE_TIMEOUT: Duration = Duration::from_secs(10);
const CACHE_FOR: Duration = Duration::from_secs(60);
pub const DEFAULT_API_BASE: &str = "https://api.stripe.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StripeError {
    Unreachable(String),
    Rejected {
        status: u16,
        code: Option<String>,
        message: String,
    },
}

impl StripeError {
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Rejected { status: 404, .. })
    }

    pub fn has_code(&self, wanted: &str) -> bool {
        matches!(self, Self::Rejected { code: Some(code), .. } if code == wanted)
    }
}

impl std::fmt::Display for StripeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(reason) => write!(f, "Stripe can't be reached: {reason}"),
            Self::Rejected {
                status, message, ..
            } => {
                write!(f, "Stripe refused the request ({status}): {message}")
            }
        }
    }
}

impl std::error::Error for StripeError {}

type Cache = Arc<Mutex<HashMap<String, (Instant, Value)>>>;

#[derive(Clone)]
pub struct Stripe {
    client: reqwest::Client,
    secret_key: String,
    webhook_secret: String,
    api_base: String,
    cache: Cache,
}

impl std::fmt::Debug for Stripe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stripe")
            .field("api_base", &self.api_base)
            .field("test_mode", &self.is_test_mode())
            .finish_non_exhaustive()
    }
}

pub struct Request<'a> {
    pub method: Method,
    pub path: &'a str,
    pub form: &'a [(&'a str, &'a str)],
    pub idempotency_key: Option<&'a str>,
}

impl<'a> Request<'a> {
    pub fn get(path: &'a str) -> Self {
        Self {
            method: Method::GET,
            path,
            form: &[],
            idempotency_key: None,
        }
    }

    pub fn post(path: &'a str, form: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            method: Method::POST,
            path,
            form,
            idempotency_key: None,
        }
    }

    pub fn delete(path: &'a str) -> Self {
        Self {
            method: Method::DELETE,
            ..Self::get(path)
        }
    }

    pub fn idempotent(self, key: Option<&'a str>) -> Self {
        Self {
            idempotency_key: key,
            ..self
        }
    }
}

impl Stripe {
    pub fn new(secret_key: String, webhook_secret: String, api_base: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(STRIPE_TIMEOUT)
                .build()
                .expect("the HTTP client's TLS backend initializes"),
            secret_key,
            webhook_secret,
            api_base: api_base.trim_end_matches('/').to_string(),
            cache: Cache::default(),
        }
    }

    pub fn is_test_mode(&self) -> bool {
        self.secret_key.starts_with("sk_test_") || self.secret_key.starts_with("rk_test_")
    }

    pub fn webhook_secret(&self) -> &str {
        &self.webhook_secret
    }

    pub async fn send(&self, request: Request<'_>) -> Result<Value, StripeError> {
        let url = format!("{}{}", self.api_base, request.path);
        let mut builder = self
            .client
            .request(request.method.clone(), url)
            .bearer_auth(&self.secret_key);
        if request.method != Method::GET {
            builder = builder.form(request.form);
        }
        if let Some(key) = request.idempotency_key {
            builder = builder.header("Idempotency-Key", key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| StripeError::Unreachable(error.without_url().to_string()))?;
        read_response(response).await
    }

    pub async fn get_cached(&self, path: &str) -> Result<Value, StripeError> {
        if let Some(value) = self.cached(path) {
            return Ok(value);
        }
        let value = self.send(Request::get(path)).await?;
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path.to_string(), (Instant::now(), value.clone()));
        }
        Ok(value)
    }

    fn cached(&self, path: &str) -> Option<Value> {
        let cache = self.cache.lock().ok()?;
        let (at, value) = cache.get(path)?;
        (at.elapsed() < CACHE_FOR).then(|| value.clone())
    }
}

async fn read_response(response: reqwest::Response) -> Result<Value, StripeError> {
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|error| StripeError::Unreachable(error.without_url().to_string()))?;
    if status.is_success() {
        return Ok(body);
    }
    if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
        return Err(StripeError::Unreachable(format!("status {status}")));
    }
    let message = body["error"]["message"]
        .as_str()
        .unwrap_or("no message")
        .to_string();
    let code = body["error"]["code"].as_str().map(str::to_string);
    Err(StripeError::Rejected {
        status: status.as_u16(),
        code,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_shows_the_keys() {
        let stripe = Stripe::new(
            "sk_test_secret123".to_string(),
            "whsec_secret456".to_string(),
            "http://127.0.0.1:1/".to_string(),
        );
        let shown = format!("{stripe:?}");

        assert!(!shown.contains("secret123"), "{shown}");
        assert!(!shown.contains("secret456"), "{shown}");
        assert!(stripe.is_test_mode());
    }

    #[tokio::test]
    async fn an_unreachable_stripe_is_reported_as_unreachable() {
        let stripe = Stripe::new(
            "sk_test_x".to_string(),
            "whsec_x".to_string(),
            "http://127.0.0.1:9".to_string(),
        );

        let error = stripe.send(Request::get("/v1/prices")).await.unwrap_err();

        assert!(matches!(error, StripeError::Unreachable(_)), "{error}");
        assert!(!error.to_string().contains("sk_test_x"));
    }
}
