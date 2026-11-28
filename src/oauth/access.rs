use std::sync::Arc;

use sqlx::{PgExecutor, PgPool};
use time::OffsetDateTime;

use crate::action::{Caller, ConnectedApp};
use crate::oauth::app_label;
use crate::oauth::queries::{query_get_access_grant, query_record_connection_use};
use crate::oauth::secrets::SecretKind;
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;

pub fn is_app_token(raw: &str) -> bool {
    SecretKind::AccessToken.hash_of(raw).is_some()
}

pub async fn app_caller(
    raw: &str,
    now: OffsetDateTime,
    db: &PgPool,
) -> Result<Option<Caller>, sqlx::Error> {
    let Some(hash) = SecretKind::AccessToken.hash_of(raw) else {
        return Ok(None);
    };
    let Some(grant) = query_get_access_grant(&hash, now, db).await? else {
        return Ok(None);
    };
    let Some(user) = active_owner(grant.user_id, db).await? else {
        return Ok(None);
    };
    let scopes = grant
        .scopes
        .into_iter()
        .filter(|scope| scope.available_to(&user) && scope.offered_by_sign_in())
        .collect();
    let app = ConnectedApp {
        connection_id: grant.connection_id,
        scopes,
        label: app_label(&grant.name, &grant.return_host),
    };
    Ok(Some(Caller::connected_app(Arc::new(user), app)))
}

pub async fn active_owner(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> Result<Option<User>, sqlx::Error> {
    let user = query_get_user_by_id(user_id, db).await?;
    Ok(user.filter(User::is_active))
}

pub async fn record_app_use(
    connection_id: i32,
    now: OffsetDateTime,
    db: impl PgExecutor<'_>,
) -> Result<(), sqlx::Error> {
    query_record_connection_use(connection_id, now, db).await
}
