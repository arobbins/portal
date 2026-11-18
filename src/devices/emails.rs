use crate::mail::{Email, EmailTemplate, Part};

pub struct SignInNotice<'a> {
    pub device: &'a str,
    pub ip: Option<&'a str>,
    pub when: &'a str,
    pub devices_url: &'a str,
    pub settings_url: &'a str,
}

pub fn new_sign_in_email(recipient: &str, notice: &SignInNotice<'_>) -> Email {
    let place = notice.ip.map_or_else(String::new, |ip| format!(" at {ip}"));
    let summary = format!(
        "Your account was signed in to from {}{place} on {}.",
        notice.device, notice.when
    );
    EmailTemplate::new("New sign-in to your account")
        .text(&summary)
        .paragraph(vec![
            Part::Text("If this was you, there's nothing to do. If it wasn't, "),
            Part::Link {
                text: "sign out that device and change your password",
                url: notice.devices_url,
            },
            Part::Text("."),
        ])
        .paragraph(vec![
            Part::Text("You get these emails because new sign-in alerts are on. "),
            Part::Link {
                text: "Change your notification settings",
                url: notice.settings_url,
            },
            Part::Text("."),
        ])
        .to(recipient)
}
