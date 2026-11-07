use crate::mail::Email;

pub fn register_verification_email(recipient_email: &str, verify_url: &str) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "Confirm your email".to_string(),
        text: format!("Confirm your account: {verify_url}"),
        html: format!("<p>Welcome! <a href=\"{verify_url}\">Confirm your account</a>.</p>"),
    }
}
