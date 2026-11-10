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
use crate::components::form::{Autocomplete, FormVariant, InputType, form, form_error, form_field};
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::page_heading;
use crate::user::auth::redirect_signed_in_users;
use crate::user::model::AccountStatus;
use crate::user::name::{NameProblem, check_name};
use crate::user::pages::verify_email::issue_verification;
use crate::user::queries::{
    NewUser, query_delete_abandoned_signup, query_get_user_by_email, query_insert_user,
};
use crate::user::service::account_exists_email;
use crate::utils::parse_email;
use crate::web::{app_url, config, db, mailer, rate_limits, session};

const FLASHES: &[Flash] = &[Flash::RegistrationSent];

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
    Name(NameProblem),
    InvalidEmail,
    SendFailed,
    TooManyAttempts,
}

impl RegisterProblem {
    fn invalid_name(self) -> bool {
        matches!(self, Self::MissingDetails | Self::Name(_))
    }

    fn invalid_email(self) -> bool {
        matches!(self, Self::MissingDetails | Self::InvalidEmail)
    }

    fn status(self) -> StatusCode {
        match self {
            Self::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::OK,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::MissingDetails => "Please enter your name and email.",
            Self::Name(problem) => problem.message(),
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

    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_request_context::<RegisterFailed>(cx);
    let name = failed.map(|failed| failed.name.clone());
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);
    let toast = signal(cx, || toast_name(flash).to_string());

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (problem.status())
            }
            page_heading(text: "Register")
            flash_toasts(toast: &toast, flashes: FLASHES)
            form(
                submitting: Some(&is_submitting),
                variant: FormVariant::Page,
                attrs: attributes! { method="post" action=(href!(register_submit)) },
                form_field(
                    label: "Name",
                    id: "name",
                    autocomplete: Some(Autocomplete::Name),
                    name: "name",
                    placeholder: Some("Name"),
                    value: name.as_deref(),
                    required: true,
                    invalid: problem.is_some_and(RegisterProblem::invalid_name)
                )
                form_field(
                    label: "Email",
                    id: "email",
                    autocomplete: Some(Autocomplete::Email),
                    name: "email",
                    input_type: InputType::Email,
                    placeholder: Some("Email"),
                    value: email.as_deref(),
                    required: true,
                    invalid: problem.is_some_and(RegisterProblem::invalid_email)
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
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    "Register account"
                )
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
    if name.is_empty() || input.email.trim().is_empty() {
        return Ok(Err(RegisterProblem::MissingDetails));
    }

    let name = match check_name(name) {
        Ok(name) => name,
        Err(problem) => return Ok(Err(RegisterProblem::Name(problem))),
    };

    let Some(email) = parse_email(&input.email) else {
        return Ok(Err(RegisterProblem::InvalidEmail));
    };

    if !rate_limits(cx).allow_email_to(&email) {
        tracing::warn!("registration email skipped: too many emails to this address");
        return Ok(Ok(()));
    }

    let existing = query_get_user_by_email(&email, db(cx)).await?;
    let sent = match existing.as_ref().map(|user| (user, user.status())) {
        Some((user, AccountStatus::Active { .. })) => {
            let login_url = app_url(cx, "/login");
            mailer(cx)
                .send(&account_exists_email(&user.email, &login_url))
                .await
        }
        Some((user, AccountStatus::AwaitingPassword)) => {
            issue_verification(db(cx), config(cx), user.id, &user.email).await?
        }
        None => {
            let new_user = NewUser {
                name,
                email: &email,
            };
            match query_insert_user(&new_user, db(cx)).await? {
                Some(user_id) => {
                    let sent = issue_verification(db(cx), config(cx), user_id, &email).await?;
                    if sent.is_err() {
                        query_delete_abandoned_signup(user_id, db(cx)).await?;
                    }
                    sent
                }
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
