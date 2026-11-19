use crate::mail::{Email, EmailTemplate};

pub fn data_exported_email(recipient: &str) -> Email {
    EmailTemplate::new("Your account data was downloaded")
        .text("A copy of all your account data was just downloaded after confirming your password or passkey.")
        .text("If you didn't do this, change your password and sign out everywhere right away, then contact support.")
        .to(recipient)
}
