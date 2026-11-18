use rand::Rng;
use sha2::{Digest, Sha256};

pub const BASE62: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const PUBLIC_ID_LENGTH: usize = 20;

pub fn random_base62(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| char::from(BASE62[rng.gen_range(0..BASE62.len())]))
        .collect()
}

pub fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicIdKind {
    ApiToken,
    Device,
    Feedback,
    Passkey,
    User,
}

impl PublicIdKind {
    pub fn prefix(self) -> &'static str {
        match self {
            Self::ApiToken => "tok_",
            Self::Device => "dev_",
            Self::Feedback => "fbk_",
            Self::User => "usr_",
            Self::Passkey => "pk_",
        }
    }

    pub fn generate(self) -> String {
        format!("{}{}", self.prefix(), random_base62(PUBLIC_ID_LENGTH))
    }

    pub fn is_valid(self, id: &str) -> bool {
        id.strip_prefix(self.prefix()).is_some_and(|rest| {
            rest.len() == PUBLIC_ID_LENGTH && rest.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_strings_are_base62_and_unique() {
        let first = random_base62(32);

        assert_eq!(first.len(), 32);
        assert!(first.bytes().all(|byte| byte.is_ascii_alphanumeric()));
        assert_ne!(first, random_base62(32));
    }

    #[test]
    fn secrets_hash_to_hex_sha256() {
        assert_eq!(
            hash_secret("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn public_ids_have_their_prefix_and_validate() {
        for kind in [
            PublicIdKind::ApiToken,
            PublicIdKind::Device,
            PublicIdKind::Passkey,
        ] {
            let id = kind.generate();
            assert!(id.starts_with(kind.prefix()));
            assert!(kind.is_valid(&id));
        }
        assert!(!PublicIdKind::Passkey.is_valid(&PublicIdKind::ApiToken.generate()));
        assert!(!PublicIdKind::ApiToken.is_valid("tok_short"));
        assert!(!PublicIdKind::ApiToken.is_valid("tok_ééééééééééééééééééé"));
    }
}
