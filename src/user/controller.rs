use crate::user::queries::*;
use crate::utils::*;
use crate::web::CurrentUser;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use sqlx::PgPool;
use tower_sessions::Session;

pub async fn verify_user_controller(
    State(db): State<PgPool>,
    session: Session,
    Path(token): Path<String>,
) -> Response {
    let token_hash = get_token_hash_from_token_string(&token);

    let user_id = query_get_user_id_by_token_hash(&db, &token_hash)
        .await
        .unwrap();

    let Some(user_id) = user_id else {
        return (
            StatusCode::BAD_REQUEST,
            html_error("This verification link is invalid or has expired"),
        )
            .into_response();
    };

    let user = query_update_user_verified_time(user_id, &db).await.unwrap();
    query_delete_email_verification_token(user_id, &db)
        .await
        .unwrap();

    session.cycle_id().await.unwrap();
    session.insert("user_id", user_id).await.unwrap();
    session
        .insert("current_user", CurrentUser { name: user.name })
        .await
        .unwrap();

    Redirect::to("/account?verified=true").into_response()
}
