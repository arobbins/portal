use sha2::{Digest, Sha256};

pub fn get_token_hash_from_token_string(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn create_body_page_class(page_path: String) -> String {
    if page_path.is_empty() {
        return format!("page-{}", "home");
    }

    format!("page-{}", page_path)
}

const MAX_EMAIL_LENGTH: usize = 254;

pub fn parse_email(input: &str) -> Option<String> {
    let email = input.trim();
    let is_plausible = email.len() <= MAX_EMAIL_LENGTH
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty());

    is_plausible.then(|| email.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_a_valid_email() {
        assert_eq!(
            parse_email("  Ada@Example.TEST "),
            Some("ada@example.test".to_string())
        );
    }

    #[test]
    fn rejects_missing_parts_and_oversized_input() {
        assert_eq!(parse_email(""), None);
        assert_eq!(parse_email("no-at-sign"), None);
        assert_eq!(parse_email("@example.test"), None);
        assert_eq!(parse_email("ada@"), None);
        assert_eq!(
            parse_email(&format!("{}@example.test", "a".repeat(250))),
            None
        );
    }
}
