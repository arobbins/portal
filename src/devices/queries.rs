use sqlx::PgExecutor;
use time::OffsetDateTime;

use crate::user::model::{Role, User};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub public_id: String,
    pub user_agent: String,
    pub ip: Option<String>,
    pub created_at: OffsetDateTime,
    pub last_seen_at: OffsetDateTime,
}

pub struct NewDevice<'a> {
    pub public_id: &'a str,
    pub user_id: i32,
    pub session_version: i32,
    pub user_agent: &'a str,
    pub ip: Option<String>,
}

pub struct SignedInDevice {
    pub user: User,
    pub last_seen_at: OffsetDateTime,
}

pub async fn query_insert_device(
    device: &NewDevice<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO user_devices (public_id, user_id, session_version, user_agent, ip) VALUES ($1, $2, $3, $4, $5)",
        device.public_id,
        device.user_id,
        device.session_version,
        device.user_agent,
        device.ip,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_get_signed_in_device(
    user_id: i32,
    public_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<SignedInDevice>, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT u.id, u.name, u.email, u.password, u.role AS "role: Role", u.verified_at, u.session_version, u.webauthn_id, u.public_id, u.disabled_at, u.admin_granted_at, d.last_seen_at
        FROM user_devices d JOIN users u ON u.id = d.user_id
        WHERE d.public_id = $1 AND d.user_id = $2 AND d.session_version = u.session_version AND u.disabled_at IS NULL"#,
        public_id,
        user_id,
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|row| SignedInDevice {
        user: User {
            id: row.id,
            name: row.name,
            email: row.email,
            password: row.password,
            role: row.role,
            verified_at: row.verified_at,
            session_version: row.session_version,
            webauthn_id: row.webauthn_id,
            public_id: row.public_id,
            disabled_at: row.disabled_at,
            admin_granted_at: row.admin_granted_at,
        },
        last_seen_at: row.last_seen_at,
    }))
}

pub async fn query_touch_device(
    public_id: &str,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE user_devices SET last_seen_at = $1 WHERE public_id = $2",
        now,
        public_id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_move_device_to_version(
    public_id: &str,
    user: &User,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE user_devices SET session_version = $1 WHERE public_id = $2 AND user_id = $3",
        user.session_version,
        public_id,
        user.id,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn query_list_devices(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Vec<Device>, sqlx::Error> {
    sqlx::query_as!(
        Device,
        "SELECT d.public_id, d.user_agent, d.ip, d.created_at, d.last_seen_at FROM user_devices d JOIN users u ON u.id = d.user_id WHERE d.user_id = $1 AND d.session_version = u.session_version ORDER BY d.last_seen_at DESC",
        user_id,
    )
    .fetch_all(db)
    .await
}

pub async fn query_delete_device(
    user_id: i32,
    public_id: &str,
    db: impl PgExecutor<'_>,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM user_devices d USING users u WHERE u.id = d.user_id AND d.user_id = $1 AND d.public_id = $2 AND d.session_version = u.session_version RETURNING d.user_agent",
        user_id,
        public_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_delete_other_devices(
    user_id: i32,
    keep: &str,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM user_devices WHERE user_id = $1 AND public_id <> $2",
        user_id,
        keep,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

pub async fn query_delete_stale_devices(
    idle_since: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM user_devices d USING users u WHERE u.id = d.user_id AND (d.last_seen_at < $1 OR d.session_version <> u.session_version)",
        idle_since,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected())
}

pub async fn query_delete_user_devices(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM user_devices WHERE user_id = $1", user_id)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}
