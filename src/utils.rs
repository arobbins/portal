use axum::response::Html;
use sha2::{Digest, Sha256};

pub fn get_token_hash_from_token_string(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn html_error(message: &str) -> Html<String> {
    Html(format!(
        "<p class=\"form__message form__message--error\">{message}</p>"
    ))
}

pub fn html_success(message: &str) -> Html<String> {
    Html(format!(
        "<p class=\"form__message form__message--success\">{message}</p>"
    ))
}

pub fn create_body_page_class(page_path: String) -> String {
    println!("page_path: {}", page_path);
    println!("page_path type: {}", std::any::type_name_of_val(&page_path));

    if page_path.is_empty() {
        return format!("page-{}", "home");
    }

    format!("page-{}", page_path)
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
