use sqlx::PgExecutor;
use time::OffsetDateTime;

use super::log::AdminLogAction;
use crate::user::model::Role;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserListing {
    pub public_id: String,
    pub name: String,
    pub email: String,
    pub role: Role,
    pub status: ListedStatus,
    pub created_at: OffsetDateTime,
    pub last_active_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum ListedStatus {
    Active,
    AwaitingPassword,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCounts {
    pub created_at: OffsetDateTime,
    pub passkeys: i64,
    pub devices: i64,
    pub api_tokens: i64,
}

pub async fn query_search_users(
    search: &str,
    limit: i64,
    offset: i64,
    db: impl PgExecutor<'_>,
) -> Result<Vec<UserListing>, sqlx::Error> {
    sqlx::query_as!(
        UserListing,
        r#"SELECT u.public_id, u.name, u.email, u.role AS "role: Role",
            CASE WHEN u.disabled_at IS NOT NULL THEN 'disabled' WHEN u.verified_at IS NULL THEN 'awaiting_password' ELSE 'active' END AS "status!: ListedStatus",
            u.created_at, GREATEST(
                (SELECT max(d.last_seen_at) FROM user_devices d WHERE d.user_id = u.id),
                (SELECT max(e.created_at) FROM account_events e WHERE e.user_id = u.id AND e.kind = 'signed_in')
            ) AS last_active_at
        FROM users u
        WHERE $1 = '' OR strpos(lower(u.email), lower($1)) > 0 OR strpos(lower(u.name), lower($1)) > 0
        ORDER BY u.created_at DESC, u.id DESC
        LIMIT $2 OFFSET $3"#,
        search,
        limit,
        offset,
    )
    .fetch_all(db)
    .await
}

pub async fn query_count_user_things(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<UserCounts, sqlx::Error> {
    sqlx::query_as!(
        UserCounts,
        r#"SELECT u.created_at,
            (SELECT count(*) FROM user_passkeys p WHERE p.user_id = u.id) AS "passkeys!",
            (SELECT count(*) FROM user_devices d WHERE d.user_id = u.id AND d.session_version = u.session_version) AS "devices!",
            (SELECT count(*) FROM api_tokens t WHERE t.user_id = u.id AND t.expires_at > now()) AS "api_tokens!"
        FROM users u WHERE u.id = $1"#,
        user_id,
    )
    .fetch_one(db)
    .await
}

pub async fn query_delete_all_user_tokens(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM user_tokens WHERE user_id = $1", user_id)
        .execute(db)
        .await?;

    Ok(())
}

pub struct NewLogEntry<'a> {
    pub admin_public_id: Option<&'a str>,
    pub admin_email: Option<&'a str>,
    pub action: AdminLogAction,
    pub target_public_id: &'a str,
    pub target_email: &'a str,
    pub ip: Option<String>,
}

pub async fn query_insert_admin_log(
    entry: &NewLogEntry<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO admin_log (admin_public_id, admin_email, action, target_public_id, target_email, ip) VALUES ($1, $2, $3, $4, $5, $6)",
        entry.admin_public_id,
        entry.admin_email,
        entry.action as AdminLogAction,
        entry.target_public_id,
        entry.target_email,
        entry.ip,
    )
    .execute(db)
    .await?;

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub admin_email: Option<String>,
    pub action: AdminLogAction,
    pub target_public_id: String,
    pub target_email: String,
    pub target_exists: bool,
    pub created_at: OffsetDateTime,
}

pub async fn query_list_admin_log(
    limit: i64,
    db: impl PgExecutor<'_>,
) -> Result<Vec<LogEntry>, sqlx::Error> {
    sqlx::query_as!(
        LogEntry,
        r#"SELECT l.admin_email, l.action AS "action: AdminLogAction", l.target_public_id, l.target_email,
            EXISTS (SELECT 1 FROM users u WHERE u.public_id = l.target_public_id) AS "target_exists!",
            l.created_at
        FROM admin_log l
        ORDER BY l.created_at DESC, l.id DESC
        LIMIT $1"#,
        limit,
    )
    .fetch_all(db)
    .await
}
