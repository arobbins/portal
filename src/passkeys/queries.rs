use sqlx::PgExecutor;
use sqlx::types::Json;
use webauthn_rs::prelude::Passkey as WebauthnPasskey;

#[derive(Debug)]
pub struct Passkey {
    pub id: i32,
    pub public_id: String,
    pub credential_id: Vec<u8>,
    pub passkey: Json<WebauthnPasskey>,
    pub name: String,
    pub backup_eligible: bool,
    pub backed_up: bool,
    pub created_at: time::OffsetDateTime,
    pub last_used_at: Option<time::OffsetDateTime>,
}

pub struct NewPasskey<'a> {
    pub user_id: i32,
    pub public_id: &'a str,
    pub passkey: &'a WebauthnPasskey,
    pub name: &'a str,
    pub aaguid: Option<uuid::Uuid>,
    pub backup_eligible: bool,
    pub backed_up: bool,
}

pub async fn query_list_passkeys(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Vec<Passkey>, sqlx::Error> {
    sqlx::query_as!(
        Passkey,
        "SELECT id, public_id, credential_id, passkey AS \"passkey: Json<WebauthnPasskey>\", name, backup_eligible, backed_up, created_at, last_used_at FROM user_passkeys WHERE user_id = $1 ORDER BY created_at",
        user_id,
    )
    .fetch_all(db)
    .await
}

pub async fn query_user_has_passkey(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM user_passkeys WHERE user_id = $1) AS "exists!""#,
        user_id,
    )
    .fetch_one(db)
    .await
}

pub async fn query_get_passkey_by_credential(
    user_id: i32,
    credential_id: &[u8],
    db: impl PgExecutor<'_>,
) -> Result<Option<Passkey>, sqlx::Error> {
    sqlx::query_as!(
        Passkey,
        "SELECT id, public_id, credential_id, passkey AS \"passkey: Json<WebauthnPasskey>\", name, backup_eligible, backed_up, created_at, last_used_at FROM user_passkeys WHERE user_id = $1 AND credential_id = $2",
        user_id,
        credential_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_insert_passkey(
    new_passkey: &NewPasskey<'_>,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO user_passkeys (user_id, public_id, credential_id, passkey, name, aaguid, backup_eligible, backed_up) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        new_passkey.user_id,
        new_passkey.public_id,
        new_passkey.passkey.cred_id().as_slice(),
        Json(new_passkey.passkey) as _,
        new_passkey.name,
        new_passkey.aaguid,
        new_passkey.backup_eligible,
        new_passkey.backed_up,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_record_passkey_use(
    id: i32,
    passkey: &WebauthnPasskey,
    backed_up: bool,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE user_passkeys SET passkey = $1, backed_up = $2, last_used_at = now() WHERE id = $3",
        Json(passkey) as _,
        backed_up,
        id,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_rename_passkey(
    public_id: &str,
    user_id: i32,
    name: &str,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE user_passkeys SET name = $1 WHERE public_id = $2 AND user_id = $3",
        name,
        public_id,
        user_id,
    )
    .execute(db)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn query_delete_passkey(
    public_id: &str,
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM user_passkeys WHERE public_id = $1 AND user_id = $2 RETURNING name",
        public_id,
        user_id,
    )
    .fetch_optional(db)
    .await
}

pub async fn query_get_passkey_prompt_due(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let due = sqlx::query_scalar!(
        "SELECT (passkey_prompt_after IS NULL OR passkey_prompt_after <= now()) AND NOT EXISTS (SELECT 1 FROM user_passkeys WHERE user_passkeys.user_id = users.id) AS \"due!\" FROM users WHERE id = $1",
        user_id,
    )
    .fetch_optional(db)
    .await?;

    Ok(due.unwrap_or(false))
}

pub async fn query_snooze_passkey_prompt(
    user_id: i32,
    days: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET passkey_prompt_after = now() + make_interval(days => $2) WHERE id = $1",
        user_id,
        days,
    )
    .execute(db)
    .await?;

    Ok(())
}

pub async fn query_decline_passkey_prompt(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET passkey_prompt_after = 'infinity' WHERE id = $1",
        user_id,
    )
    .execute(db)
    .await?;

    Ok(())
}
