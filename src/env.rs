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
        Env { app: self, ip }
    }

    #[cfg(test)]
    pub fn for_tests(db: PgPool) -> Self {
        let config = Config::for_tests();
        Self {
            limits: Arc::new(RateLimits::new(
                db.clone(),
                crate::rate_limit::RateLimitMode::Strict,
            )),
            webauthn: Arc::new(crate::passkeys::webauthn::relying_party(&config)),
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
}

impl Env<'_> {
    pub fn db(&self) -> &PgPool {
        &self.app.db
    }

    pub fn limits(&self) -> &RateLimits {
        &self.app.limits
    }

    pub fn now(&self) -> OffsetDateTime {
        self.app.clock.now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_clock_never_moves() {
        let at = OffsetDateTime::from_unix_timestamp(1_000).unwrap();

        assert_eq!(Clock::Fixed(at).now(), at);
        assert_eq!(Clock::Fixed(at).now(), Clock::Fixed(at).now());
    }
}
