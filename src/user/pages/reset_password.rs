use serde::Deserialize;
use sqlx::PgPool;
use topcoat::{
    Error, Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{SeeOther, rewrite, see_other, too_many_requests},
        href, page, path_param,
        request::{self, client_ip},
        route,
    },
    runtime::signal,
    view::{View, ViewExt, attributes, view},
};

use super::expired_link::expired_link;
use super::new_password_form::new_password_form;
use super::verify_email::issue_verification;
use crate::components::button::{ButtonType, button};
use crate::components::form::{Autocomplete, FormVariant, InputType, form, form_error, form_field};
use crate::config::Config;
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::page_heading;
use crate::user::auth::{hash_password, redirect_signed_in_users, sign_in};
use crate::user::links::email_user_link;
use crate::user::model::AccountStatus;
use crate::user::password::{PasswordProblem, check_new_password};
use crate::user::queries::{
    query_consume_user_token, query_delete_user_tokens, query_find_valid_user_token,
    query_get_user_by_email, query_reset_password,
};
use crate::user::service::password_reset_email;
use crate::user::tokens::TokenPurpose;
use crate::utils::{get_token_hash_from_token_string, parse_email};
use crate::web::{config, db, rate_limits, session};

const RETRY_AFTER_SECS: u64 = 10;
const PURPOSE: TokenPurpose = TokenPurpose::PasswordReset;
const FLASHES: &[Flash] = &[Flash::ResetLinkSent];

path_param!(token);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResetRequestProblem {
    InvalidEmail,
    TooManyAttempts,
}

impl ResetRequestProblem {
    fn invalid_email(self) -> bool {
        self == Self::InvalidEmail
    }

    fn status(self) -> StatusCode {
        match self {
            Self::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::OK,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::InvalidEmail => "Please enter a valid email address.",
            Self::TooManyAttempts => "Too many attempts. Please wait a few minutes and try again.",
        }
    }
}

struct ResetRequestFailed {
    email: String,
    problem: ResetRequestProblem,
}

#[page("/reset-password")]
pub async fn reset_password_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_request_context::<ResetRequestFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);
    let toast = signal(cx, || toast_name(flash).to_string());

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (problem.status())
            }
            page_heading(text: "Reset password")
            <p>
                "Enter your email and we'll send you a link to choose a new password."
            </p>
            flash_toasts(toast: &toast, flashes: FLASHES)
            form(
                submitting: Some(&is_submitting),
                variant: FormVariant::Page,
                attrs: attributes! { method="post" action=(href!(reset_password_submit)) },
                form_field(
                    label: "Email",
                    id: "email",
                    autocomplete: Some(Autocomplete::Email),
                    name: "email",
                    input_type: InputType::Email,
                    placeholder: Some("Email"),
                    value: email.as_deref(),
                    required: true,
                    invalid: problem.is_some_and(ResetRequestProblem::invalid_email)
                )
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    "Send reset link"
                )
            )
            <p><a href="/login">"Back to login"</a></p>
        </section>
    })
}

#[derive(Deserialize)]
pub struct ResetRequestInput {
    email: String,
}

#[route(POST "/reset-password")]
pub async fn reset_password_submit(
    cx: &Cx,
    Form(input): Form<ResetRequestInput>,
) -> Result<SeeOther> {
    let email = parse_email(&input.email);

    let outcome = match email {
        _ if !rate_limits(cx).allow_email_request(client_ip(cx)) => {
            Err(ResetRequestProblem::TooManyAttempts)
        }
        None => Err(ResetRequestProblem::InvalidEmail),
        Some(email) if !rate_limits(cx).allow_email_to(&email) => {
            tracing::warn!("password reset email skipped: too many emails to this address");
            Ok(())
        }
        Some(email) => {
            let db = db(cx).clone();
            let config = config(cx).clone();
            tokio::spawn(async move {
                if let Err(e) = send_reset_link(&db, &config, &email).await {
                    tracing::error!("Failed to send password reset email: {}", e);
                }
            });
            Ok(())
        }
    };

    match outcome {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::ResetLinkSent).await?;
            Ok(see_other(href!(reset_password_page).resolve(cx)))
        }
        Err(problem) => Err(
            rewrite(href!(reset_password_page).resolve(cx), Body::empty())
                .method(Method::GET)
                .with(ResetRequestFailed {
                    email: input.email,
                    problem,
                })
                .into(),
        ),
    }
}

async fn send_reset_link(db: &PgPool, config: &Config, email: &str) -> Result<(), String> {
    let Some(user) = query_get_user_by_email(email, db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };

    let sent = match user.status() {
        AccountStatus::Active { .. } => {
            email_user_link(db, &config.mailer, user.id, PURPOSE, |token| {
                password_reset_email(
                    &user.email,
                    &config.url(&format!("/reset-password/{token}")),
                )
            })
            .await
        }
        AccountStatus::AwaitingPassword => {
            issue_verification(db, config, user.id, &user.email).await
        }
    };

    sent.map_err(|e| e.to_string())?
}

struct NewPasswordFailed {
    problem: PasswordProblem,
}

#[page("/reset-password/{token}")]
pub async fn choose_password_page(cx: &Cx) -> Result<impl View> {
    let token_hash = get_token_hash_from_token_string(path_param::<Token>(cx));
    let is_valid = query_find_valid_user_token(PURPOSE, &token_hash, db(cx))
        .await?
        .is_some();

    if !is_valid {
        return Ok(view! {
            expired_link(
                <p>
                    "This reset link is invalid or has already been used. Links work once and expire after 1 hour."
                </p>
                <p>
                    <a href=(href!(reset_password_page).resolve(cx))>
                        "Request a new link"
                    </a>
                </p>
            )
        }
        .boxed());
    }

    let problem = try_request_context::<NewPasswordFailed>(cx).map(|failed| failed.problem);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Choose a new password")
            <p>"Choosing a new password signs you out on every other device."</p>
            new_password_form(submit_label: "Reset password", problem: problem)
        </section>
    }
    .boxed())
}

#[derive(Deserialize)]
pub struct NewPasswordInput {
    password: String,
    confirm_password: String,
}

#[route(POST "/reset-password/{token}")]
pub async fn choose_password_submit(
    cx: &Cx,
    Form(input): Form<NewPasswordInput>,
) -> Result<SeeOther> {
    let back_to_form = || rewrite(request::uri(cx).path(), Body::empty()).method(Method::GET);

    if !rate_limits(cx).allow_password_submission(client_ip(cx)) {
        return Err(too_many_requests(RETRY_AFTER_SECS).into());
    }

    let token_hash = get_token_hash_from_token_string(path_param::<Token>(cx));
    if query_find_valid_user_token(PURPOSE, &token_hash, db(cx))
        .await?
        .is_none()
    {
        return Err(back_to_form().into());
    }

    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return Err(back_to_form().with(NewPasswordFailed { problem }).into());
    }

    let password_hash = hash_password(&input.password).map_err(|e| Error::msg(e.to_string()))?;
    let mut tx = db(cx).begin().await?;

    let Some(user_id) = query_consume_user_token(PURPOSE, &token_hash, &mut *tx).await? else {
        return Err(back_to_form().into());
    };

    query_delete_user_tokens(user_id, PURPOSE, &mut *tx).await?;
    let Some(user) = query_reset_password(user_id, &password_hash, &mut *tx).await? else {
        return Err(back_to_form().into());
    };
    tx.commit().await?;

    let session = session(cx)?;
    sign_in(&session, &user).await?;
    set_flash(&session, Flash::PasswordReset).await?;

    Ok(see_other("/account"))
}
