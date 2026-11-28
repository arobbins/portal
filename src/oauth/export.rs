use crate::api::scope::scope_names;
use crate::data_export::{Dataset, maybe_utc, utc};
use crate::env::Env;
use crate::oauth::queries::query_list_connections;
use crate::user::model::User;

pub async fn connected_apps_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let connections = query_list_connections(user.id, env.db()).await?;
    Ok(Dataset {
        file: "connected_apps.csv",
        about: "Apps you connected by approving them, such as an AI assistant: the address they return to, what each may do (scope names separated by spaces), when you connected it and when it last acted. Their tokens are secrets and are never included.",
        tables: &["oauth_connections"],
        columns: &[
            "name",
            "returns_to",
            "scopes",
            "connected_at",
            "last_used_at",
        ],
        rows: connections
            .into_iter()
            .map(|connection| {
                vec![
                    connection.name,
                    connection.return_host,
                    scope_names(&connection.scopes),
                    utc(connection.created_at),
                    maybe_utc(connection.last_used_at),
                ]
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests;
