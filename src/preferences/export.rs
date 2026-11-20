use super::newsletters::newsletters_of;
use super::notifications::{NotificationTopic, wants};
use crate::data_export::{Dataset, yes_no};
use crate::env::Env;
use crate::user::model::User;

pub async fn notifications_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let mut rows = Vec::new();
    for topic in NotificationTopic::ALL {
        let on = wants(user.id, topic, env.db()).await?;
        rows.push(vec![topic.label().to_string(), yes_no(on)]);
    }
    for (newsletter, on) in newsletters_of(user.id, env.db()).await? {
        rows.push(vec![newsletter.label().to_string(), yes_no(on)]);
    }
    Ok(Dataset {
        file: "notifications.csv",
        about: "Optional emails and newsletters, and whether you get them. Security emails are always sent.",
        tables: &["notification_opt_outs", "newsletter_opt_outs"],
        columns: &["email", "on"],
        rows,
    })
}
