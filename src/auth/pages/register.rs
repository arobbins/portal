use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, page, route,
    },
    runtime::signal,
    view::{View, attributes, view},
};

use crate::action::{ActionProblem, page_status};
use crate::app::page_title;
use crate::auth::actions::{EMAIL_FIELD, Register, RegisterProblem, register};
use crate::auth::session::redirect_signed_in_users;
use crate::components::button::{ButtonType, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, form, form_actions, form_error, form_field,
};
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::page_heading;
use crate::user::name::NAME_FIELD;
use crate::web::session;

const FLASHES: &[Flash] = &[Flash::RegistrationSent];

#[derive(Deserialize)]
pub struct RegisterForm {
    name: String,
    email: String,
    #[serde(default)]
    website: String,
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
    let is_submitting = signal(cx, || false);
    let toast = signal(cx, || toast_name(flash).to_string());

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
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
                    name: NAME_FIELD,
                    placeholder: Some("Name"),
                    value: name.as_deref(),
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
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Register account"
                    )
                )
            )
        </section>
    })
}

#[route(POST "/register")]
pub async fn register_submit(cx: &Cx, Form(form): Form<RegisterForm>) -> Result<SeeOther> {
    let sent = see_other(href!(register_page).resolve(cx));
    if !form.website.is_empty() {
        tracing::info!("registration ignored: honeypot field was filled");
        set_flash(&session(cx)?, Flash::RegistrationSent).await?;
        return Ok(sent);
    }

    let input = Register {
        name: form.name,
        email: form.email,
    };
    match register(cx, &input).await? {
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
