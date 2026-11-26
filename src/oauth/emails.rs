use crate::account::tabs::AccountSection;
use crate::api::scope::ApiScope;
use crate::env::Env;
use crate::mail::{Email, EmailTemplate, Part};

pub fn app_connected_email(
    env: &Env<'_>,
    recipient: &str,
    name: &str,
    label: &str,
    scopes: &[ApiScope],
) -> Email {
    let subject = format!("{name} was connected to your account");
    let intro = format!("{label} can now use your account through the MCP endpoint.");
    let access = match scopes {
        [] => "It can't read or change anything in your account.".to_string(),
        scopes => {
            let labels: Vec<&str> = scopes.iter().map(|scope| scope.choice_label()).collect();
            format!("It may: {}.", labels.join("; "))
        }
    };
    let connected_apps = env.app.url(&AccountSection::ConnectedApps.path());
    EmailTemplate::new(&subject)
        .text(&intro)
        .text(&access)
        .paragraph(vec![
            Part::Text("If this wasn't you, "),
            Part::Link {
                text: "disconnect it in Connected apps",
                url: &connected_apps,
            },
            Part::Text(" and change your password."),
        ])
        .to(recipient)
}
