use time::Duration;

use crate::secret::{checked_secret, hash_secret, is_checked_secret};

const SECRET_LENGTH: usize = 43;

pub const ACCESS_TOKEN_LIFETIME: Duration = Duration::hours(1);
pub const REFRESH_TOKEN_IDLE_LIMIT: Duration = Duration::days(30);
pub const CODE_LIFETIME: Duration = Duration::minutes(10);
pub const REQUEST_IDLE_LIMIT: Duration = Duration::minutes(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    AccessToken,
    RefreshToken,
    Code,
}

impl SecretKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::AccessToken => "wsa_",
            Self::RefreshToken => "wsr_",
            Self::Code => "wsc_",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AccessToken => "access",
            Self::RefreshToken => "refresh",
            Self::Code => "code",
        }
    }

    pub fn generate(self) -> NewSecret {
        let raw = checked_secret(self.prefix(), SECRET_LENGTH);
        NewSecret {
            hash: hash_secret(&raw),
            raw,
        }
    }

    pub fn hash_of(self, raw: &str) -> Option<String> {
        is_checked_secret(self.prefix(), SECRET_LENGTH, raw).then(|| hash_secret(raw))
    }

    pub fn lifetime(self) -> Duration {
        match self {
            Self::AccessToken => ACCESS_TOKEN_LIFETIME,
            Self::RefreshToken => REFRESH_TOKEN_IDLE_LIMIT,
            Self::Code => CODE_LIFETIME,
        }
    }
}

pub struct NewSecret {
    pub raw: String,
    pub hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_tokens::token::generate_api_token;

    #[test]
    fn each_kind_only_accepts_its_own_secrets() {
        let kinds = [
            SecretKind::AccessToken,
            SecretKind::RefreshToken,
            SecretKind::Code,
        ];
        for kind in kinds {
            let secret = kind.generate();
            assert_eq!(kind.hash_of(&secret.raw), Some(secret.hash.clone()));
            for other in kinds.into_iter().filter(|other| *other != kind) {
                assert_eq!(other.hash_of(&secret.raw), None);
            }
        }
        assert_eq!(
            SecretKind::AccessToken.hash_of(generate_api_token().raw.expose()),
            None
        );
    }

    #[test]
    fn lifetimes_match_the_spec() {
        assert_eq!(SecretKind::AccessToken.lifetime(), Duration::hours(1));
        assert_eq!(SecretKind::RefreshToken.lifetime(), Duration::days(30));
        assert_eq!(SecretKind::Code.lifetime(), Duration::minutes(10));
    }
}
