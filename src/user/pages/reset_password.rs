use serde::Deserialize;
use topcoat::{
    Error, Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{SeeOther, rewrite, see_other},
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
use crate::components::form::{Autocomplete, InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
use crate::components::toaster::{NO_TOAST, toaster};
use crate::flash::{Flash, set_flash, take_flash};
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
use crate::utils::{get_token_hash_from_token_string, normalize_email};
use crate::web::{app_url, db, rate_limits, session};

const PURPOSE: TokenPurpose = TokenPurpose::PasswordReset;
const SENT_TOAST: &str = "sent";

path_param!(token);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResetRequestProblem {
    InvalidEmail,
    TooManyAttempts,
}

impl ResetRequestProblem {
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

    let sent = take_flash(cx, &[Flash::ResetLinkSent]).await?.is_some();
    let failed = try_request_context::<ResetRequestFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);
    let toast = signal(cx, || if sent { SENT_TOAST } else { NO_TOAST }.to_string());

    Ok(view! {
        <section class="l-contain l-skinny">
            if problem == Some(ResetRequestProblem::TooManyAttempts) {
                (StatusCode::TOO_MANY_REQUESTS)
            }
            page_heading(text: "Reset password")
            <p>
                "Enter your email and we'll send you a link to choose a new password."
            </p>
            toaster(
                toast: &toast,
                name: SENT_TOAST,
                kind: NoticeKind::Success,
                "If an account exists for that email, we've sent a link to reset your password."
            )
            form(
                submitting: Some(&is_submitting),
                attrs: attributes! {
                    class="form form--skinny"
                    method="post"
                    action=(href!(reset_password_submit))
                },
                form_field(
                    label: "Email",
                    id: "email",
                    autocomplete: Some(Autocomplete::Email),
                    name: "email",
                    input_type: InputType::Email,
                    placeholder: Some("Email"),
                    value: email.as_deref(),
                    required: true
                )
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    disabled: $(is_submitting.get()),
                    "Send reset link"
                )
                if let Some(problem) = problem {
                    notice(kind: NoticeKind::Error, (problem.message()))
                }
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
    let email = normalize_email(&input.email);

    let outcome = if !rate_limits(cx).allow_email_request(client_ip(cx)) {
        Err(ResetRequestProblem::TooManyAttempts)
    } else if !email.contains('@') {
        Err(ResetRequestProblem::InvalidEmail)
    } else {
        send_reset_link(cx, &email).await?;
        Ok(())
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

async fn send_reset_link(cx: &Cx, email: &str) -> Result<()> {
    if !rate_limits(cx).allow_email_to(email) {
        tracing::warn!("password reset email skipped: too many emails to this address");
        return Ok(());
    }

    let Some(user) = query_get_user_by_email(email, db(cx)).await? else {
        return Ok(());
    };

    let sent = match user.status() {
        AccountStatus::Active { .. } => {
            let tx = db(cx).begin().await?;
            email_user_link(cx, tx, user.id, PURPOSE, |token| {
                let reset_url = app_url(&format!("/reset-password/{token}"))?;
                Ok(password_reset_email(&user.email, &reset_url))
            })
            .await?
        }
        AccountStatus::AwaitingPassword => issue_verification(cx, user.id, &user.email).await?,
    };

    if let Err(e) = sent {
        tracing::error!("Failed to send password reset email: {}", e);
    }

    Ok(())
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

    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return Err(back_to_form().with(NewPasswordFailed { problem }).into());
    }

    let password_hash = hash_password(&input.password).map_err(|e| Error::msg(e.to_string()))?;
    let token_hash = get_token_hash_from_token_string(path_param::<Token>(cx));
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
