use super::actions::activity_history;
use crate::data_export::{Dataset, utc};
use crate::env::Env;
use crate::user::model::User;

pub async fn activity_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let activity = activity_history(env, user.id).await?;
    Ok(Dataset {
        file: "activity.csv",
        about: "Everything recorded about your account: sign-ins and changes, newest first.",
        tables: &["account_events"],
        columns: &["what", "at", "how", "ip_address", "details"],
        rows: activity
            .into_iter()
            .map(|event| {
                vec![
                    event.summary,
                    utc(event.at),
                    event.by.unwrap_or_else(|| "In the browser".to_string()),
                    event.ip.unwrap_or_default(),
                    event.device.unwrap_or_default(),
                ]
            })
            .collect(),
    })
}
