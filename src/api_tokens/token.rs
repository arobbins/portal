use crate::secret::{checked_secret, hash_secret, is_checked_secret};

pub const TOKEN_PREFIX: &str = "wsk_";
const SECRET_LENGTH: usize = 43;
#[cfg(test)]
const CHECKSUM_LENGTH: usize = 6;
#[cfg(test)]
pub const TOKEN_LENGTH: usize = TOKEN_PREFIX.len() + SECRET_LENGTH + CHECKSUM_LENGTH;
const LAST_FOUR: usize = 4;

#[derive(Clone, PartialEq, Eq)]
pub struct RevealedToken(String);

impl RevealedToken {
    pub fn expose(&self) -> &str {
        &self.0
    }

    #[cfg(test)]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for RevealedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RevealedToken(…)")
    }
}

pub struct NewApiToken {
    pub raw: RevealedToken,
    pub hash: String,
    pub last_four: String,
}

pub fn generate_api_token() -> NewApiToken {
    let raw = checked_secret(TOKEN_PREFIX, SECRET_LENGTH);

    NewApiToken {
        hash: hash_secret(&raw),
        last_four: raw[raw.len() - LAST_FOUR..].to_string(),
        raw: RevealedToken(raw),
    }
}

pub fn is_well_formed(raw: &str) -> bool {
    is_checked_secret(TOKEN_PREFIX, SECRET_LENGTH, raw)
}

pub fn display_hint(last_four: &str) -> String {
    format!("{TOKEN_PREFIX}…{last_four}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_revealed_token_never_shows_in_debug_output() {
        let token = generate_api_token();

        assert!(!format!("{:?}", token.raw).contains(token.raw.expose()));
    }

    #[test]
    fn generated_tokens_are_prefixed_well_formed_and_unique() {
        let first = generate_api_token();
        let second = generate_api_token();
        let (first_raw, second_raw) = (first.raw.expose(), second.raw.expose());

        assert!(first_raw.starts_with(TOKEN_PREFIX));
        assert_eq!(first_raw.len(), TOKEN_LENGTH);
        assert!(is_well_formed(first_raw));
        assert_ne!(first_raw, second_raw);
        assert_eq!(first.hash, hash_secret(first_raw));
        assert!(first_raw.ends_with(&first.last_four));
    }

    #[test]
    fn a_changed_character_fails_the_checksum() {
        let token = generate_api_token().raw.into_inner();
        let position = TOKEN_PREFIX.len() + 5;
        let original = token.as_bytes()[position];
        let replacement = if original == b'a' { "b" } else { "a" };
        let tampered = format!(
            "{}{replacement}{}",
            &token[..position],
            &token[position + 1..]
        );

        assert!(!is_well_formed(&tampered));
    }

    #[test]
    fn other_shapes_are_rejected_without_a_lookup() {
        let token = generate_api_token().raw.into_inner();

        assert!(!is_well_formed(""));
        assert!(!is_well_formed(&token[1..]));
        assert!(!is_well_formed(&format!("{token}x")));
        assert!(!is_well_formed(&token.replacen(TOKEN_PREFIX, "ghp_", 1)));
        assert!(!is_well_formed(&format!("{}é", &token[..TOKEN_LENGTH - 2])));
    }
}
