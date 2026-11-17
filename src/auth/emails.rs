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

pub fn email_change_email(recipient: &str, confirm_url: &str) -> Email {
    let expiry = format!(
        "This link expires in {}. If you didn't ask for this, you can ignore this email and nothing will change.",
        TokenPurpose::EmailChange.lifetime_text()
    );
    EmailTemplate::new("Confirm your new email")
        .paragraph(vec![
            Part::Text("You asked to sign in with this email address from now on. "),
            Part::Link {
                text: "Confirm your new email",
                url: confirm_url,
            },
        ])
        .text(&expiry)
        .to(recipient)
}

pub fn email_in_use_email(recipient: &str) -> Email {
    EmailTemplate::new("This email already has an account")
        .text("Someone asked to move their account to this email address, but it already belongs to an account, so nothing was changed.")
        .text("If this wasn't you, you can ignore this email.")
        .to(recipient)
}

pub fn email_changed_email(recipient: &str, new_email: &str) -> Email {
    let notice = format!("Your account now signs in with {new_email} instead of this address.");
    EmailTemplate::new("Your email was changed")
        .text(&notice)
        .text("If you didn't make this change, someone else may know your password. Contact support right away so we can help you get your account back.")
        .to(recipient)
}

pub fn account_removed_email(recipient: &str) -> Email {
    EmailTemplate::new("Your account was deleted")
        .text("Our team has deleted your account and everything in it, and you've been signed out everywhere.")
        .text("If you think this was a mistake, reply to this email or contact support.")
        .to(recipient)
}

pub fn account_deleted_email(recipient: &str) -> Email {
    EmailTemplate::new("Your account was deleted")
        .text("Your account and everything in it have been deleted, and you've been signed out everywhere.")
        .text("If you didn't do this, someone else may know your password. Contact support right away.")
        .to(recipient)
}
