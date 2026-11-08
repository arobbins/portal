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

use crate::components::button::{ButtonType, button};
use crate::components::form::{Autocomplete, InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
use crate::components::toaster::toaster;
use crate::layout::page_heading::page_heading;
use crate::user::auth::redirect_signed_in_users;
use crate::user::pages::verify_email::issue_verification;
use crate::user::queries::{NewUser, query_get_user_by_email, query_insert_user};
use crate::user::service::account_exists_email;
use crate::utils::normalize_email;
use crate::web::{app_url, db, mailer, rate_limits};

const MAX_NAME_LENGTH: usize = 100;
const SEND_FAILED: &str = "We couldn't send the email. Please try again.";

#[derive(Deserialize)]
pub struct RegisterInput {
    name: String,
    email: String,
    #[serde(default)]
    website: String,
}

const TOO_MANY_ATTEMPTS: &str = "Too many attempts. Please wait a few minutes and try again.";

struct RegisterFailed {
    name: String,
    email: String,
    message: &'static str,
}

#[query_params]
struct RegisterQuery {
    sent: Option<bool>,
}

#[page("/register")]
pub async fn register_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let sent = query_params::<RegisterQuery>(cx)
        .ok()
        .and_then(|query| query.sent)
        .unwrap_or(false);
    let failed = try_request_context::<RegisterFailed>(cx);
    let name = failed.map(|failed| failed.name.clone());
    let email = failed.map(|failed| failed.email.clone());
    let error = failed.map(|failed| failed.message);
    let is_submitting = signal(cx, || false);
    let is_sent_open = signal(cx, || sent);

    Ok(view! {
        <section class="l-contain l-skinny">
            if error == Some(TOO_MANY_ATTEMPTS) {
                (StatusCode::TOO_MANY_REQUESTS)
            }
            page_heading(text: "Register")
            toaster(
                open: &is_sent_open,
                kind: NoticeKind::Success,
                "Check your email to confirm your registration."
            )
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
                if let Some(error) = error {
                    notice(kind: NoticeKind::Error, (error))
                }
            )
        </section>
    })
}

#[route(POST "/register")]
pub async fn register_submit(cx: &Cx, Form(input): Form<RegisterInput>) -> Result<SeeOther> {
    let sent = see_other(href!(register_page).query(&[("sent", "true")]).resolve(cx));

    if !input.website.is_empty() {
        tracing::info!("registration ignored: honeypot field was filled");
        return Ok(sent);
    }

    let outcome = if rate_limits(cx).allow_signup(client_ip(cx)) {
        attempt_register(cx, &input).await?
    } else {
        Err(TOO_MANY_ATTEMPTS)
    };

    match outcome {
        Ok(()) => Ok(sent),
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
    let name = input.name.trim();
    let email = normalize_email(&input.email);

    if name.is_empty() || email.is_empty() {
        return Ok(Err("Please enter your name and email."));
    }

    if name.chars().count() > MAX_NAME_LENGTH {
        return Ok(Err("Name must be 100 characters or fewer."));
    }

    if !email.contains('@') {
        return Ok(Err("Please enter a valid email address."));
    }

    if !rate_limits(cx).allow_email_to(&email) {
        tracing::warn!("registration email skipped: too many emails to this address");
        return Ok(Ok(()));
    }

    let sent = match query_get_user_by_email(&email, db(cx)).await? {
        Some(user) if user.verified_at.is_some() => {
            let login_url = app_url("/login")?;
            mailer(cx)
                .send(&account_exists_email(&user.email, &login_url))
                .await
        }
        Some(user) => issue_verification(cx, user.id, &user.email).await?,
        None => {
            let new_user = NewUser {
                name,
                email: &email,
                password_hash: None,
            };
            let user_id = query_insert_user(&new_user, db(cx)).await?;
            issue_verification(cx, user_id, &email).await?
        }
    };

    if let Err(e) = sent {
        tracing::error!("Failed to send registration email: {}", e);
        return Ok(Err(SEND_FAILED));
    }

    Ok(Ok(()))
}
