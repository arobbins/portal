use std::sync::LazyLock;

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use password_hash::{PasswordHash, SaltString};
use topcoat::{
    Error, Result,
    context::Cx,
    router::{StatusCode, request::client_ip},
};

use crate::action::{ActionProblem, TooManyAttempts};
use crate::secret::random_base62;
use crate::user::model::{AccountStatus, User};
use crate::web::rate_limits;

pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 256;
pub const PASSWORD_FIELD: &str = "password";
pub const CONFIRMATION_FIELD: &str = "confirm_password";
pub const CURRENT_PASSWORD_FIELD: &str = "current_password";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordProblem {
    TooShort,
    TooLong,
    Mismatch,
}

impl PasswordProblem {
    pub fn invalid_password(self) -> bool {
        matches!(self, Self::TooShort | Self::TooLong)
    }

    pub fn invalid_confirmation(self) -> bool {
        self == Self::Mismatch
    }
}

impl ActionProblem for PasswordProblem {
    fn code(self) -> &'static str {
        match self {
            Self::TooShort => "password_too_short",
            Self::TooLong => "password_too_long",
            Self::Mismatch => "password_mismatch",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::TooShort => "Password must be at least 8 characters.",
            Self::TooLong => "Password must be 256 characters or fewer.",
            Self::Mismatch => "Passwords do not match.",
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::TooShort | Self::TooLong => Some(PASSWORD_FIELD),
            Self::Mismatch => Some(CONFIRMATION_FIELD),
        }
    }
}

pub fn check_new_password(password: &str, confirmation: &str) -> Result<(), PasswordProblem> {
    let length = password.chars().count();

    if length < MIN_PASSWORD_LENGTH {
        return Err(PasswordProblem::TooShort);
    }

    if length > MAX_PASSWORD_LENGTH {
        return Err(PasswordProblem::TooLong);
    }

    if password != confirmation {
        return Err(PasswordProblem::Mismatch);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentPasswordProblem {
    Wrong,
    TooManyAttempts,
}

impl ActionProblem for CurrentPasswordProblem {
    fn code(self) -> &'static str {
        match self {
            Self::Wrong => "wrong_password",
            Self::TooManyAttempts => TooManyAttempts.code(),
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Wrong => "Your current password is incorrect.",
            Self::TooManyAttempts => TooManyAttempts.message(),
        }
    }

    fn field(self) -> Option<&'static str> {
        match self {
            Self::Wrong => Some(CURRENT_PASSWORD_FIELD),
            Self::TooManyAttempts => None,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::Wrong => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyAttempts => TooManyAttempts.status(),
        }
    }
}

pub fn check_current_password(
    cx: &Cx,
    user: &User,
    password: &str,
) -> Result<(), CurrentPasswordProblem> {
    let limits = rate_limits(cx);
    let ip = client_ip(cx);
    if !limits.allow_password_submission(ip) || limits.login_blocked(&user.email, ip) {
        return Err(CurrentPasswordProblem::TooManyAttempts);
    }

    let hash = match user.status() {
        AccountStatus::Active { password_hash } => Some(password_hash),
        AccountStatus::AwaitingPassword => None,
    };
    if !password_matches(hash, password) {
        limits.record_failed_login(&user.email, ip);
        return Err(CurrentPasswordProblem::Wrong);
    }
    limits.clear_failed_logins(&user.email, ip);

    Ok(())
}

static UNMATCHABLE_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password(&random_base62(32)).expect("argon2 can hash a random password"));

pub fn password_matches(password_hash: Option<&str>, password: &str) -> bool {
    let checked_hash = password_hash.unwrap_or(&UNMATCHABLE_HASH);
    let matches = PasswordHash::new(checked_hash).is_ok_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    });

    matches && password_hash.is_some()
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(rand::thread_rng());
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::msg(e.to_string()))?
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_matching_password_of_a_sensible_length() {
        assert_eq!(check_new_password("long-enough", "long-enough"), Ok(()));
    }

    #[test]
    fn rejects_short_long_and_mismatched_passwords() {
        assert_eq!(
            check_new_password("short", "short"),
            Err(PasswordProblem::TooShort)
        );
        let long = "a".repeat(MAX_PASSWORD_LENGTH + 1);
        assert_eq!(
            check_new_password(&long, &long),
            Err(PasswordProblem::TooLong)
        );
        assert_eq!(
            check_new_password("long-enough", "different"),
            Err(PasswordProblem::Mismatch)
        );
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(
            check_new_password("ééééééé", "ééééééé"),
            Err(PasswordProblem::TooShort)
        );
    }

    #[test]
    fn the_length_messages_match_the_limits() {
        assert!(
            PasswordProblem::TooShort
                .message()
                .contains(&format!("{MIN_PASSWORD_LENGTH} characters"))
        );
        assert!(
            PasswordProblem::TooLong
                .message()
                .contains(&format!("{MAX_PASSWORD_LENGTH} characters"))
        );
    }

    #[test]
    fn passwords_match_only_their_own_hash() {
        let hash = hash_password("correct-password").unwrap();

        assert!(password_matches(Some(&hash), "correct-password"));
        assert!(!password_matches(Some(&hash), "wrong-password"));
        assert!(!password_matches(Some("not-a-hash"), "correct-password"));
        assert!(!password_matches(None, "correct-password"));
    }
}
