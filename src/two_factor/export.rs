use super::queries::query_get_two_factor;
use crate::data_export::{Dataset, maybe_utc, yes_no};
use crate::env::Env;
use crate::user::model::User;

pub async fn two_factor_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let enabled_at = query_get_two_factor(user.id, env.db())
        .await?
        .map(|row| row.enabled_at);
    Ok(Dataset {
        file: "two_factor.csv",
        about: "Whether two-factor authentication is on. Its secret and recovery codes aren't included.",
        tables: &["user_two_factor"],
        columns: &["enabled", "enabled_at"],
        rows: vec![vec![yes_no(enabled_at.is_some()), maybe_utc(enabled_at)]],
    })
}
