use crate::mail::{Email, EmailTemplate, Part};

pub fn api_token_created_email(recipient: &str, token_name: &str, developer_url: &str) -> Email {
    let created = format!(
        "A new API token ({token_name}) was created for your account. Apps that have it can use your account through the API."
    );
    EmailTemplate::new("A new API token was created")
        .text(&created)
        .paragraph(vec![
            Part::Text("If this wasn't you, "),
            Part::Link {
                text: "revoke it",
                url: developer_url,
            },
            Part::Text(" and change your password."),
        ])
        .to(recipient)
}
