use std::env;

use argon2::{Argon2, PasswordHasher};
use password_hash::SaltString;
use rand::Rng;
use rand::distributions::Alphanumeric;
use serde::Deserialize;
use topcoat::{
    Error, Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, page, query_params, route,
    },
    runtime::{Event, signal},
    view::{View, attributes, view},
};
use tower_sessions::cookie::time;

use crate::components::button::{ButtonType, button};
use crate::components::form::{InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
use crate::layout::page_heading::page_heading;
use crate::user::auth::redirect_signed_in_users;
use crate::user::queries::{NewUser, query_insert_email_verification_token, query_insert_user};
use crate::user::service::send_register_verification_email;
use crate::utils::{get_token_hash_from_token_string, normalize_email};
use crate::web::db;

#[derive(Deserialize)]
pub struct RegisterInput {
    name: String,
    email: String,
    new_password: String,
    confirm_password: String,
}

struct RegisterFailed {
    name: String,
    email: String,
    message: &'static str,
}

#[query_params]
struct RegisterQuery {
    registered: Option<bool>,
}

#[page("/register")]
pub async fn register_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let registered = query_params::<RegisterQuery>(cx)
        .ok()
        .and_then(|query| query.registered)
        .unwrap_or(false);
    let failed = try_request_context::<RegisterFailed>(cx);
    let name = failed.map(|failed| failed.name.clone());
    let email = failed.map(|failed| failed.email.clone());
    let error = failed.map(|failed| failed.message);
    let is_submitting = signal(cx, || false);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Register")
            form(
                attrs: attributes! {
                    class="form form--skinny"
                    method="post"
                    action=(href!(register_submit))
                    @submit=$(|_e: Event| is_submitting.set(true))
                },
                form_field(
                    label: "Name",
                    id: "name",
                    name: "name",
                    placeholder: Some("Name"),
                    value: name.as_deref(),
                )
                form_field(
                    label: "Email",
                    id: "email",
                    name: "email",
                    input_type: InputType::Email,
                    placeholder: Some("Email"),
                    value: email.as_deref(),
                )
                form_field(
                    label: "New password",
                    id: "new-password",
                    name: "new_password",
                    input_type: InputType::Password,
                    placeholder: Some("Enter a password"),
                )
                form_field(
                    label: "Confirm new password",
                    id: "confirm-password",
                    name: "confirm_password",
                    input_type: InputType::Password,
                    placeholder: Some("Confirm password"),
                )
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    disabled: $(is_submitting.get()),
                    "Register account"
                )
                if registered {
                    notice(
                        kind: NoticeKind::Success,
                        "Account created! Check your email to confirm your account."
                    )
                }
                if let Some(error) = error {
                    notice(kind: NoticeKind::Error, (error))
                }
            )
        </section>
    })
}

#[route(POST "/register")]
pub async fn register_submit(cx: &Cx, Form(input): Form<RegisterInput>) -> Result<SeeOther> {
    match attempt_register(cx, &input).await? {
        Ok(()) => Ok(see_other(
            href!(register_page)
                .query(&[("registered", "true")])
                .resolve(cx),
        )),
        Err(message) => Err(rewrite(href!(register_page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(RegisterFailed {
                name: input.name,
                email: input.email,
                message,
            })
            .into()),
    }
}

async fn attempt_register(cx: &Cx, input: &RegisterInput) -> Result<Result<(), &'static str>> {
    const FAILED: &str = "Registration failed. Please try again.";

    let email = normalize_email(&input.email);
    let name = input.name.trim();

    if name.is_empty()
        || email.is_empty()
        || input.new_password.is_empty()
        || input.confirm_password.is_empty()
    {
        return Ok(Err("Missing required values."));
    }

    if input.new_password != input.confirm_password {
        return Ok(Err("Passwords do not match."));
    }

    let salt = SaltString::generate(rand::thread_rng());
    let Ok(password_hash) = Argon2::default().hash_password(input.new_password.as_bytes(), &salt)
    else {
        return Ok(Err("Failed to hash password."));
    };
    let password_hash = password_hash.to_string();

    let new_user = NewUser {
        name,
        email: &email,
        password_hash: &password_hash,
    };

    let Ok(mut tx) = db(cx).begin().await else {
        return Ok(Err(FAILED));
    };

    let user_id = match query_insert_user(&new_user, &mut *tx).await {
        Ok(user_id) => user_id,
        Err(e) => {
            tracing::error!("Registration failed: {}", e);
            return Ok(Err("Registration failed. Email may already be in use."));
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
        return Ok(Err(FAILED));
    }

    let app_base_url = env::var("APP_BASE_URL").map_err(|e| Error::from(e).context("APP_BASE_URL is not set"))?;
    let verify_url = format!("{app_base_url}/user/verify/{token}");

    if let Err(e) = send_register_verification_email(&email, &verify_url).await {
        tracing::error!("Failed to send verification email: {}", e);
        return Ok(Err("Could not send verification email. Please try again."));
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit registration: {}", e);
        return Ok(Err(FAILED));
    }

    Ok(Ok(()))
}
