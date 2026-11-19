use super::queries::query_list_passkeys;
use crate::data_export::{Dataset, maybe_utc, utc, yes_no};
use crate::env::Env;
use crate::user::model::User;

pub async fn passkeys_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let passkeys = query_list_passkeys(user.id, env.db()).await?;
    Ok(Dataset {
        file: "passkeys.csv",
        about: "Passkeys you can sign in with. The keys themselves stay on your devices and aren't included.",
        tables: &["user_passkeys"],
        columns: &["name", "added_at", "last_used_at", "backed_up"],
        rows: passkeys
            .into_iter()
            .map(|passkey| {
                vec![
                    passkey.name,
                    utc(passkey.created_at),
                    maybe_utc(passkey.last_used_at),
                    yes_no(passkey.backed_up),
                ]
            })
            .collect(),
    })
}
