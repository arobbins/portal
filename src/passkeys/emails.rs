use crate::mail::{Email, EmailTemplate, Part};

pub fn passkey_added_email(recipient: &str, passkey_name: &str, security_url: &str) -> Email {
    let added = format!(
        "A new passkey ({passkey_name}) was added to your account. You can now use it to sign in."
    );
    EmailTemplate::new("A passkey was added to your account")
        .text(&added)
        .paragraph(vec![
            Part::Text("If this wasn't you, "),
            Part::Link {
                text: "change your password and remove the passkey",
                url: security_url,
            },
        ])
        .to(recipient)
}
