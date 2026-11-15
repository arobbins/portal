use std::net::{IpAddr, Ipv4Addr};

use sqlx::PgPool;

mod queries;

use queries::{
    query_count_rate_limit_hits, query_delete_old_rate_limit_hits, query_delete_rate_limit_hits,
    query_insert_rate_limit_hit, query_take_rate_limit_slot,
};

const UNKNOWN_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const RELAXED_SCALE: u32 = 1000;
const MAX_KEY_LENGTH: usize = 254;
pub const API_CALLS_PER_MINUTE: u32 = 600;
pub const BAD_API_TOKENS_PER_WINDOW: u32 = 20;
pub const BAD_API_TOKEN_WINDOW_MINUTES: u64 = 10;
pub const KEEP_HITS_FOR: time::Duration = time::Duration::days(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitMode {
    Strict,
    Relaxed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Limit {
    PasswordSubmissionsByIp,
    EmailRequestsByIp,
    PasskeyChallengesByIp,
    EmailsToAddress,
    ApiCallsByToken,
    FailedLoginsByAccountAndIp,
    FailedLoginsByAccount,
    FailedApiAuthByIp,
}

impl Limit {
    fn bucket(self) -> &'static str {
        match self {
            Self::PasswordSubmissionsByIp => "password_submissions_by_ip",
            Self::EmailRequestsByIp => "email_requests_by_ip",
            Self::PasskeyChallengesByIp => "passkey_challenges_by_ip",
            Self::EmailsToAddress => "emails_to_address",
            Self::ApiCallsByToken => "api_calls_by_token",
            Self::FailedLoginsByAccountAndIp => "failed_logins_by_account_and_ip",
            Self::FailedLoginsByAccount => "failed_logins_by_account",
            Self::FailedApiAuthByIp => "failed_api_auth_by_ip",
        }
    }

    fn window_seconds(self) -> f64 {
        let minutes: u64 = match self {
            Self::PasswordSubmissionsByIp | Self::PasskeyChallengesByIp | Self::ApiCallsByToken => {
                1
            }
            Self::EmailRequestsByIp => 10,
            Self::FailedLoginsByAccountAndIp => 15,
            Self::FailedApiAuthByIp => BAD_API_TOKEN_WINDOW_MINUTES,
            Self::EmailsToAddress | Self::FailedLoginsByAccount => 60,
        };
        (minutes * 60) as f64
    }

    fn max(self, mode: RateLimitMode) -> i64 {
        let base: u32 = match self {
            Self::PasswordSubmissionsByIp => 10,
            Self::EmailRequestsByIp => 5,
            Self::PasskeyChallengesByIp => 30,
            Self::EmailsToAddress => 3,
            Self::ApiCallsByToken => API_CALLS_PER_MINUTE,
            Self::FailedLoginsByAccountAndIp => 5,
            Self::FailedLoginsByAccount => 50,
            Self::FailedApiAuthByIp => BAD_API_TOKENS_PER_WINDOW,
        };
        let scales = !matches!(
            self,
            Self::FailedLoginsByAccountAndIp | Self::FailedLoginsByAccount
        );
        let scale = match mode {
            RateLimitMode::Relaxed if scales => RELAXED_SCALE,
            _ => 1,
        };
        i64::from(base * scale)
    }
}

pub struct RateLimits {
    db: PgPool,
    mode: RateLimitMode,
}

fn ip_key(ip: Option<IpAddr>) -> String {
    ip.unwrap_or(UNKNOWN_IP).to_string()
}

fn failed_login_key(email: &str, ip: Option<IpAddr>) -> String {
    format!("{} {}", account_key(email), ip_key(ip))
}

fn account_key(email: &str) -> String {
    email
        .trim()
        .chars()
        .take(MAX_KEY_LENGTH)
        .collect::<String>()
        .to_lowercase()
}

impl RateLimits {
    pub fn new(db: PgPool, mode: RateLimitMode) -> Self {
        Self { db, mode }
    }

    async fn allow(&self, limit: Limit, key: &str) -> Result<bool, sqlx::Error> {
        query_take_rate_limit_slot(
            limit.bucket(),
            key,
            limit.window_seconds(),
            limit.max(self.mode),
            &self.db,
        )
        .await
    }

    async fn blocked(&self, limit: Limit, key: &str) -> Result<bool, sqlx::Error> {
        let hits =
            query_count_rate_limit_hits(limit.bucket(), key, limit.window_seconds(), &self.db)
                .await?;
        Ok(hits >= limit.max(self.mode))
    }

    async fn record(&self, limit: Limit, key: &str) -> Result<(), sqlx::Error> {
        query_insert_rate_limit_hit(limit.bucket(), key, &self.db).await
    }

    pub async fn allow_password_submission(&self, ip: Option<IpAddr>) -> Result<bool, sqlx::Error> {
        self.allow(Limit::PasswordSubmissionsByIp, &ip_key(ip))
            .await
    }

    pub async fn allow_email_request(&self, ip: Option<IpAddr>) -> Result<bool, sqlx::Error> {
        self.allow(Limit::EmailRequestsByIp, &ip_key(ip)).await
    }

    pub async fn allow_passkey_challenge(&self, ip: Option<IpAddr>) -> Result<bool, sqlx::Error> {
        self.allow(Limit::PasskeyChallengesByIp, &ip_key(ip)).await
    }

    pub async fn allow_email_to(&self, address: &str) -> Result<bool, sqlx::Error> {
        self.allow(Limit::EmailsToAddress, &account_key(address))
            .await
    }

    pub async fn allow_api_call(&self, token_id: i32) -> Result<bool, sqlx::Error> {
        self.allow(Limit::ApiCallsByToken, &token_id.to_string())
            .await
    }

    pub async fn login_blocked(
        &self,
        email: &str,
        ip: Option<IpAddr>,
    ) -> Result<bool, sqlx::Error> {
        Ok(self
            .blocked(
                Limit::FailedLoginsByAccountAndIp,
                &failed_login_key(email, ip),
            )
            .await?
            || self
                .blocked(Limit::FailedLoginsByAccount, &account_key(email))
                .await?)
    }

    pub async fn record_failed_login(
        &self,
        email: &str,
        ip: Option<IpAddr>,
    ) -> Result<(), sqlx::Error> {
        self.record(
            Limit::FailedLoginsByAccountAndIp,
            &failed_login_key(email, ip),
        )
        .await?;
        self.record(Limit::FailedLoginsByAccount, &account_key(email))
            .await
    }

    pub async fn clear_failed_logins(
        &self,
        email: &str,
        ip: Option<IpAddr>,
    ) -> Result<(), sqlx::Error> {
        let key = failed_login_key(email, ip);
        query_delete_rate_limit_hits(Limit::FailedLoginsByAccountAndIp.bucket(), &key, &self.db)
            .await
    }

    pub async fn api_auth_blocked(&self, ip: Option<IpAddr>) -> Result<bool, sqlx::Error> {
        self.blocked(Limit::FailedApiAuthByIp, &ip_key(ip)).await
    }

    pub async fn forget_old_hits(&self, now: time::OffsetDateTime) -> Result<u64, sqlx::Error> {
        query_delete_old_rate_limit_hits(now - KEEP_HITS_FOR, &self.db).await
    }

    pub async fn record_failed_api_auth(&self, ip: Option<IpAddr>) -> Result<(), sqlx::Error> {
        self.record(Limit::FailedApiAuthByIp, &ip_key(ip)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(last: u8) -> Option<IpAddr> {
        Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, last)))
    }

    async fn allowed_in_a_row(times: usize, check: impl AsyncFn() -> bool) -> bool {
        for _ in 0..times {
            if !check().await {
                return false;
            }
        }
        true
    }

    #[sqlx::test]
    async fn password_submissions_are_limited_per_ip(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);

        assert!(
            allowed_in_a_row(10, async || limits
                .allow_password_submission(ip(1))
                .await
                .unwrap())
            .await
        );
        assert!(!limits.allow_password_submission(ip(1)).await.unwrap());
        assert!(limits.allow_password_submission(ip(2)).await.unwrap());
    }

    #[sqlx::test]
    async fn email_requests_are_limited_per_ip(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);

        assert!(
            allowed_in_a_row(5, async || limits.allow_email_request(ip(1)).await.unwrap()).await
        );
        assert!(!limits.allow_email_request(ip(1)).await.unwrap());
        assert!(limits.allow_email_request(ip(2)).await.unwrap());
    }

    #[sqlx::test]
    async fn emails_are_limited_per_address_regardless_of_case(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);

        assert!(
            allowed_in_a_row(3, async || limits
                .allow_email_to("victim@example.test")
                .await
                .unwrap())
            .await
        );
        assert!(!limits.allow_email_to("Victim@Example.test").await.unwrap());
        assert!(
            limits
                .allow_email_to("someone-else@example.test")
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn five_failed_logins_block_that_account_from_that_ip(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);

        for _ in 0..5 {
            assert!(
                !limits
                    .login_blocked("owner@example.test", ip(1))
                    .await
                    .unwrap()
            );
            limits
                .record_failed_login("owner@example.test", ip(1))
                .await
                .unwrap();
        }

        assert!(
            limits
                .login_blocked("Owner@Example.test", ip(1))
                .await
                .unwrap()
        );
        assert!(
            !limits
                .login_blocked("owner@example.test", ip(2))
                .await
                .unwrap()
        );
        assert!(
            !limits
                .login_blocked("someone-else@example.test", ip(1))
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn a_successful_login_clears_the_failures(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);
        for _ in 0..5 {
            limits
                .record_failed_login("owner@example.test", ip(1))
                .await
                .unwrap();
        }

        limits
            .clear_failed_logins("owner@example.test", ip(1))
            .await
            .unwrap();

        assert!(
            !limits
                .login_blocked("owner@example.test", ip(1))
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn many_failures_from_many_ips_block_the_account_everywhere(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);
        for attempt in 0..50u8 {
            limits
                .record_failed_login("owner@example.test", ip(attempt))
                .await
                .unwrap();
        }

        assert!(
            limits
                .login_blocked("owner@example.test", ip(200))
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn failures_expire_after_the_window(db: PgPool) {
        let limits = RateLimits::new(db.clone(), RateLimitMode::Strict);
        for _ in 0..5 {
            limits
                .record_failed_login("owner@example.test", ip(1))
                .await
                .unwrap();
        }
        sqlx::query!("UPDATE rate_limit_hits SET at = now() - interval '16 minutes'")
            .execute(&db)
            .await
            .unwrap();

        assert!(
            !limits
                .login_blocked("owner@example.test", ip(1))
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn twenty_bad_api_tokens_block_that_ip_only(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);
        for _ in 0..BAD_API_TOKENS_PER_WINDOW {
            assert!(!limits.api_auth_blocked(ip(7)).await.unwrap());
            limits.record_failed_api_auth(ip(7)).await.unwrap();
        }

        assert!(limits.api_auth_blocked(ip(7)).await.unwrap());
        assert!(!limits.api_auth_blocked(ip(8)).await.unwrap());
    }

    #[sqlx::test]
    async fn api_calls_are_limited_per_token(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Strict);

        assert!(
            allowed_in_a_row(API_CALLS_PER_MINUTE as usize, async || limits
                .allow_api_call(1)
                .await
                .unwrap())
            .await
        );
        assert!(!limits.allow_api_call(1).await.unwrap());
        assert!(limits.allow_api_call(2).await.unwrap());
    }

    #[sqlx::test]
    async fn relaxed_mode_allows_far_more_but_never_relaxes_login_lockout(db: PgPool) {
        let limits = RateLimits::new(db, RateLimitMode::Relaxed);

        assert!(
            allowed_in_a_row(50, async || limits
                .allow_password_submission(ip(1))
                .await
                .unwrap())
            .await
        );
        for _ in 0..5 {
            limits
                .record_failed_login("owner@example.test", ip(1))
                .await
                .unwrap();
        }
        assert!(
            limits
                .login_blocked("owner@example.test", ip(1))
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn old_hits_are_cleaned_up(db: PgPool) {
        let limits = RateLimits::new(db.clone(), RateLimitMode::Strict);
        limits.record_failed_api_auth(ip(1)).await.unwrap();
        sqlx::query!("UPDATE rate_limit_hits SET at = now() - interval '2 days'")
            .execute(&db)
            .await
            .unwrap();

        let limits = RateLimits::new(db.clone(), RateLimitMode::Strict);
        assert_eq!(
            limits
                .forget_old_hits(time::OffsetDateTime::now_utc())
                .await
                .unwrap(),
            1
        );
    }
}
