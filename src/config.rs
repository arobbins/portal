use std::env;
use std::path::PathBuf;
use std::time::Duration;

use crate::mail::Mailer;
use crate::rate_limit::RateLimitMode;

const MAIL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct Config {
    pub app_base_url: String,
    pub bind_address: String,
    pub mailer: Mailer,
    pub rate_limit_mode: RateLimitMode,
    pub trusted_proxy_hops: Option<usize>,
}

impl Config {
    pub fn from_env() -> Result<Self, Vec<String>> {
        Self::from_lookup(|key| env::var(key).ok().filter(|value| !value.is_empty()))
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, Vec<String>> {
        let mut problems = Vec::new();
        let mut require = |key: &str, why: &str| {
            get(key).unwrap_or_else(|| {
                problems.push(format!("{key} must be set {why}"));
                String::new()
            })
        };

        let app_base_url = require("APP_BASE_URL", "(e.g. http://127.0.0.1:3000)");

        let mailer = match get("MAIL_TRANSPORT").as_deref() {
            None | Some("postmark") => Mailer::Postmark {
                client: reqwest::Client::builder()
                    .timeout(MAIL_TIMEOUT)
                    .build()
                    .expect("the HTTP client's TLS backend initializes"),
                server_token: require("POSTMARK_SERVER_TOKEN", "when MAIL_TRANSPORT=postmark"),
                base_url: require("POSTMARK_BASE_URL", "when MAIL_TRANSPORT=postmark"),
                from: require("POSTMARK_FROM_EMAIL", "when MAIL_TRANSPORT=postmark"),
            },
            Some("log") => Mailer::Log,
            Some("file") => Mailer::File {
                dir: PathBuf::from(require("MAIL_DIR", "when MAIL_TRANSPORT=file")),
            },
            Some(other) => {
                problems.push(format!(
                    "MAIL_TRANSPORT must be postmark, log or file (got `{other}`)"
                ));
                Mailer::Log
            }
        };

        let rate_limit_mode = match get("RATE_LIMITS").as_deref() {
            None | Some("strict") => RateLimitMode::Strict,
            Some("relaxed") => RateLimitMode::Relaxed,
            Some(other) => {
                problems.push(format!(
                    "RATE_LIMITS must be strict or relaxed (got `{other}`)"
                ));
                RateLimitMode::Strict
            }
        };

        let trusted_proxy_hops = get("TRUSTED_PROXY_HOPS").and_then(|hops| {
            hops.parse::<usize>().ok().or_else(|| {
                problems.push(format!(
                    "TRUSTED_PROXY_HOPS must be a whole number (got `{hops}`)"
                ));
                None
            })
        });

        let host = get("HOST").unwrap_or_else(|| "127.0.0.1".to_string());
        let port = get("PORT").unwrap_or_else(|| "3000".to_string());
        if port.parse::<u16>().is_err() {
            problems.push(format!("PORT must be a port number (got `{port}`)"));
        }

        if problems.is_empty() {
            Ok(Self {
                app_base_url: app_base_url.trim_end_matches('/').to_string(),
                bind_address: format!("{host}:{port}"),
                mailer,
                rate_limit_mode,
                trusted_proxy_hops,
            })
        } else {
            Err(problems)
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.app_base_url)
    }

    pub fn secure_cookies(&self) -> bool {
        self.app_base_url.starts_with("https://")
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
            ("APP_BASE_URL", "http://127.0.0.1:3000/"),
            ("MAIL_TRANSPORT", "log"),
        ])
        .unwrap();

        assert_eq!(config.url("/login"), "http://127.0.0.1:3000/login");
        assert_eq!(config.bind_address, "127.0.0.1:3000");
        assert_eq!(config.rate_limit_mode, RateLimitMode::Strict);
        assert!(!config.secure_cookies());
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
