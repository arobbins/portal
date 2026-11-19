use crate::data_export::{Dataset, maybe_utc, utc};
use crate::env::Env;
use crate::user::model::User;
use crate::user::queries::query_get_user_created_at;

pub async fn profile_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let created_at = query_get_user_created_at(user.id, env.db()).await?;
    Ok(Dataset {
        file: "profile.csv",
        about: "Your account: who you are and how we reach you.",
        tables: &["users"],
        columns: &[
            "id",
            "name",
            "email",
            "time_zone",
            "created_at",
            "email_confirmed_at",
        ],
        rows: vec![vec![
            user.public_id.clone(),
            user.name.clone(),
            user.email.clone(),
            user.time_zone.clone().unwrap_or_default(),
            utc(created_at),
            maybe_utc(user.verified_at),
        ]],
    })
}
