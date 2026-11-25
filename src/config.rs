use std::env;
use std::path::PathBuf;
use std::time::Duration;

use url::{Host, Url};

use crate::mail::Mailer;
use crate::rate_limit::RateLimitMode;
use crate::stripe::{DEFAULT_API_BASE, Stripe};

const MAIL_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_JOB_WORKERS: usize = 2;

struct Reader<'a> {
    get: &'a dyn Fn(&str) -> Option<String>,
    problems: Vec<String>,
}

impl Reader<'_> {
    fn optional(&self, key: &str) -> Option<String> {
        (self.get)(key)
    }

    fn require(&mut self, key: &str, why: &str) -> String {
        self.optional(key).unwrap_or_else(|| {
            self.problems.push(format!("{key} must be set {why}"));
            String::new()
        })
    }

    fn optional_url(&mut self, key: &str) -> Option<String> {
        let raw = self.optional(key)?;
        match Url::parse(&raw) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => Some(raw),
            _ => {
                self.problems
                    .push(format!("{key} must be a full http(s) URL (got `{raw}`)"));
                None
            }
        }
    }

    fn parse_optional<T: std::str::FromStr>(&mut self, key: &str, what: &str) -> Option<T> {
        let raw = self.optional(key)?;
        let parsed = raw.parse::<T>().ok();
        if parsed.is_none() {
            self.problems
                .push(format!("{key} must be {what} (got `{raw}`)"));
        }
        parsed
    }
}

fn parse_mailer(reader: &mut Reader<'_>) -> Mailer {
    match reader.optional("MAIL_TRANSPORT").as_deref() {
        None | Some("postmark") => postmark_mailer(reader),
        Some("log") => Mailer::Log,
        Some("file") => Mailer::File {
            dir: PathBuf::from(reader.require("MAIL_DIR", "when MAIL_TRANSPORT=file")),
        },
        Some(other) => {
            reader.problems.push(format!(
                "MAIL_TRANSPORT must be postmark, log or file (got `{other}`)"
            ));
            Mailer::Log
        }
    }
}

fn postmark_mailer(reader: &mut Reader<'_>) -> Mailer {
    let why = "when MAIL_TRANSPORT=postmark";
    Mailer::Postmark {
        client: reqwest::Client::builder()
            .timeout(MAIL_TIMEOUT)
            .build()
            .expect("the HTTP client's TLS backend initializes"),
        server_token: reader.require("POSTMARK_SERVER_TOKEN", why),
        base_url: reader.require("POSTMARK_BASE_URL", why),
        from: reader.require("POSTMARK_FROM_EMAIL", why),
    }
}

fn parse_stripe(reader: &mut Reader<'_>) -> Option<Stripe> {
    let secret_key = reader.optional("STRIPE_SECRET_KEY");
    let webhook_secret = reader.optional("STRIPE_WEBHOOK_SECRET");
    let api_base = reader
        .optional_url("STRIPE_API_BASE")
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());
    match (secret_key, webhook_secret) {
        (Some(secret_key), Some(webhook_secret)) => {
            Some(Stripe::new(secret_key, webhook_secret, api_base))
        }
        (None, None) => None,
        _ => {
            reader.problems.push(
                "STRIPE_SECRET_KEY and STRIPE_WEBHOOK_SECRET must be set together (leave both empty to turn billing off)".to_string(),
            );
            None
        }
    }
}

fn parse_rate_limit_mode(reader: &mut Reader<'_>) -> RateLimitMode {
    match reader.optional("RATE_LIMITS").as_deref() {
        None | Some("strict") => RateLimitMode::Strict,
        Some("relaxed") => RateLimitMode::Relaxed,
        Some(other) => {
            reader.problems.push(format!(
                "RATE_LIMITS must be strict or relaxed (got `{other}`)"
            ));
            RateLimitMode::Strict
        }
    }
}

fn parse_passkey_domain(reader: &mut Reader<'_>, app_base_url: &str) -> String {
    passkey_domain(app_base_url).unwrap_or_else(|problem| {
        if !app_base_url.is_empty() {
            reader.problems.push(problem);
        }
        String::new()
    })
}

fn parse_bind_address(reader: &mut Reader<'_>) -> String {
    let host = reader
        .optional("HOST")
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = reader
        .optional("PORT")
        .unwrap_or_else(|| "3000".to_string());
    if port.parse::<u16>().is_err() {
        reader
            .problems
            .push(format!("PORT must be a port number (got `{port}`)"));
    }
    format!("{host}:{port}")
}

#[derive(Debug, Clone)]
pub struct Config {
    pub app_base_url: String,
    pub passkey_domain: String,
    pub bind_address: String,
    pub mailer: Mailer,
    pub rate_limit_mode: RateLimitMode,
    pub trusted_proxy_hops: Option<usize>,
    pub job_workers: usize,
    pub support_url: Option<String>,
    pub status_url: Option<String>,
    pub stripe: Option<Stripe>,
}

