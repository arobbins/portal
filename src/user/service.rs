use crate::mail::Email;

pub fn register_verification_email(recipient_email: &str, verify_url: &str) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "Confirm your email".to_string(),
        text: format!(
            "Welcome! Confirm your email and set your password: {verify_url}\n\nThis link expires in 24 hours."
        ),
        html: format!(
            "<p>Welcome! <a href=\"{verify_url}\">Confirm your email and set your password</a>.</p><p>This link expires in 24 hours.</p>"
        ),
    }
}

pub fn account_exists_email(recipient_email: &str, login_url: &str) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "You already have an account".to_string(),
        text: format!(
            "Someone tried to register with this email address, but you already have an account. Log in at {login_url}\n\nIf this wasn't you, you can ignore this email."
        ),
        html: format!(
            "<p>Someone tried to register with this email address, but you already have an account. <a href=\"{login_url}\">Log in</a>.</p><p>If this wasn't you, you can ignore this email.</p>"
        ),
    }
}
