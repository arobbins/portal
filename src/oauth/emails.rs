use crate::account::tabs::AccountSection;
use crate::api::scope::ApiScope;
use crate::env::Env;
use crate::mail::{Email, EmailTemplate, Part};
use crate::oauth::app_label;

pub fn app_connected_email(
    env: &Env<'_>,
    recipient: &str,
    name: &str,
    return_host: &str,
    scopes: &[ApiScope],
) -> Email {
    let subject = format!("{name} was connected to your account");
    let returns_to = format!("Returns to {return_host}");
    let label = app_label(name, return_host);
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
        .text(&returns_to)
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

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;
    use crate::env::App;

    #[sqlx::test]
    async fn the_email_leads_with_where_the_app_returns_to(db: PgPool) {
        let app = App::for_tests(db);
        let scopes = [ApiScope::ProfileRead];

        let email = app_connected_email(
            &app.env(None),
            "owner@example.test",
            "Claude",
            "evil.test",
            &scopes,
        );

        assert_eq!(email.subject, "Claude was connected to your account");
        assert_eq!(
            email.text.split("\n\n").next(),
            Some("Returns to evil.test")
        );
        assert!(email.html.starts_with("<p>Returns to evil.test</p>"));
    }
}
