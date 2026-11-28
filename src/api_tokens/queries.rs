use crate::api::scope::{ApiScope, TokenAudience};
use sqlx::PgExecutor;

pub struct NewApiTokenRow<'a> {
    pub user_id: i32,
    pub public_id: &'a str,
    pub name: &'a str,
    pub token_hash: &'a str,
    pub last_four: &'a str,
    pub scopes: &'a [ApiScope],
    pub audience: TokenAudience,
    pub expires_at: time::OffsetDateTime,
    pub max_per_user: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiToken {
    pub public_id: String,
    pub name: String,
    pub last_four: String,
    pub scopes: Vec<ApiScope>,
    pub audience: TokenAudience,
    pub created_at: time::OffsetDateTime,
    pub expires_at: time::OffsetDateTime,
    pub last_used_at: Option<time::OffsetDateTime>,
}

#[derive(Debug)]
pub struct ApiTokenGrant {
    pub id: i32,
    pub user_id: i32,
    pub scopes: Vec<ApiScope>,
    pub audience: TokenAudience,
}

pub async fn query_insert_api_token(
    token: &NewApiTokenRow<'_>,
    db: impl PgExecutor<'_>,
) -> Result<Option<i32>, sqlx::Error> {
    let id = sqlx::query_scalar!(
        "INSERT INTO api_tokens (user_id, public_id, name, token_hash, last_four, scopes, audience, expires_at) SELECT $1, $2, $3, $4, $5, $6, $7, $8 WHERE (SELECT count(*) FROM api_tokens WHERE user_id = $1) < $9 RETURNING id",
        token.user_id,
        token.public_id,
        token.name,
        token.token_hash,
        token.last_four,
        token.scopes as &[ApiScope],
        token.audience as TokenAudience,
        token.expires_at,
        token.max_per_user,
    )
    .fetch_optional(db)
    .await?;

    Ok(id)
}

pub async fn query_list_api_tokens(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Vec<ApiToken>, sqlx::Error> {
    sqlx::query_as!(
        ApiToken,
        r#"SELECT public_id, name, last_four, scopes AS "scopes: Vec<ApiScope>", audience AS "audience: TokenAudience", created_at, expires_at, last_used_at FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC"#,
        user_id,
    )
    .fetch_all(db)
    .await
}

pub async fn query_get_api_token_by_hash(
    token_hash: &str,
    now: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<Option<ApiTokenGrant>, sqlx::Error> {
    let grant = sqlx::query_as!(
        ApiTokenGrant,
        r#"SELECT id, user_id, scopes AS "scopes: Vec<ApiScope>", audience AS "audience: TokenAudience" FROM api_tokens WHERE token_hash = $1 AND expires_at > $2"#,
        token_hash,
        now,
    )
    .fetch_optional(db)
    .await?;

    Ok(grant)
}

pub async fn query_record_api_token_use(
    id: i32,
    now: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE api_tokens SET last_used_at = $2 WHERE id = $1 AND (last_used_at IS NULL OR last_used_at < $2::timestamptz - interval '5 minutes')",
        id,
        now,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_delete_api_token(
    public_id: &str,
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<String>, sqlx::Error> {
    let name = sqlx::query_scalar!(
        "DELETE FROM api_tokens WHERE public_id = $1 AND user_id = $2 RETURNING name",
        public_id,
        user_id,
    )
    .fetch_optional(db)
    .await?;

    Ok(name)
}

pub async fn query_delete_user_api_tokens(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM api_tokens WHERE user_id = $1", user_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

pub async fn query_delete_expired_api_tokens(
    cutoff: time::OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM api_tokens WHERE expires_at < $1", cutoff)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

pub async fn query_get_api_token_name(
    id: i32,
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT name FROM api_tokens WHERE id = $1 AND user_id = $2",
        id,
        user_id,
    )
    .fetch_optional(db)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_tokens::testing::issue_token;
    use crate::api_tokens::token::generate_api_token;
    use crate::user::queries::{NewUser, query_insert_user};
    use sqlx::PgPool;

    async fn user(db: &PgPool) -> i32 {
        let new_user = NewUser {
            name: "Owner",
            email: "owner@example.test",
        };
        query_insert_user(&new_user, db).await.unwrap().unwrap()
    }

    #[sqlx::test]
    async fn a_token_cannot_be_revoked_by_another_user(db: PgPool) {
        let user_id = user(&db).await;
        let token = issue_token(user_id, &["profile:read"], time::Duration::days(1), &db).await;

        assert_eq!(
            query_delete_api_token(&token.public_id, user_id + 1, &db)
                .await
                .unwrap(),
            None
        );
    }

    #[sqlx::test]
    async fn each_user_has_a_token_limit(db: PgPool) {
        let user_id = user(&db).await;
        let scopes = vec![ApiScope::ProfileRead];
        let insert = |public_id: &'static str| {
            let token = generate_api_token();
            let db = db.clone();
            let scopes = scopes.clone();
            async move {
                let row = NewApiTokenRow {
                    user_id,
                    public_id,
                    name: "Limited",
                    token_hash: &token.hash,
                    last_four: &token.last_four,
                    scopes: &scopes,
                    audience: TokenAudience::Api,
                    expires_at: time::OffsetDateTime::now_utc() + time::Duration::days(1),
                    max_per_user: 1,
                };
                query_insert_api_token(&row, &db).await.unwrap()
            }
        };

        assert!(insert("tok_aaaaaaaaaaaaaaaaaaaa").await.is_some());
        assert_eq!(insert("tok_bbbbbbbbbbbbbbbbbbbb").await, None);
    }
}