impl Config {
    pub fn from_env() -> Result<Self, Vec<String>> {
        Self::from_lookup(|key| env::var(key).ok().filter(|value| !value.is_empty()))
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, Vec<String>> {
        let mut reader = Reader {
            get: &get,
            problems: Vec::new(),
        };
        let app_base_url = reader.require("APP_BASE_URL", "(e.g. http://localhost:3000)");
        let config = Self {
            passkey_domain: parse_passkey_domain(&mut reader, &app_base_url),
            app_base_url: app_base_url.trim_end_matches('/').to_string(),
            bind_address: parse_bind_address(&mut reader),
            mailer: parse_mailer(&mut reader),
            rate_limit_mode: parse_rate_limit_mode(&mut reader),
            trusted_proxy_hops: reader.parse_optional("TRUSTED_PROXY_HOPS", "a whole number"),
            job_workers: reader
                .parse_optional("JOB_WORKERS", "a whole number")
                .unwrap_or(DEFAULT_JOB_WORKERS),
            support_url: reader.optional_url("SUPPORT_URL"),
            status_url: reader.optional_url("STATUS_URL"),
            stripe: parse_stripe(&mut reader),
        };

        if reader.problems.is_empty() {
            Ok(config)
        } else {
            Err(reader.problems)
        }
    }

    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self::from_lookup(|key| match key {
            "APP_BASE_URL" => Some("http://localhost".to_string()),
            "MAIL_TRANSPORT" => Some("log".to_string()),
            _ => None,
        })
        .expect("the test configuration is valid")
    }

    pub fn origin(&self) -> &str {
        &self.app_base_url
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.app_base_url)
    }

    pub fn secure_cookies(&self) -> bool {
        self.app_base_url.starts_with("https://")
    }
}

fn passkey_domain(app_base_url: &str) -> Result<String, String> {
    let url = Url::parse(app_base_url)
        .map_err(|e| format!("APP_BASE_URL must be a full URL (got `{app_base_url}`: {e})"))?;

    match url.host() {
        Some(Host::Domain(domain)) => Ok(domain.to_string()),
        _ => Err(format!(
            "APP_BASE_URL must use a domain name such as localhost, not an IP address (got `{app_base_url}`); passkeys don't work on IP addresses"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(pairs: &[(&str, &str)]) -> Result<Config, Vec<String>> {
        let values: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        Config::from_lookup(|key| values.get(key).cloned())
    }

    #[test]
    fn a_minimal_local_setup_works() {
        let config = config(&[
            ("APP_BASE_URL", "http://localhost:3000/"),
            ("MAIL_TRANSPORT", "log"),
        ])
        .unwrap();

        assert_eq!(config.url("/login"), "http://localhost:3000/login");
        assert_eq!(config.bind_address, "127.0.0.1:3000");
        assert_eq!(config.rate_limit_mode, RateLimitMode::Strict);
        assert!(!config.secure_cookies());
    }

    #[test]
    fn passkeys_need_a_domain_name() {
        let config = config(&[
            ("APP_BASE_URL", "http://localhost:3000"),
            ("MAIL_TRANSPORT", "log"),
        ])
        .unwrap();
        assert_eq!(config.passkey_domain, "localhost");

        let problems = config_problems(&[
            ("APP_BASE_URL", "http://127.0.0.1:3000"),
            ("MAIL_TRANSPORT", "log"),
        ]);
        assert!(problems[0].contains("domain name"), "{problems:?}");
    }

    fn config_problems(pairs: &[(&str, &str)]) -> Vec<String> {
        config(pairs).unwrap_err()
    }

    #[test]
    fn https_sites_get_secure_cookies() {
        let config = config(&[
            ("APP_BASE_URL", "https://example.com"),
            ("MAIL_TRANSPORT", "log"),
        ])
        .unwrap();

        assert!(config.secure_cookies());
    }

    #[test]
    fn support_and_status_links_are_optional_full_urls() {
        let local = [
            ("APP_BASE_URL", "http://localhost"),
            ("MAIL_TRANSPORT", "log"),
        ];
        let unset = config(&local).unwrap();
        assert_eq!(unset.support_url, None);
        assert_eq!(unset.status_url, None);

        let set = config(&[
            local[0],
            local[1],
            ("SUPPORT_URL", "https://help.example.com"),
            ("STATUS_URL", "https://status.example.com/"),
        ])
        .unwrap();
        assert_eq!(set.support_url.as_deref(), Some("https://help.example.com"));
        assert_eq!(
            set.status_url.as_deref(),
            Some("https://status.example.com/")
        );

        let problems = config_problems(&[
            local[0],
            local[1],
            ("SUPPORT_URL", "/support"),
            ("STATUS_URL", "mailto:ops@example.com"),
        ]);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].starts_with("SUPPORT_URL"), "{problems:?}");
    }

    #[test]
    fn billing_is_off_without_stripe_keys_and_needs_both() {
        let local = [
            ("APP_BASE_URL", "http://localhost"),
            ("MAIL_TRANSPORT", "log"),
        ];
        assert!(config(&local).unwrap().stripe.is_none());

        let set = config(&[
            local[0],
            local[1],
            ("STRIPE_SECRET_KEY", "sk_test_abc"),
            ("STRIPE_WEBHOOK_SECRET", "whsec_abc"),
        ])
        .unwrap();
        let stripe = set.stripe.unwrap();
        assert!(stripe.is_test_mode());
        assert_eq!(stripe.webhook_secret(), "whsec_abc");

        let problems = config_problems(&[local[0], local[1], ("STRIPE_SECRET_KEY", "sk_live_abc")]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("STRIPE_SECRET_KEY"), "{problems:?}");
    }

    #[test]
    fn every_problem_is_reported_at_once() {
        let problems = config(&[
            ("MAIL_TRANSPORT", "carrier-pigeon"),
            ("RATE_LIMITS", "relax"),
            ("TRUSTED_PROXY_HOPS", "one"),
            ("PORT", "http"),
        ])
        .unwrap_err();

        assert_eq!(problems.len(), 5, "{problems:?}");
    }

    #[test]
    fn postmark_is_the_default_and_needs_its_settings() {
        let problems = config(&[("APP_BASE_URL", "http://localhost")]).unwrap_err();

        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(
            problems
                .iter()
                .all(|problem| problem.starts_with("POSTMARK_"))
        );
    }
}
