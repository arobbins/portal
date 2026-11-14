use crate::mail::Email;
use crate::utils::escape_html;

pub fn api_token_created_email(
    recipient_email: &str,
    token_name: &str,
    developer_url: &str,
) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "A new API token was created".to_string(),
        text: format!(
            "A new API token ({token_name}) was created for your account. Apps that have it can use your account through the API.\n\nIf this wasn't you, revoke it and change your password: {developer_url}"
        ),
        html: format!(
            "<p>A new API token ({}) was created for your account. Apps that have it can use your account through the API.</p><p>If this wasn't you, <a href=\"{developer_url}\">revoke it</a> and change your password.</p>",
            escape_html(token_name)
        ),
    }
}
