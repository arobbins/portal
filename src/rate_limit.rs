use std::collections::HashMap;
use std::env;
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU32;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};

const UNKNOWN_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const RELAXED_SCALE: u32 = 1000;

struct FailureWindow<K> {
    window: Duration,
    max_failures: usize,
    failures: Mutex<HashMap<K, Vec<Instant>>>,
}

impl<K: Eq + Hash> FailureWindow<K> {
    fn new(window: Duration, max_failures: usize) -> Self {
        Self {
            window,
            max_failures,
            failures: Mutex::new(HashMap::new()),
        }
    }

    fn with_recent<T>(&self, key: K, read: impl FnOnce(&mut Vec<Instant>) -> T) -> T {
        let mut failures = self
            .failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let recent = failures.entry(key).or_default();
        recent.retain(|at| at.elapsed() < self.window);
        read(recent)
    }

    fn is_blocked(&self, key: K) -> bool {
        self.with_recent(key, |recent| recent.len() >= self.max_failures)
    }

    fn record(&self, key: K) {
        self.with_recent(key, |recent| recent.push(Instant::now()));
    }

    fn clear(&self, key: &K) {
        self.failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(key);
    }

    fn forget_idle(&self) {
        let mut failures = self
            .failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        failures.retain(|_, recent| {
            recent.retain(|at| at.elapsed() < self.window);
            !recent.is_empty()
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitMode {
    Strict,
    Relaxed,
}

pub struct RateLimits {
    logins_by_ip: DefaultKeyedRateLimiter<IpAddr>,
    signups_by_ip: DefaultKeyedRateLimiter<IpAddr>,
    emails_by_address: DefaultKeyedRateLimiter<String>,
    failed_logins_by_account_and_ip: FailureWindow<(String, IpAddr)>,
    failed_logins_by_account: FailureWindow<String>,
}

fn account_key(email: &str) -> String {
    email.trim().to_lowercase()
}

fn quota(refill_every: Duration, burst: u32) -> Quota {
    Quota::with_period(refill_every)
        .expect("refill period must be non-zero")
        .allow_burst(NonZeroU32::new(burst).expect("burst must be non-zero"))
}

impl RateLimits {
    pub fn new(mode: RateLimitMode) -> Self {
        let scale = match mode {
            RateLimitMode::Strict => 1,
            RateLimitMode::Relaxed => RELAXED_SCALE,
        };

        Self {
            logins_by_ip: RateLimiter::keyed(quota(Duration::from_secs(6), 10 * scale)),
            signups_by_ip: RateLimiter::keyed(quota(Duration::from_secs(120), 5 * scale)),
            emails_by_address: RateLimiter::keyed(quota(Duration::from_secs(1200), 3 * scale)),
            failed_logins_by_account_and_ip: FailureWindow::new(Duration::from_secs(15 * 60), 5),
            failed_logins_by_account: FailureWindow::new(Duration::from_secs(60 * 60), 50),
        }
    }

    pub fn from_env() -> Self {
        match env::var("RATE_LIMITS").as_deref() {
            Ok("relaxed") => Self::new(RateLimitMode::Relaxed),
            _ => Self::new(RateLimitMode::Strict),
        }
    }

    pub fn allow_login(&self, ip: Option<IpAddr>) -> bool {
        self.logins_by_ip
            .check_key(&ip.unwrap_or(UNKNOWN_IP))
            .is_ok()
    }

    pub fn allow_signup(&self, ip: Option<IpAddr>) -> bool {
        self.signups_by_ip
            .check_key(&ip.unwrap_or(UNKNOWN_IP))
            .is_ok()
    }

    pub fn allow_email_to(&self, address: &str) -> bool {
        self.emails_by_address
            .check_key(&account_key(address))
            .is_ok()
    }

    pub fn login_blocked(&self, email: &str, ip: Option<IpAddr>) -> bool {
        let account = account_key(email);
        let ip = ip.unwrap_or(UNKNOWN_IP);

        self.failed_logins_by_account_and_ip
            .is_blocked((account.clone(), ip))
            || self.failed_logins_by_account.is_blocked(account)
    }

    pub fn record_failed_login(&self, email: &str, ip: Option<IpAddr>) {
        let account = account_key(email);
        let ip = ip.unwrap_or(UNKNOWN_IP);

        self.failed_logins_by_account_and_ip
            .record((account.clone(), ip));
        self.failed_logins_by_account.record(account);
    }

    pub fn clear_failed_logins(&self, email: &str, ip: Option<IpAddr>) {
        self.failed_logins_by_account_and_ip
            .clear(&(account_key(email), ip.unwrap_or(UNKNOWN_IP)));
    }

    pub fn forget_idle_keys(&self) {
        self.logins_by_ip.retain_recent();
        self.signups_by_ip.retain_recent();
        self.emails_by_address.retain_recent();
        self.failed_logins_by_account_and_ip.forget_idle();
        self.failed_logins_by_account.forget_idle();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(last: u8) -> Option<IpAddr> {
        Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, last)))
    }

    #[test]
    fn logins_are_limited_per_ip() {
        let limits = RateLimits::new(RateLimitMode::Strict);

        for _ in 0..10 {
            assert!(limits.allow_login(ip(1)));
        }
        assert!(!limits.allow_login(ip(1)));
        assert!(limits.allow_login(ip(2)));
    }

    #[test]
    fn signups_are_limited_per_ip() {
        let limits = RateLimits::new(RateLimitMode::Strict);

        for _ in 0..5 {
            assert!(limits.allow_signup(ip(1)));
        }
        assert!(!limits.allow_signup(ip(1)));
        assert!(limits.allow_signup(ip(2)));
    }

    #[test]
    fn emails_are_limited_per_address_regardless_of_case() {
        let limits = RateLimits::new(RateLimitMode::Strict);

        for _ in 0..3 {
            assert!(limits.allow_email_to("victim@example.test"));
        }
        assert!(!limits.allow_email_to("Victim@Example.test"));
        assert!(limits.allow_email_to("someone-else@example.test"));
    }

    #[test]
    fn five_failed_logins_block_that_account_from_that_ip() {
        let limits = RateLimits::new(RateLimitMode::Strict);

        for _ in 0..5 {
            assert!(!limits.login_blocked("owner@example.test", ip(1)));
            limits.record_failed_login("owner@example.test", ip(1));
        }

        assert!(limits.login_blocked("Owner@Example.test", ip(1)));
        assert!(!limits.login_blocked("owner@example.test", ip(2)));
        assert!(!limits.login_blocked("someone-else@example.test", ip(1)));
    }

    #[test]
    fn a_successful_login_clears_the_failures() {
        let limits = RateLimits::new(RateLimitMode::Strict);

        for _ in 0..4 {
            limits.record_failed_login("owner@example.test", ip(1));
        }
        limits.clear_failed_logins("owner@example.test", ip(1));
        limits.record_failed_login("owner@example.test", ip(1));

        assert!(!limits.login_blocked("owner@example.test", ip(1)));
    }

    #[test]
    fn many_failures_from_many_ips_block_the_account_everywhere() {
        let limits = RateLimits::new(RateLimitMode::Strict);

        for attacker in 0..50 {
            limits.record_failed_login("owner@example.test", ip(attacker));
        }

        assert!(limits.login_blocked("owner@example.test", ip(200)));
    }

    #[test]
    fn failures_expire_after_the_window() {
        let window = FailureWindow::new(Duration::from_millis(50), 1);
        window.record("key");
        assert!(window.is_blocked("key"));

        std::thread::sleep(Duration::from_millis(60));

        assert!(!window.is_blocked("key"));
    }

    #[test]
    fn relaxed_mode_allows_far_more() {
        let limits = RateLimits::new(RateLimitMode::Relaxed);

        for _ in 0..100 {
            assert!(limits.allow_login(ip(1)));
            assert!(limits.allow_email_to("victim@example.test"));
        }
    }
}
