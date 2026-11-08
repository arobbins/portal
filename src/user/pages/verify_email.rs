use serde::Deserialize;
use sqlx::PgExecutor;
use topcoat::{
    Error, Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        page, path_param,
        request::{self, client_ip},
        route,
    },
    runtime::{Event, signal},
    view::{View, ViewExt, attributes, view},
};

use crate::components::button::{ButtonType, button};
use crate::components::form::{Autocomplete, InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
use crate::layout::page_heading::page_heading;
use crate::user::auth::{hash_password, sign_in};
use crate::user::queries::{
    query_consume_user_token, query_delete_user_tokens, query_find_valid_user_token,
    query_get_user_by_email, query_insert_user_token, query_set_password_and_verify_user,
};
use crate::user::service::register_verification_email;
use crate::user::tokens::{TokenPurpose, generate_token};
use crate::utils::{get_token_hash_from_token_string, normalize_email};
use crate::web::{app_url, db, mailer, rate_limits, session};

const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_PASSWORD_LENGTH: usize = 256;

path_param!(token);

pub async fn create_verification_token(
    user_id: i32,
    db: impl PgExecutor<'_>,
) -> std::result::Result<String, sqlx::Error> {
    let token = generate_token();
    query_insert_user_token(user_id, TokenPurpose::EmailVerification, &token.hash, db).await?;
    Ok(token.raw)
}

pub async fn send_verification_email(
    cx: &Cx,
    email: &str,
    token: &str,
) -> Result<std::result::Result<(), String>> {
    let verify_url = app_url(&format!("/user/verify/{token}"))?;

    Ok(mailer(cx)
        .send(&register_verification_email(email, &verify_url))
        .await)
}

pub async fn issue_verification(
    cx: &Cx,
    user_id: i32,
    email: &str,
) -> Result<std::result::Result<(), String>> {
    let mut tx = db(cx).begin().await?;
    query_delete_user_tokens(user_id, TokenPurpose::EmailVerification, &mut *tx).await?;
    let token = create_verification_token(user_id, &mut *tx).await?;

    let sent = send_verification_email(cx, email, &token).await?;
    if sent.is_ok() {
        tx.commit().await?;
    }

    Ok(sent)
}

struct SetPasswordFailed {
    message: &'static str,
}

#[page("/user/verify/{token}")]
pub async fn verify_email_page(cx: &Cx) -> Result<impl View> {
    let token_hash = get_token_hash_from_token_string(path_param::<Token>(cx));
    let is_valid =
        query_find_valid_user_token(TokenPurpose::EmailVerification, &token_hash, db(cx))
            .await?
            .is_some();

    if !is_valid {
        return Ok(view! {
            (StatusCode::BAD_REQUEST)
            <section class="l-contain l-skinny">
                page_heading(text: "This link has expired")
                <p>
                    "This confirmation link is invalid or has already been used. Links work once and expire after 24 hours."
                </p>
                <p>
                    "To get a new one, "
                    <a href="/register">"register again"</a>
                    " with the same email."
                </p>
            </section>
        }
        .boxed());
    }

    let error = try_request_context::<SetPasswordFailed>(cx).map(|failed| failed.message);
    let is_submitting = signal(cx, || false);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Set your password")
            <p>
                "Your email is almost confirmed. Choose a password to finish setting up your account."
            </p>
            form(
                attrs: attributes! {
                    class="form form--skinny"
                    method="post"
                    @submit=$(|_e: Event| is_submitting.set(true))
                },
                form_field(
                    label: "New password",
                    id: "new-password",
                    autocomplete: Some(Autocomplete::NewPassword),
                    name: "password",
                    input_type: InputType::Password,
                    placeholder: Some("At least 8 characters"),
                    required: true
                )
                form_field(
                    label: "Confirm new password",
                    id: "confirm-password",
                    autocomplete: Some(Autocomplete::NewPassword),
                    name: "confirm_password",
                    input_type: InputType::Password,
                    placeholder: Some("Confirm password"),
                    required: true
                )
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    disabled: $(is_submitting.get()),
                    "Set password"
                )
                if let Some(error) = error {
                    notice(kind: NoticeKind::Error, (error))
                }
            )
        </section>
    }
    .boxed())
}

#[derive(Deserialize)]
pub struct SetPasswordInput {
    password: String,
    confirm_password: String,
}

#[route(POST "/user/verify/{token}")]
pub async fn set_password_submit(cx: &Cx, Form(input): Form<SetPasswordInput>) -> Result<SeeOther> {
    let back_to_form = |message: &'static str| {
        rewrite(request::uri(cx).path(), Body::empty())
            .method(Method::GET)
            .with(SetPasswordFailed { message })
    };

    if input.password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err(back_to_form("Password must be at least 8 characters.").into());
    }

    if input.password.chars().count() > MAX_PASSWORD_LENGTH {
        return Err(back_to_form("Password must be 256 characters or fewer.").into());
    }

    if input.password != input.confirm_password {
        return Err(back_to_form("Passwords do not match.").into());
    }

    let password_hash = hash_password(&input.password).map_err(|e| Error::msg(e.to_string()))?;
    let token_hash = get_token_hash_from_token_string(path_param::<Token>(cx));
    let mut tx = db(cx).begin().await?;

    let Some(user_id) =
        query_consume_user_token(TokenPurpose::EmailVerification, &token_hash, &mut *tx).await?
    else {
        return Err(rewrite(request::uri(cx).path(), Body::empty())
            .method(Method::GET)
            .into());
    };

    query_delete_user_tokens(user_id, TokenPurpose::EmailVerification, &mut *tx).await?;
    let user = query_set_password_and_verify_user(user_id, &password_hash, &mut *tx).await?;
    tx.commit().await?;

    sign_in(&session(cx)?, &user).await?;

    Ok(see_other("/account?verified=true"))
}

#[derive(Deserialize)]
pub struct ResendInput {
    email: String,
}

#[route(POST "/user/verify/resend")]
pub async fn resend_verification(cx: &Cx, Form(input): Form<ResendInput>) -> Result<SeeOther> {
    let email = normalize_email(&input.email);

    let allowed =
        rate_limits(cx).allow_signup(client_ip(cx)) && rate_limits(cx).allow_email_to(&email);

    if allowed
        && let Some(user) = query_get_user_by_email(&email, db(cx)).await?
        && user.verified_at.is_none()
        && let Err(e) = issue_verification(cx, user.id, &user.email).await?
    {
        tracing::error!("Failed to resend verification email: {}", e);
    }

    Ok(see_other("/login?resent=true"))
}
