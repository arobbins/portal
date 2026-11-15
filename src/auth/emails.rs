use crate::auth::links::TokenPurpose;
use crate::mail::{Email, EmailTemplate, Part};

pub fn register_verification_email(recipient: &str, verify_url: &str) -> Email {
    let expiry = format!(
        "This link expires in {}.",
        TokenPurpose::EmailVerification.lifetime_text()
    );
    EmailTemplate::new("Confirm your email")
        .paragraph(vec![
            Part::Text("Welcome! "),
            Part::Link {
                text: "Confirm your email and set your password",
                url: verify_url,
            },
        ])
        .text(&expiry)
        .to(recipient)
}

pub fn account_exists_email(recipient: &str, login_url: &str) -> Email {
    EmailTemplate::new("You already have an account")
        .paragraph(vec![
            Part::Text(
                "Someone tried to register with this email address, but you already have an account. ",
            ),
            Part::Link { text: "Sign in", url: login_url },
        ])
        .text("If this wasn't you, you can ignore this email.")
        .to(recipient)
}

pub fn password_reset_email(recipient: &str, reset_url: &str) -> Email {
    let expiry = format!(
        "This link expires in {}. If you didn't ask for this, you can ignore this email and your password won't change.",
        TokenPurpose::PasswordReset.lifetime_text()
    );
    EmailTemplate::new("Reset your password")
        .paragraph(vec![
            Part::Text("We received a request to reset your password. "),
            Part::Link {
                text: "Choose a new password",
                url: reset_url,
            },
        ])
        .text(&expiry)
        .to(recipient)
}
