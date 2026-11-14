use crate::mail::Email;
use crate::utils::escape_html;

pub fn passkey_added_email(recipient_email: &str, passkey_name: &str, security_url: &str) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "A passkey was added to your account".to_string(),
        text: format!(
            "A new passkey ({passkey_name}) was added to your account. You can now use it to sign in.\n\nIf this wasn't you, change your password and remove the passkey: {security_url}"
        ),
        html: format!(
            "<p>A new passkey ({}) was added to your account. You can now use it to sign in.</p><p>If this wasn't you, <a href=\"{security_url}\">change your password and remove the passkey</a>.</p>",
            escape_html(passkey_name)
        ),
    }
}
