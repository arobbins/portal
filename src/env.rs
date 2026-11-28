use std::net::IpAddr;
use std::sync::Arc;

use sqlx::PgPool;
use time::OffsetDateTime;
use webauthn_rs::prelude::Webauthn;

use crate::config::Config;
use crate::rate_limit::RateLimits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    System,
    #[cfg(test)]
    Fixed(OffsetDateTime),
}

impl Clock {
    pub fn now(self) -> OffsetDateTime {
        match self {
            Self::System => OffsetDateTime::now_utc(),
            #[cfg(test)]
            Self::Fixed(at) => at,
        }
    }
}

#[derive(Clone)]
pub struct App {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub limits: Arc<RateLimits>,
    pub webauthn: Arc<Webauthn>,
    pub clock: Clock,
}

impl App {
    pub fn url(&self, path: &str) -> String {
        self.config.url(path)
    }

    pub fn env(&self, ip: Option<IpAddr>) -> Env<'_> {
        Env {
            app: self,
            ip,
            idempotency_key: None,
        }
    }

    pub fn new(db: PgPool, config: Config, webauthn: Webauthn) -> Self {
        Self {
            limits: Arc::new(RateLimits::new(db.clone(), config.rate_limit_mode)),
            webauthn: Arc::new(webauthn),
            config: Arc::new(config),
            clock: Clock::System,
            db,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Env<'a> {
    pub app: &'a App,
    pub ip: Option<IpAddr>,
    pub idempotency_key: Option<&'a str>,
}

impl<'a> Env<'a> {
    pub fn with_idempotency_key(self, key: Option<&'a str>) -> Self {
        Self {
            idempotency_key: key,
            ..self
        }
    }

    pub fn db(&self) -> &PgPool {
        &self.app.db
    }

    pub fn limits(&self) -> &RateLimits {
        &self.app.limits
    }

    pub fn now(&self) -> OffsetDateTime {
        self.app.clock.now()
    }

    pub fn stripe(&self) -> Option<&'a crate::stripe::Stripe> {
        self.app.config.stripe.as_ref()
    }
}

pub fn is_within(stamped_at: i64, now: OffsetDateTime, window: time::Duration) -> bool {
    (0..=window.whole_seconds()).contains(&(now.unix_timestamp() - stamped_at))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_is_within_its_window_from_zero_to_the_end_inclusive() {
        let now = OffsetDateTime::from_unix_timestamp(1_000).unwrap();
        let window = time::Duration::seconds(60);

        assert!(is_within(1_000, now, window));
        assert!(is_within(940, now, window));
        assert!(!is_within(939, now, window));
        assert!(!is_within(1_001, now, window));
    }

    #[test]
    fn a_fixed_clock_never_moves() {
        let at = OffsetDateTime::from_unix_timestamp(1_000).unwrap();

        assert_eq!(Clock::Fixed(at).now(), at);
        assert_eq!(Clock::Fixed(at).now(), Clock::Fixed(at).now());
    }
}
