use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_request_context},
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

use crate::components::button::{ButtonType, button};
use crate::components::form::{Autocomplete, InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
use crate::components::toaster::{NO_TOAST, toaster};
use crate::flash::{Flash, set_flash, take_flash};
use crate::layout::page_heading::page_heading;
use crate::user::auth::redirect_signed_in_users;
use crate::user::pages::verify_email::{issue_verification, issue_verification_in};
use crate::user::queries::{NewUser, query_get_user_by_email, query_insert_user};
use crate::user::service::account_exists_email;
use crate::utils::normalize_email;
use crate::web::{app_url, db, mailer, rate_limits, session};

const MAX_NAME_LENGTH: usize = 100;
const SENT_TOAST: &str = "sent";

#[derive(Deserialize)]
pub struct RegisterInput {
    name: String,
    email: String,
    #[serde(default)]
    website: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegisterProblem {
    MissingDetails,
    NameTooLong,
    InvalidEmail,
    SendFailed,
    TooManyAttempts,
}

impl RegisterProblem {
    fn message(self) -> &'static str {
        match self {
            Self::MissingDetails => "Please enter your name and email.",
            Self::NameTooLong => "Name must be 100 characters or fewer.",
            Self::InvalidEmail => "Please enter a valid email address.",
            Self::SendFailed => "We couldn't send the email. Please try again.",
            Self::TooManyAttempts => "Too many attempts. Please wait a few minutes and try again.",
        }
    }
}

struct RegisterFailed {
    name: String,
    email: String,
    problem: RegisterProblem,
}

#[page("/register")]
pub async fn register_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let sent = take_flash(cx, &[Flash::RegistrationSent]).await?.is_some();
    let failed = try_request_context::<RegisterFailed>(cx);
    let name = failed.map(|failed| failed.name.clone());
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);
    let toast = signal(cx, || if sent { SENT_TOAST } else { NO_TOAST }.to_string());

    Ok(view! {
        <section class="l-contain l-skinny">
            if problem == Some(RegisterProblem::TooManyAttempts) {
                (StatusCode::TOO_MANY_REQUESTS)
            }
            page_heading(text: "Register")
            toaster(
                toast: &toast,
                name: SENT_TOAST,
                kind: NoticeKind::Success,
                "Check your email to confirm your registration."
            )
            form(
                submitting: Some(&is_submitting),
                attrs: attributes! {
                    class="form form--skinny"
                    method="post"
                    action=(href!(register_submit))
                },
                form_field(
                    label: "Name",
                    id: "name",
                    autocomplete: Some(Autocomplete::Name),
                    name: "name",
                    placeholder: Some("Name"),
                    value: name.as_deref(),
                    required: true
                )
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
                <div class="a-form__trap" aria-hidden="true">
                    <label for="website">"Website"</label>
                    <input
                        type="text"
                        id="website"
                        name="website"
                        tabindex="-1"
                        autocomplete="off"
                    >
                </div>
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    disabled: $(is_submitting.get()),
                    "Register account"
                )
                if let Some(problem) = problem {
                    notice(kind: NoticeKind::Error, (problem.message()))
                }
            )
        </section>
    })
}

#[route(POST "/register")]
pub async fn register_submit(cx: &Cx, Form(input): Form<RegisterInput>) -> Result<SeeOther> {
    let sent = see_other(href!(register_page).resolve(cx));

    if !input.website.is_empty() {
        tracing::info!("registration ignored: honeypot field was filled");
        set_flash(&session(cx)?, Flash::RegistrationSent).await?;
        return Ok(sent);
    }

    let outcome = if rate_limits(cx).allow_email_request(client_ip(cx)) {
        attempt_register(cx, &input).await?
    } else {
        Err(RegisterProblem::TooManyAttempts)
    };

    match outcome {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::RegistrationSent).await?;
            Ok(sent)
        }
        Err(problem) => Err(rewrite(href!(register_page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(RegisterFailed {
                name: input.name,
                email: input.email,
                problem,
            })
            .into()),
    }
}

async fn attempt_register(cx: &Cx, input: &RegisterInput) -> Result<Result<(), RegisterProblem>> {
    let name = input.name.trim();
    let email = normalize_email(&input.email);

    if name.is_empty() || email.is_empty() {
        return Ok(Err(RegisterProblem::MissingDetails));
    }

    if name.chars().count() > MAX_NAME_LENGTH {
        return Ok(Err(RegisterProblem::NameTooLong));
    }

    if !email.contains('@') {
        return Ok(Err(RegisterProblem::InvalidEmail));
    }

    if !rate_limits(cx).allow_email_to(&email) {
        tracing::warn!("registration email skipped: too many emails to this address");
        return Ok(Ok(()));
    }

    let sent = match query_get_user_by_email(&email, db(cx)).await? {
        Some(user) if user.status().is_active() => {
            let login_url = app_url("/login")?;
            mailer(cx)
                .send(&account_exists_email(&user.email, &login_url))
                .await
        }
        Some(user) => issue_verification(cx, user.id, &user.email).await?,
        None => {
            let mut tx = db(cx).begin().await?;
            let new_user = NewUser {
                name,
                email: &email,
            };
            match query_insert_user(&new_user, &mut *tx).await? {
                Some(user_id) => issue_verification_in(cx, tx, user_id, &email).await?,
                None => Ok(()),
            }
        }
    };

    if let Err(e) = sent {
        tracing::error!("Failed to send registration email: {}", e);
        return Ok(Err(RegisterProblem::SendFailed));
    }

    Ok(Ok(()))
}
