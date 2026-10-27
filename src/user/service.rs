use std::env;
use reqwest::header::ACCEPT;
use std::collections::HashMap;

pub fn create_register_verification_email_body(
    recipient_email: &str,
    verify_url: &str,
) -> HashMap<&'static str, String> {
    let mut map = HashMap::new();

    map.insert("From", "hello@andrewrobbins.dev".to_string());
    map.insert("To", recipient_email.to_string());
    map.insert("Subject", "Confirm your email".to_string());
    map.insert("TextBody", format!("Confirm your account: {verify_url}"));
    map.insert("HtmlBody", format!("<p>Welcome! <a href=\"{verify_url}\">Confirm your account</a>.</p>"));
    map.insert("MessageStream", "outbound".to_string());

    map
}

pub async fn send_register_verification_email(
    recipient_email: &str,
    verify_url: &str,
) -> Result<(), String> {
    let postmark_server_token = env::var("POSTMARK_SERVER_TOKEN").unwrap();
    let postmark_base_url = env::var("POSTMARK_BASE_URL").unwrap();

    let email_body = create_register_verification_email_body(recipient_email, verify_url);

    let response = reqwest::Client::new()
        .post(format!("{postmark_base_url}/email"))
        .header("X-Postmark-Server-Token", postmark_server_token)
        .header(ACCEPT, "application/json")
        .json(&email_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("Postmark returned {status}: {body}"));
    }

    Ok(())
}