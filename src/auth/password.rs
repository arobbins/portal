use std::sync::LazyLock;

use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use password_hash::{PasswordHash, SaltString};
use topcoat::{Error, Result};

use crate::action::{ActionProblem, Outcome, ProblemSpec, TooManyAttempts, fail};
use crate::env::Env;
use crate::secret::random_base62;
use crate::user::model::{AccountStatus, User};

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
    fn spec(self) -> ProblemSpec {
        match self {
            Self::TooShort => ProblemSpec::new(
                "password_too_short",
                "Password must be at least 8 characters.",
            )
            .on_field(PASSWORD_FIELD),
            Self::TooLong => ProblemSpec::new(
                "password_too_long",
                "Password must be 256 characters or fewer.",
            )
            .on_field(PASSWORD_FIELD),
            Self::Mismatch => ProblemSpec::new("password_mismatch", "Passwords do not match.")
                .on_field(CONFIRMATION_FIELD),
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
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Wrong => {
                ProblemSpec::new("wrong_password", "Your current password is incorrect.")
                    .on_field(CURRENT_PASSWORD_FIELD)
            }
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn check_current_password(
    env: &Env<'_>,
    user: &User,
    password: &str,
) -> Outcome<(), CurrentPasswordProblem> {
    let limits = env.limits();
    let ip = env.ip;
    if !limits.allow_password_submission(ip).await? || limits.login_blocked(&user.email, ip).await?
    {
        return fail(CurrentPasswordProblem::TooManyAttempts);
    }

    let hash = match user.status() {
        AccountStatus::Active { password_hash } => Some(password_hash),
        AccountStatus::AwaitingPassword => None,
    };
    if !password_matches(hash, password) {
        limits.record_failed_login(&user.email, ip).await?;
        return fail(CurrentPasswordProblem::Wrong);
    }
    limits.clear_failed_logins(&user.email, ip).await?;

    Ok(Ok(()))
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
