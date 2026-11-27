use super::{
    API_CALLS_PER_MINUTE, AUTHORIZATION_STARTS_PER_IP_PER_HOUR, BAD_API_TOKEN_WINDOW_MINUTES,
    BAD_API_TOKENS_PER_WINDOW, BILLING_SESSIONS_PER_ACCOUNT_PER_HOUR,
    CLIENT_REGISTRATIONS_PER_IP_PER_HOUR, EXPORTS_PER_ACCOUNT_PER_HOUR,
    FEEDBACK_PER_ACCOUNT_PER_DAY, FEEDBACK_PER_TOKEN_PER_HOUR, PLAN_LISTS_PER_IP_PER_MINUTE,
    RELAXED_SCALE, RateLimitMode, TOKEN_REQUESTS_PER_CLIENT_PER_MINUTE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Limit {
    PasswordSubmissionsByIp,
    EmailRequestsByIp,
    PasskeyChallengesByIp,
    EmailsToAddress,
    ApiCallsByToken,
    FailedLoginsByAccountAndIp,
    FailedLoginsByAccount,
    FailedSecondFactorsByAccount,
    FailedApiAuthByIp,
    FeedbackByToken,
    FeedbackByAccount,
    ExportsByAccount,
    BillingSessionsByAccount,
    PlanListsByIp,
    ApiCallsByApp,
    ClientRegistrationsByIp,
    AuthorizationStartsByIp,
    TokenRequestsByClient,
}

impl Limit {
    pub(super) fn bucket(self) -> &'static str {
        match self {
            Self::PasswordSubmissionsByIp => "password_submissions_by_ip",
            Self::EmailRequestsByIp => "email_requests_by_ip",
            Self::PasskeyChallengesByIp => "passkey_challenges_by_ip",
            Self::EmailsToAddress => "emails_to_address",
            Self::ApiCallsByToken => "api_calls_by_token",
            Self::FailedLoginsByAccountAndIp => "failed_logins_by_account_and_ip",
            Self::FailedLoginsByAccount => "failed_logins_by_account",
            Self::FailedSecondFactorsByAccount => "failed_second_factors_by_account",
            Self::FailedApiAuthByIp => "failed_api_auth_by_ip",
            Self::FeedbackByToken => "feedback_by_token",
            Self::FeedbackByAccount => "feedback_by_account",
            Self::ExportsByAccount => "exports_by_account",
            Self::BillingSessionsByAccount => "billing_sessions_by_account",
            Self::PlanListsByIp => "plan_lists_by_ip",
            Self::ApiCallsByApp => "api_calls_by_app",
            Self::ClientRegistrationsByIp => "client_registrations_by_ip",
            Self::AuthorizationStartsByIp => "authorization_starts_by_ip",
            Self::TokenRequestsByClient => "token_requests_by_client",
        }
    }

    pub(super) fn window_seconds(self) -> f64 {
        let minutes: u64 = match self {
            Self::PasswordSubmissionsByIp
            | Self::PasskeyChallengesByIp
            | Self::ApiCallsByToken
            | Self::ApiCallsByApp
            | Self::TokenRequestsByClient
            | Self::PlanListsByIp => 1,
            Self::EmailRequestsByIp => 10,
            Self::FailedLoginsByAccountAndIp | Self::FailedSecondFactorsByAccount => 15,
            Self::FailedApiAuthByIp => BAD_API_TOKEN_WINDOW_MINUTES,
            Self::EmailsToAddress
            | Self::FailedLoginsByAccount
            | Self::FeedbackByToken
            | Self::ExportsByAccount
            | Self::BillingSessionsByAccount
            | Self::ClientRegistrationsByIp
            | Self::AuthorizationStartsByIp => 60,
            Self::FeedbackByAccount => 24 * 60,
        };
        (minutes * 60) as f64
    }

    pub(super) fn max(self, mode: RateLimitMode) -> i64 {
        let base: u32 = match self {
            Self::PasswordSubmissionsByIp => 10,
            Self::EmailRequestsByIp => 5,
            Self::PasskeyChallengesByIp => 30,
            Self::EmailsToAddress => 3,
            Self::ApiCallsByToken => API_CALLS_PER_MINUTE,
            Self::FailedLoginsByAccountAndIp | Self::FailedSecondFactorsByAccount => 5,
            Self::FailedLoginsByAccount => 50,
            Self::FailedApiAuthByIp => BAD_API_TOKENS_PER_WINDOW,
            Self::FeedbackByToken => FEEDBACK_PER_TOKEN_PER_HOUR,
            Self::FeedbackByAccount => FEEDBACK_PER_ACCOUNT_PER_DAY,
            Self::ExportsByAccount => EXPORTS_PER_ACCOUNT_PER_HOUR,
            Self::BillingSessionsByAccount => BILLING_SESSIONS_PER_ACCOUNT_PER_HOUR,
            Self::PlanListsByIp => PLAN_LISTS_PER_IP_PER_MINUTE,
            Self::ApiCallsByApp => API_CALLS_PER_MINUTE,
            Self::ClientRegistrationsByIp => CLIENT_REGISTRATIONS_PER_IP_PER_HOUR,
            Self::AuthorizationStartsByIp => AUTHORIZATION_STARTS_PER_IP_PER_HOUR,
            Self::TokenRequestsByClient => TOKEN_REQUESTS_PER_CLIENT_PER_MINUTE,
        };
        let scales = !matches!(
            self,
            Self::FailedLoginsByAccountAndIp
                | Self::FailedLoginsByAccount
                | Self::FailedSecondFactorsByAccount
        );
        let scale = match mode {
            RateLimitMode::Relaxed if scales => RELAXED_SCALE,
            _ => 1,
        };
        i64::from(base * scale)
    }
}
