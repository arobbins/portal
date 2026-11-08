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

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
