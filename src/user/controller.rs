use crate::user::service::*;
use crate::user::queries::*;
use crate::utils::*;
use crate::web::CurrentUser;

use sqlx::PgPool;
use tower_sessions::Session;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use password_hash::{PasswordHash, SaltString};
use std::env;
use rand::Rng;
use rand::distributions::Alphanumeric;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use tower_sessions::cookie::time;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    name: String,
    email: String,
    new_password: String,
    confirm_password: String,
}

pub async fn verify_user_controller(
    State(db): State<PgPool>,
    session: Session,
    Path(token): Path<String>,
) -> Response {
    let token_hash = get_token_hash_from_token_string(&token);

    let user_id = query_get_user_id_by_token_hash(&db, &token_hash).await.unwrap();

    let Some(user_id) = user_id else {
        return (
            StatusCode::BAD_REQUEST,
            html_error("This verification link is invalid or has expired"),
        )
            .into_response();
    };

    let user = query_update_user_verified_time(user_id, &db).await.unwrap();
    query_delete_email_verification_token(user_id, &db).await.unwrap();

    session.cycle_id().await.unwrap();
    session.insert("user_id", user_id).await.unwrap();
    session.insert("current_user", CurrentUser { name: user.name }).await.unwrap();

    Redirect::to("/account?verified=true").into_response()
}

pub async fn login_user_controller(
    State(db): State<PgPool>,
    session: Session,
    Form(payload): Form<LoginRequest>,
) -> Response {
    let invalid = || {
        (
            StatusCode::UNAUTHORIZED,
            html_error("Invalid email or password."),
        )
            .into_response()
    };

    let email = normalize_email(&payload.email);
    let user = query_get_user_by_email(&email, &db).await.unwrap();

    let Some(user) = user else {
        return invalid();
    };

    let parsed_hash = match PasswordHash::new(&user.password) {
        Ok(hash) => hash,
        Err(_) => return invalid(),
    };

    if Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return invalid();
    }

    if user.verified_at.is_none() {
        return (
            StatusCode::FORBIDDEN,
            html_error("Please verify your email before logging in."),
        )
            .into_response();
    }

    session.cycle_id().await.unwrap();
    session.insert("user_id", user.id).await.unwrap();
    session.insert("current_user", CurrentUser { name: user.name.clone() }).await.unwrap();

    (StatusCode::OK, [("HX-Redirect", "/account")], Html("")).into_response()
}

pub async fn register_user_controller(
    State(db): State<PgPool>,
    Form(payload): Form<RegisterRequest>,
) -> Response {
    let email = normalize_email(&payload.email);
    let name = payload.name.trim();

    if name.is_empty() || email.is_empty() || payload.new_password.is_empty() || payload.confirm_password.is_empty() {
        return (StatusCode::BAD_REQUEST, html_error("Missing required values.")).into_response();
    }

    if payload.new_password != payload.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            html_error("Passwords do not match."),
        )
            .into_response();
    }

    let salt = SaltString::generate(rand::thread_rng());
    let argon2 = Argon2::default();
    let password_hash = match argon2.hash_password(payload.new_password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                html_error("Failed to hash password."),
            )
                .into_response()
        }
    };

    let new_user = NewUser {
        name,
        email: &email,
        password_hash: &password_hash,
    };

    let server_error = |message: &str| {
        (StatusCode::INTERNAL_SERVER_ERROR, html_error(message)).into_response()
    };

    let mut tx = match db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start registration transaction: {}", e);
            return server_error("Registration failed. Please try again.");
        }
    };

    let user_id = match query_insert_user(&new_user, &mut *tx).await {
        Ok(user_id) => user_id,
        Err(e) => {
            tracing::error!("Registration failed: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                html_error("Registration failed. Email may already be in use."),
            )
                .into_response();
        }
    };

    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let token_hash = get_token_hash_from_token_string(&token);
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(24);

    if let Err(e) = query_insert_email_verification_token(user_id, &token_hash, expires_at, &mut *tx).await {
        tracing::error!("Failed to store verification token: {}", e);
        return server_error("Registration failed. Please try again.");
    }

    let app_base_url = env::var("APP_BASE_URL").unwrap();
    let verify_url = format!("{app_base_url}/user/verify/{token}");

    if let Err(e) = send_register_verification_email(&email, &verify_url).await {
        tracing::error!("Failed to send verification email: {}", e);
        return server_error("Could not send verification email. Please try again.");
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit registration: {}", e);
        return server_error("Registration failed. Please try again.");
    }

    (
        StatusCode::OK,
        html_success("Account created! Check your email to confirm your account."),
    )
        .into_response()
}

pub async fn logout_user_controller(session: Session) -> Response {
    session.flush().await.unwrap();

    (StatusCode::OK, [("HX-Redirect", "/login")], Html("")).into_response()
}