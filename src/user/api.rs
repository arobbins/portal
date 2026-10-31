use leptos::prelude::*;

use crate::user::model::AccountUser;

#[cfg(feature = "ssr")]
mod ssr {
    use leptos::prelude::*;
    use sqlx::PgPool;
    use tower_sessions::Session;

    use crate::web::AppState;

    pub fn state() -> Result<AppState, ServerFnError> {
        use_context::<AppState>().ok_or_else(|| ServerFnError::new("Server state is unavailable."))
    }

    pub fn db() -> Result<PgPool, ServerFnError> {
        state().map(|state| state.db)
    }

    pub async fn session() -> Result<Session, ServerFnError> {
        leptos_axum::extract::<Session>()
            .await
            .map_err(|e| internal("Failed to read session", e))
    }

    pub fn internal(context: &str, error: impl std::fmt::Display) -> ServerFnError {
        tracing::error!("{context}: {error}");
        ServerFnError::new("Something went wrong. Please try again.")
    }
}

#[server]
pub async fn login(email: String, password: String) -> Result<(), ServerFnError> {
    use argon2::{Argon2, PasswordVerifier};
    use password_hash::PasswordHash;

    use crate::user::queries::query_get_user_by_email;
    use crate::utils::normalize_email;
    use crate::web::CurrentUser;

    let db = ssr::db()?;
    let session = ssr::session().await?;
    let invalid = || ServerFnError::new("Invalid email or password.");

    let email = normalize_email(&email);
    let user = query_get_user_by_email(&email, &db)
        .await
        .map_err(|e| ssr::internal("Failed to look up user", e))?;

    let Some(user) = user else {
        return Err(invalid());
    };

    let parsed_hash = PasswordHash::new(&user.password).map_err(|_| invalid())?;

    if Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(invalid());
    }

    if user.verified_at.is_none() {
        return Err(ServerFnError::new("Please verify your email before logging in."));
    }

    session
        .cycle_id()
        .await
        .map_err(|e| ssr::internal("Failed to cycle session", e))?;
    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ssr::internal("Failed to store user id", e))?;
    session
        .insert("current_user", CurrentUser { name: user.name })
        .await
        .map_err(|e| ssr::internal("Failed to store current user", e))?;

    leptos_axum::redirect("/account");
    Ok(())
}

#[server]
pub async fn register(
    name: String,
    email: String,
    new_password: String,
    confirm_password: String,
) -> Result<String, ServerFnError> {
    use argon2::{Argon2, PasswordHasher};
    use password_hash::SaltString;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::env;
    use tower_sessions::cookie::time;

    use crate::user::queries::{query_insert_email_verification_token, query_insert_user, NewUser};
    use crate::user::service::send_register_verification_email;
    use crate::utils::{get_token_hash_from_token_string, normalize_email};

    let db = ssr::db()?;
    let email = normalize_email(&email);
    let name = name.trim();

    if name.is_empty() || email.is_empty() || new_password.is_empty() || confirm_password.is_empty() {
        return Err(ServerFnError::new("Missing required values."));
    }

    if new_password != confirm_password {
        return Err(ServerFnError::new("Passwords do not match."));
    }

    let salt = SaltString::generate(rand::thread_rng());
    let password_hash = Argon2::default()
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(|_| ServerFnError::new("Failed to hash password."))?
        .to_string();

    let new_user = NewUser {
        name,
        email: &email,
        password_hash: &password_hash,
    };

    let failed = || ServerFnError::new("Registration failed. Please try again.");

    let mut tx = db.begin().await.map_err(|e| {
        tracing::error!("Failed to start registration transaction: {}", e);
        failed()
    })?;

    let user_id = query_insert_user(&new_user, &mut *tx).await.map_err(|e| {
        tracing::error!("Registration failed: {}", e);
        ServerFnError::new("Registration failed. Email may already be in use.")
    })?;

    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let token_hash = get_token_hash_from_token_string(&token);
    let expires_at = time::OffsetDateTime::now_utc() + time::Duration::hours(24);

    query_insert_email_verification_token(user_id, &token_hash, expires_at, &mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to store verification token: {}", e);
            failed()
        })?;

    let app_base_url = env::var("APP_BASE_URL").map_err(|e| ssr::internal("APP_BASE_URL is not set", e))?;
    let verify_url = format!("{app_base_url}/user/verify/{token}");

    send_register_verification_email(&email, &verify_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send verification email: {}", e);
            ServerFnError::new("Could not send verification email. Please try again.")
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit registration: {}", e);
        failed()
    })?;

    Ok("Account created! Check your email to confirm your account.".to_string())
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let session = ssr::session().await?;

    session
        .flush()
        .await
        .map_err(|e| ssr::internal("Failed to clear session", e))?;

    leptos_axum::redirect("/login");
    Ok(())
}

#[server]
pub async fn get_account_user() -> Result<Option<AccountUser>, ServerFnError> {
    use crate::user::auth::AuthenticatedUser;

    let state = ssr::state()?;
    let user: Option<AuthenticatedUser> = leptos_axum::extract_with_state(&state)
        .await
        .map_err(|e| ssr::internal("Failed to read signed-in user", e))?;

    match user {
        Some(AuthenticatedUser(user)) => Ok(Some(user.into())),
        None => {
            leptos_axum::redirect("/login");
            Ok(None)
        }
    }
}
