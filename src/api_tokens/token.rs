use sha2::{Digest, Sha256};

use crate::secret::{BASE62, hash_secret, random_base62};

pub const TOKEN_PREFIX: &str = "wsk_";
const SECRET_LENGTH: usize = 43;
const CHECKSUM_LENGTH: usize = 6;
pub const TOKEN_LENGTH: usize = TOKEN_PREFIX.len() + SECRET_LENGTH + CHECKSUM_LENGTH;
const LAST_FOUR: usize = 4;

pub struct NewApiToken {
    pub raw: String,
    pub hash: String,
    pub last_four: String,
}

pub fn generate_api_token() -> NewApiToken {
    let secret = random_base62(SECRET_LENGTH);
    let raw = format!("{TOKEN_PREFIX}{secret}{}", checksum(&secret));

    NewApiToken {
        hash: hash_secret(&raw),
        last_four: raw[raw.len() - LAST_FOUR..].to_string(),
        raw,
    }
}

pub fn is_well_formed(raw: &str) -> bool {
    let Some(body) = raw.strip_prefix(TOKEN_PREFIX) else {
        return false;
    };
    if raw.len() != TOKEN_LENGTH || !body.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return false;
    }
    let (secret, given) = body.split_at(SECRET_LENGTH);

    checksum(secret) == given
}

pub fn display_hint(last_four: &str) -> String {
    format!("{TOKEN_PREFIX}…{last_four}")
}

fn checksum(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    let mut value = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    let mut digits = [b'0'; CHECKSUM_LENGTH];
    for digit in digits.iter_mut().rev() {
        *digit = BASE62[(value % 62) as usize];
        value /= 62;
    }

    String::from_utf8(digits.to_vec()).expect("base62 digits are ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_prefixed_well_formed_and_unique() {
        let first = generate_api_token();
        let second = generate_api_token();

        assert!(first.raw.starts_with(TOKEN_PREFIX));
        assert_eq!(first.raw.len(), TOKEN_LENGTH);
        assert!(is_well_formed(&first.raw));
        assert_ne!(first.raw, second.raw);
        assert_eq!(first.hash, hash_secret(&first.raw));
        assert!(first.raw.ends_with(&first.last_four));
    }

    #[test]
    fn a_changed_character_fails_the_checksum() {
        let token = generate_api_token().raw;
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
        let token = generate_api_token().raw;

        assert!(!is_well_formed(""));
        assert!(!is_well_formed(&token[1..]));
        assert!(!is_well_formed(&format!("{token}x")));
        assert!(!is_well_formed(&token.replacen(TOKEN_PREFIX, "ghp_", 1)));
        assert!(!is_well_formed(&format!("{}é", &token[..TOKEN_LENGTH - 2])));
    }
}
