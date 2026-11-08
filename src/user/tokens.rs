use rand::Rng;
use rand::distributions::Alphanumeric;
use time::{Duration, OffsetDateTime};

use crate::utils::get_token_hash_from_token_string;

const TOKEN_LENGTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPurpose {
    EmailVerification,
}

impl TokenPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmailVerification => "email_verification",
        }
    }

    pub fn expires_at(self) -> OffsetDateTime {
        let lifetime = match self {
            Self::EmailVerification => Duration::hours(24),
        };
        OffsetDateTime::now_utc() + lifetime
    }
}

pub struct GeneratedToken {
    pub raw: String,
    pub hash: String,
}

pub fn generate_token() -> GeneratedToken {
    let raw: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LENGTH)
        .map(char::from)
        .collect();
    let hash = get_token_hash_from_token_string(&raw);

    GeneratedToken { raw, hash }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_unique_and_hashed() {
        let first = generate_token();
        let second = generate_token();

        assert_eq!(first.raw.len(), TOKEN_LENGTH);
        assert_ne!(first.raw, second.raw);
        assert_ne!(first.raw, first.hash);
        assert_eq!(first.hash, get_token_hash_from_token_string(&first.raw));
    }
}
