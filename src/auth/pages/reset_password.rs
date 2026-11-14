use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other, too_many_requests},
        href, page, path_param, request, route,
    },
    runtime::signal,
    view::{View, ViewExt, attributes, view},
};

use crate::account::pages::account::account_page;
use crate::action::{ActionProblem, page_status};
use crate::app::page_title;
use crate::auth::actions::{
    EMAIL_FIELD, LinkPasswordProblem, NewPassword, RequestPasswordReset, ResetRequestProblem,
    link_is_valid, request_password_reset, set_password_from_link,
};
use crate::auth::links::TokenPurpose;
use crate::auth::pages::expired_link::expired_link;
use crate::auth::pages::login::login_page;
use crate::auth::pages::new_password_form::new_password_form;
use crate::auth::password::PasswordProblem;
use crate::auth::session::{redirect_signed_in_users, sign_in};
use crate::components::button::{ButtonType, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, form, form_actions, form_error, form_field,
};
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::page_heading;
use crate::web::session;

pub const RETRY_AFTER_SECS: u64 = 10;
const PURPOSE: TokenPurpose = TokenPurpose::PasswordReset;
const FLASHES: &[Flash] = &[Flash::ResetLinkSent];

path_param!(token);

struct ResetRequestFailed {
    email: String,
    problem: ResetRequestProblem,
}

pub struct LinkPasswordFailed {
    pub problem: PasswordProblem,
}

#[page("/reset-password", with(Metadata::new().title(page_title("Reset password"))))]
pub async fn reset_password_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<ResetRequestFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);
    let toast = signal(cx, || toast_name(flash).to_string());

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
            }
            page_heading(text: "Reset password")
            <p class="p-auth__intro">
                "Enter your email to receive a link for choosing a fancy new password."
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
                    name: EMAIL_FIELD,
                    input_type: InputType::Email,
                    placeholder: Some("Enter email"),
                    value: email.as_deref(),
                    required: true,
                    invalid: problem == Some(ResetRequestProblem::InvalidEmail)
                )
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Send reset link"
                    )
                )
            )
            <p class="p-auth__switch">
                "Do you remember? "
                <a class="p-auth__switch-link" href=(href!(login_page).resolve(cx))>
                    "Sign in instead"
                </a>
            </p>
        </section>
    })
}

#[route(POST "/reset-password")]
pub async fn reset_password_submit(
    cx: &Cx,
    Form(input): Form<RequestPasswordReset>,
) -> Result<SeeOther> {
    match request_password_reset(cx, &input).await? {
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

#[page(
    "/reset-password/{token}",
    with(Metadata::new().title(page_title("Choose a new password")))
)]
pub async fn choose_password_page(cx: &Cx) -> Result<impl View> {
    if !link_is_valid(cx, PURPOSE, token(cx)).await? {
        return Ok(view! {
            expired_link(
                <p>
                    "This reset link is invalid or has already been used. Links work once and expire after "
                    (PURPOSE.lifetime_text())
                    "."
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

    let problem = try_context::<LinkPasswordFailed>(cx).map(|failed| failed.problem);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Choose a new password")
            <p class="p-auth__intro">
                "Choosing a new password signs you out on every other device."
            </p>
            new_password_form(submit_label: "Reset password", problem: problem)
        </section>
    }
    .boxed())
}

#[route(POST "/reset-password/{token}")]
pub async fn choose_password_submit(cx: &Cx, Form(input): Form<NewPassword>) -> Result<SeeOther> {
    match set_password_from_link(cx, PURPOSE, token(cx), &input).await? {
        Ok(user) => {
            let session = session(cx)?;
            sign_in(&session, &user).await?;
            set_flash(&session, Flash::PasswordReset).await?;
            Ok(see_other(href!(account_page).resolve(cx)))
        }
        Err(problem) => Err(link_password_failed(cx, problem)),
    }
}

pub fn link_password_failed(cx: &Cx, problem: LinkPasswordProblem) -> topcoat::Error {
    let back_to_form = rewrite(request::uri(cx).path(), Body::empty()).method(Method::GET);
    match problem {
        LinkPasswordProblem::TooManyAttempts => too_many_requests(RETRY_AFTER_SECS).into(),
        LinkPasswordProblem::ExpiredLink => back_to_form.into(),
        LinkPasswordProblem::Password(problem) => {
            back_to_form.with(LinkPasswordFailed { problem }).into()
        }
    }
}
