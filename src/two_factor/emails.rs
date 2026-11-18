use crate::mail::{Email, EmailTemplate, Part};

pub fn two_factor_enabled_email(recipient: &str, sign_in_url: &str) -> Email {
    EmailTemplate::new("Two-factor authentication is on")
        .text("Signing in to your account with a password now also needs a code from your authenticator app or a recovery code.")
        .paragraph(vec![
            Part::Text("If you didn't do this, "),
            Part::Link {
                text: "review your sign-in settings",
                url: sign_in_url,
            },
            Part::Text(" and contact support right away."),
        ])
        .to(recipient)
}

pub fn two_factor_disabled_email(recipient: &str, sign_in_url: &str) -> Email {
    EmailTemplate::new("Two-factor authentication is off")
        .text("Signing in to your account with a password no longer needs a code from your authenticator app.")
        .paragraph(vec![
            Part::Text("If you didn't do this, "),
            Part::Link {
                text: "change your password and turn it back on",
                url: sign_in_url,
            },
            Part::Text(" right away."),
        ])
        .to(recipient)
}
