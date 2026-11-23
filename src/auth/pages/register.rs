use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, page, route,
    },
    runtime::signal,
    view::{View, attributes, component, view},
};

use super::link_sent::{link_sent_notice, remember_link_sent_to, take_link_sent_to};
use super::login::login_page;
use super::switch_link::switch_link;
use crate::action::{ActionProblem, page_status};
use crate::app::page_title;
use crate::auth::actions::{EMAIL_FIELD, Register, RegisterProblem, register};
use crate::auth::links::TokenPurpose;
use crate::auth::session::redirect_signed_in_users;
use crate::components::button::{ButtonType, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, form, form_actions, form_error, form_field,
};
use crate::flash::{Flash, set_flash, take_flash};
use crate::layout::page_heading::page_heading;
use crate::user::name::NAME_FIELD;
use crate::web::{env, redisplay, session};

const FLASHES: &[Flash] = &[Flash::RegistrationSent];

#[derive(Deserialize)]
pub struct RegisterForm {
    name: String,
    email: String,
    #[serde(default)]
    website: String,
    #[serde(default)]
    time_zone: String,
}

struct RegisterFailed {
    name: String,
    email: String,
    problem: RegisterProblem,
}

#[page("/register", with(Metadata::new().title(page_title("Register"))))]
pub async fn register_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<RegisterFailed>(cx);
    let name = failed.map(|failed| failed.name.clone());
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let sent_to = match flash {
        Some(_) => take_link_sent_to(cx).await?,
        None => None,
    };
    let sent_after = format!(
        " to confirm it and set your password. It expires in {}.",
        TokenPurpose::EmailVerification.lifetime_text()
    );
    let register_href = href!(register_page).resolve(cx);
    let login_href = href!(login_page).resolve(cx);

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
            }
            page_heading(text: "Register")
            if let Some(email) = &sent_to {
                link_sent_notice(
                    before: "We sent a link to ",
                    email: email.as_str(),
                    after: &sent_after,
                    again_href: &register_href
                )
            } else {
                register_form(
                    name: name.as_deref(),
                    email: email.as_deref(),
                    problem: problem
                )
                switch_link(
                    prompt: "Already have an account? ",
                    href: &login_href,
                    label: "Sign in"
                )
            }
        </section>
    })
}

#[route(POST "/register")]
pub async fn register_submit(cx: &Cx, Form(form): Form<RegisterForm>) -> Result<SeeOther> {
    let sent = see_other(href!(register_page).resolve(cx));
    if !form.website.is_empty() {
        tracing::info!("registration ignored: honeypot field was filled");
        let session = session(cx)?;
        set_flash(&session, Flash::RegistrationSent).await?;
        remember_link_sent_to(&session, &form.email).await?;
        return Ok(sent);
    }

    let input = Register {
        name: form.name,
        email: form.email,
        time_zone: Some(form.time_zone).filter(|zone| !zone.is_empty()),
    };
    match register(&env(cx), &input).await? {
        Ok(()) => {
            let session = session(cx)?;
            set_flash(&session, Flash::RegistrationSent).await?;
            remember_link_sent_to(&session, &input.email).await?;
            Ok(sent)
        }
        Err(problem) => Err(redisplay(
            href!(register_page).resolve(cx),
            RegisterFailed {
                name: input.name,
                email: input.email,
                problem,
            },
        )),
    }
}

#[component]
async fn honeypot_field(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="u-visually-hidden" aria-hidden="true">
            <label for="website">"Website"</label>
            <input
                type="text"
                id="website"
                name="website"
                tabindex="-1"
                autocomplete="off"
            >
        </div>
    })
}

#[component]
async fn register_form(
    cx: &Cx,
    name: Option<&str>,
    email: Option<&str>,
    problem: Option<RegisterProblem>,
) -> Result<impl View> {
    let is_submitting = signal(cx, || false);

    Ok(view! {
        form(
            submitting: Some(&is_submitting),
            variant: FormVariant::Page,
            attrs: attributes! {
                method="post"
                action=(href!(register_submit))
                novalidate=""
            },
            form_field(
                label: "Name",
                id: "name",
                autocomplete: Some(Autocomplete::Name),
                name: NAME_FIELD,
                placeholder: Some("Name"),
                value: name,
                required: true,
                invalid: problem.is_some_and(RegisterProblem::invalid_name)
            )
            form_field(
                label: "Email",
                id: "email",
                autocomplete: Some(Autocomplete::Email),
                name: EMAIL_FIELD,
                input_type: InputType::Email,
                placeholder: Some("Email"),
                value: email,
                required: true,
                invalid: problem.is_some_and(RegisterProblem::invalid_email)
            )
            honeypot_field()
            <input type="hidden" name="time_zone" data-detect-time-zone="">
            <p class="p-auth__newsletters">
                "We'll send you our newsletter and product updates. You can turn them off any time in Settings."
            </p>
            if let Some(problem) = problem {
                form_error((problem.message()))
            }
            form_actions(
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    "Register account"
                )
            )
        )
    })
}
