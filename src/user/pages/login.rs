use argon2::{Argon2, PasswordVerifier};
use password_hash::PasswordHash;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, page, query_params,
        request::client_ip,
        route,
    },
    runtime::{Event, signal},
    view::{View, attributes, view},
};

use super::verify_email::resend_verification;
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::form::{Autocomplete, InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
use crate::layout::page_heading::page_heading;
use crate::user::auth::{redirect_signed_in_users, sign_in};
use crate::user::queries::query_get_user_by_email;
use crate::utils::normalize_email;
use crate::web::{db, rate_limits, session};

#[derive(Deserialize)]
pub struct LoginInput {
    email: String,
    password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginProblem {
    InvalidCredentials,
    Unverified,
    TooManyAttempts,
}

impl LoginProblem {
    fn message(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "Invalid email or password.",
            Self::Unverified => "Please verify your email before logging in.",
            Self::TooManyAttempts => "Too many attempts. Please wait a few minutes and try again.",
        }
    }
}

struct LoginFailed {
    email: String,
    problem: LoginProblem,
}

#[query_params]
struct LoginQuery {
    resent: Option<bool>,
}

#[page("/login")]
pub async fn login_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let resent = query_params::<LoginQuery>(cx)
        .ok()
        .and_then(|query| query.resent)
        .unwrap_or(false);
    let failed = try_request_context::<LoginFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);

    Ok(view! {
        <section class="l-contain l-skinny">
            if problem == Some(LoginProblem::TooManyAttempts) {
                (StatusCode::TOO_MANY_REQUESTS)
            }
            page_heading(text: "Login")
            if resent {
                notice(
                    kind: NoticeKind::Success,
                    "If that account still needs confirming, we've sent a new link. Check your email."
                )
            }
            form(
                attrs: attributes! {
                    class="form form--skinny"
                    method="post"
                    action=(href!(login_submit))
                    @submit=$(|_e: Event| is_submitting.set(true))
                },
                form_field(
                    label: "Email",
                    id: "email",
                    autocomplete: Some(Autocomplete::Email),
                    name: "email",
                    input_type: InputType::Email,
                    placeholder: Some("Enter email"),
                    value: email.as_deref()
                )
                form_field(
                    label: "Password",
                    id: "password",
                    autocomplete: Some(Autocomplete::CurrentPassword),
                    name: "password",
                    input_type: InputType::Password,
                    placeholder: Some("Enter password")
                )
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    disabled: $(is_submitting.get()),
                    "Login"
                )
                if let Some(problem) = problem {
                    notice(kind: NoticeKind::Error, (problem.message()))
                }
            )
            if problem == Some(LoginProblem::Unverified) {
                form(
                    attrs: attributes! {
                        class="form form--skinny"
                        method="post"
                        action=(href!(resend_verification))
                    },
                    <input type="hidden" name="email" value=(email.as_deref())>
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        button_type: ButtonType::Submit,
                        "Resend confirmation email"
                    )
                )
            }
        </section>
    })
}

#[route(POST "/login")]
pub async fn login_submit(cx: &Cx, Form(input): Form<LoginInput>) -> Result<SeeOther> {
    let limits = rate_limits(cx);
    let ip = client_ip(cx);

    let outcome = if !limits.allow_login(ip) || limits.login_blocked(&input.email, ip) {
        Err(LoginProblem::TooManyAttempts)
    } else {
        attempt_login(cx, &input).await?
    };

    match outcome {
        Ok(()) => limits.clear_failed_logins(&input.email, ip),
        Err(LoginProblem::InvalidCredentials) => limits.record_failed_login(&input.email, ip),
        Err(_) => {}
    }

    match outcome {
        Ok(()) => Ok(see_other("/account")),
        Err(problem) => Err(rewrite(href!(login_page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(LoginFailed {
                email: input.email,
                problem,
            })
            .into()),
    }
}

async fn attempt_login(cx: &Cx, input: &LoginInput) -> Result<Result<(), LoginProblem>> {
    let email = normalize_email(&input.email);

    let Some(user) = query_get_user_by_email(&email, db(cx)).await? else {
        return Ok(Err(LoginProblem::InvalidCredentials));
    };

    let Some(stored_hash) = user.password.as_deref() else {
        return Ok(Err(LoginProblem::InvalidCredentials));
    };

    let Ok(parsed_hash) = PasswordHash::new(stored_hash) else {
        return Ok(Err(LoginProblem::InvalidCredentials));
    };

    if Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Ok(Err(LoginProblem::InvalidCredentials));
    }

    if user.verified_at.is_none() {
        return Ok(Err(LoginProblem::Unverified));
    }

    sign_in(&session(cx)?, &user).await?;

    Ok(Ok(()))
}
