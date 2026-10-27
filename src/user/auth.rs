use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use axum::extract::{FromRef, FromRequestParts, OptionalFromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use sqlx::PgPool;
use tower_sessions::Session;

pub struct AuthenticatedUser(pub User);

impl<S> OptionalFromRequestParts<S> for AuthenticatedUser
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|e| e.into_response())?;

        let user_id: Option<i32> = session
            .get("user_id")
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

        let Some(user_id) = user_id else {
            return Ok(None);
        };

        let db = PgPool::from_ref(state);

        let user = query_get_user_by_id(&user_id, &db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

        Ok(user.map(AuthenticatedUser))
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        <Self as OptionalFromRequestParts<S>>::from_request_parts(parts, state)
            .await?
            .ok_or_else(|| Redirect::to("/login").into_response())
    }
}
