use super::queries::query_list_api_tokens;
use crate::data_export::{Dataset, maybe_utc, utc};
use crate::env::Env;
use crate::user::model::User;

pub async fn api_tokens_export(env: &Env<'_>, user: &User) -> Result<Dataset, sqlx::Error> {
    let tokens = query_list_api_tokens(user.id, env.db()).await?;
    Ok(Dataset {
        file: "api_tokens.csv",
        about: "API tokens you created. The tokens themselves are never stored, so only their last four characters are included.",
        tables: &["api_tokens"],
        columns: &[
            "name",
            "used_by",
            "scopes",
            "last_four",
            "created_at",
            "expires_at",
            "last_used_at",
        ],
        rows: tokens
            .into_iter()
            .map(|token| {
                let scopes: Vec<&str> = token.scopes.iter().map(|scope| scope.as_str()).collect();
                vec![
                    token.name,
                    token.audience.label().to_string(),
                    scopes.join(" "),
                    token.last_four,
                    utc(token.created_at),
                    utc(token.expires_at),
                    maybe_utc(token.last_used_at),
                ]
            })
            .collect(),
    })
}
