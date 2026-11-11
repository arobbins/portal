use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        Body, Method, StatusCode,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, page,
        request::client_ip,
        route,
    },
    runtime::signal,
    view::{View, attributes, view},
};

use super::reset_password::reset_password_page;
use crate::app::page_title;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, form, form_actions, form_error, form_field,
};
use crate::layout::page_heading::page_heading;
use crate::user::auth::{password_matches, redirect_signed_in_users, sign_in};
use crate::user::model::AccountStatus;
use crate::user::queries::query_get_user_by_email;
use crate::utils::parse_email;
use crate::web::{db, rate_limits, session};

const PASSKEY_ERROR_ID: &str = "passkey-error";

#[derive(Deserialize)]
pub struct LoginInput {
    email: String,
    password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginProblem {
    InvalidCredentials,
    TooManyAttempts,
}

impl LoginProblem {
    fn invalid_credentials(self) -> bool {
        self == Self::InvalidCredentials
    }

    fn status(self) -> StatusCode {
        match self {
            Self::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::OK,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "Invalid email or password.",
            Self::TooManyAttempts => "Too many attempts. Please wait a few minutes and try again.",
        }
    }
}

struct LoginFailed {
    email: String,
    problem: LoginProblem,
}

#[page("/login", with(Metadata::new().title(page_title("Login"))))]
pub async fn login_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let failed = try_context::<LoginFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);
    let invalid_credentials = problem.is_some_and(LoginProblem::invalid_credentials);

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (problem.status())
            }
            page_heading(text: "Login")
            form(
                submitting: Some(&is_submitting),
                variant: FormVariant::Page,
                attrs: attributes! { method="post" action=(href!(login_submit)) },
                form_field(
                    label: "Email",
                    id: "email",
                    autocomplete: Some(Autocomplete::UsernameWebauthn),
                    name: "email",
                    input_type: InputType::Email,
                    placeholder: Some("Enter email"),
                    value: email.as_deref(),
                    invalid: invalid_credentials
                )
                form_field(
                    label: "Password",
                    id: "password",
                    autocomplete: Some(Autocomplete::CurrentPassword),
                    name: "password",
                    input_type: InputType::Password,
                    placeholder: Some("Enter password"),
                    invalid: invalid_credentials
                )
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Login"
                    )
                    <a href=(href!(reset_password_page).resolve(cx))>
                        "Forgot password?"
                    </a>
                )
            )
            <div class="passkey-sign-in">
                <p class="passkey-sign-in__divider">"or"</p>
                button(
                    variant: ButtonVariant::Secondary,
                    attrs: attributes! { data-passkey-login="" },
                    "Sign in with a passkey"
                )
                form_error(id: Some(PASSKEY_ERROR_ID), hidden: true, "")
            </div>
        </section>
    })
}

#[route(POST "/login")]
pub async fn login_submit(cx: &Cx, Form(input): Form<LoginInput>) -> Result<SeeOther> {
    let limits = rate_limits(cx);
    let ip = client_ip(cx);

    let email = parse_email(&input.email);

    let outcome = match &email {
        _ if !limits.allow_password_submission(ip) => Err(LoginProblem::TooManyAttempts),
        Some(email) if limits.login_blocked(email, ip) => Err(LoginProblem::TooManyAttempts),
        Some(email) => attempt_login(cx, email, &input.password).await?,
        None => Err(LoginProblem::InvalidCredentials),
    };

    if let Some(email) = &email {
        match outcome {
            Ok(()) => limits.clear_failed_logins(email, ip),
            Err(LoginProblem::InvalidCredentials) => limits.record_failed_login(email, ip),
            Err(LoginProblem::TooManyAttempts) => {}
        }
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

async fn attempt_login(cx: &Cx, email: &str, password: &str) -> Result<Result<(), LoginProblem>> {
    let user = query_get_user_by_email(email, db(cx)).await?;
    let active = user.as_ref().and_then(|user| match user.status() {
        AccountStatus::Active { password_hash } => Some((user, password_hash)),
        AccountStatus::AwaitingPassword => None,
    });

    let matches = password_matches(active.map(|(_, hash)| hash), password);
    let Some((user, _)) = active.filter(|_| matches) else {
        return Ok(Err(LoginProblem::InvalidCredentials));
    };

    sign_in(&session(cx)?, user).await?;

    Ok(Ok(()))
}
