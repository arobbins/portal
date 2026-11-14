use crate::auth::links::TokenPurpose;
use crate::mail::Email;

pub fn register_verification_email(recipient_email: &str, verify_url: &str) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "Confirm your email".to_string(),
        text: format!(
            "Welcome! Confirm your email and set your password: {verify_url}\n\nThis link expires in {}.",
            TokenPurpose::EmailVerification.lifetime_text()
        ),
        html: format!(
            "<p>Welcome! <a href=\"{verify_url}\">Confirm your email and set your password</a>.</p><p>This link expires in {}.</p>",
            TokenPurpose::EmailVerification.lifetime_text()
        ),
    }
}

pub fn account_exists_email(recipient_email: &str, login_url: &str) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "You already have an account".to_string(),
        text: format!(
            "Someone tried to register with this email address, but you already have an account. Sign in at {login_url}\n\nIf this wasn't you, you can ignore this email."
        ),
        html: format!(
            "<p>Someone tried to register with this email address, but you already have an account. <a href=\"{login_url}\">Sign in</a>.</p><p>If this wasn't you, you can ignore this email.</p>"
        ),
    }
}

pub fn password_reset_email(recipient_email: &str, reset_url: &str) -> Email {
    Email {
        to: recipient_email.to_string(),
        subject: "Reset your password".to_string(),
        text: format!(
            "We received a request to reset your password. Choose a new one here: {reset_url}\n\nThis link expires in {}. If you didn't ask for this, you can ignore this email and your password won't change.",
            TokenPurpose::PasswordReset.lifetime_text()
        ),
        html: format!(
            "<p>We received a request to reset your password. <a href=\"{reset_url}\">Choose a new password</a>.</p><p>This link expires in {}. If you didn't ask for this, you can ignore this email and your password won't change.</p>",
            TokenPurpose::PasswordReset.lifetime_text()
        ),
    }
}
