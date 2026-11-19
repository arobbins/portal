use super::queries::query_list_devices;
use super::user_agent::describe_user_agent;
use crate::data_export::{Dataset, utc};
use crate::env::Env;
use crate::user::model::User;

pub async fn devices_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let devices = query_list_devices(user.id, env.db()).await?;
    Ok(Dataset {
        file: "devices.csv",
        about: "Browsers that are signed in to your account right now.",
        tables: &["user_devices"],
        columns: &["browser", "ip_address", "first_seen_at", "last_seen_at"],
        rows: devices
            .into_iter()
            .map(|device| {
                vec![
                    describe_user_agent(&device.user_agent),
                    device.ip.unwrap_or_default(),
                    utc(device.created_at),
                    utc(device.last_seen_at),
                ]
            })
            .collect(),
    })
}
