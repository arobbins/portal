use crate::routes::ACCOUNT_PATH;
use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        content::Form,
        error::{SeeOther, see_other, too_many_requests},
        href, page, path_param, request, route,
    },
    runtime::signal,
    view::{View, ViewExt, attributes, component, view},
};

use super::link_sent::{link_sent_notice, remember_link_sent_to, take_link_sent_to};
use super::switch_link::switch_link;
use crate::action::{ActionProblem, page_status};
use crate::app::page_title;
use crate::auth::actions::{
    EMAIL_FIELD, LinkPasswordProblem, NewPassword, RequestPasswordReset, ResetRequestProblem,
    link_is_valid, link_needs_code, request_password_reset, set_password_from_link,
};
use crate::auth::links::TokenPurpose;
use crate::auth::pages::expired_link::expired_link;
use crate::auth::pages::login::login_page;
use crate::auth::pages::new_password_form::new_password_form;
use crate::auth::session::{redirect_signed_in_users, sign_in};
use crate::components::button::{ButtonType, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, form, form_actions, form_error, form_field,
};
use crate::devices::lifecycle::SignInMethod;
use crate::flash::{Flash, set_flash, take_flash};
use crate::layout::page_heading::page_heading;
use crate::web::{env, redisplay, session};

pub const RETRY_AFTER_SECS: u64 = 10;
const PURPOSE: TokenPurpose = TokenPurpose::PasswordReset;
const FLASHES: &[Flash] = &[Flash::ResetLinkSent];

path_param!(token);

struct ResetRequestFailed {
    email: String,
    problem: ResetRequestProblem,
}

pub struct LinkPasswordFailed {
    pub problem: LinkPasswordProblem,
}

struct CodeUsedUpLink;

#[page("/reset-password", with(Metadata::new().title(page_title("Reset password"))))]
pub async fn reset_password_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<ResetRequestFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let sent_to = match flash {
        Some(_) => take_link_sent_to(cx).await?,
        None => None,
    };
    let sent_after = format!(
        ", we sent it a link to choose a new password. It expires in {}.",
        TokenPurpose::PasswordReset.lifetime_text()
    );
    let reset_href = href!(reset_password_page).resolve(cx);
    let login_href = href!(login_page).resolve(cx);

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
            }
            page_heading(text: "Reset password")
            if let Some(email) = &sent_to {
                link_sent_notice(
                    before: "If an account exists for ",
                    email: email.as_str(),
                    after: &sent_after,
                    again_href: &reset_href
                )
            } else {
                <p class="p-auth__intro">
                    "Enter your email to receive a link for choosing a fancy new password."
                </p>
                reset_form(email: email.as_deref(), problem: problem)
                switch_link(
                    prompt: "Do you remember? ",
                    href: &login_href,
                    label: "Sign in instead"
                )
            }
        </section>
    })
}

#[route(POST "/reset-password")]
pub async fn reset_password_submit(
    cx: &Cx,
    Form(input): Form<RequestPasswordReset>,
) -> Result<SeeOther> {
    match request_password_reset(&env(cx), &input).await? {
        Ok(()) => {
            let session = session(cx)?;
            set_flash(&session, Flash::ResetLinkSent).await?;
            remember_link_sent_to(&session, &input.email).await?;
            Ok(see_other(href!(reset_password_page).resolve(cx)))
        }
        Err(problem) => {
            let failed = ResetRequestFailed {
                email: input.email,
                problem,
            };
            Err(redisplay(href!(reset_password_page).resolve(cx), failed))
        }
    }
}

#[page(
    "/reset-password/{token}",
    with(Metadata::new().title(page_title("Choose a new password")))
)]
pub async fn choose_password_page(cx: &Cx) -> Result<impl View> {
    if try_context::<CodeUsedUpLink>(cx).is_some() {
        let message = LinkPasswordProblem::WrongCode.message();
        return Ok(view! { used_up_link(message: &message) }.boxed());
    }
    let env = env(cx);
    if !link_is_valid(&env, PURPOSE, token(cx)).await? {
        let expired = format!(
            "This reset link is invalid or has already been used. Links work once and expire after {}.",
            PURPOSE.lifetime_text()
        );
        return Ok(view! { used_up_link(message: &expired) }.boxed());
    }

    let problem = try_context::<LinkPasswordFailed>(cx).map(|failed| failed.problem);
    let asks_code = link_needs_code(&env, PURPOSE, token(cx)).await?;

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
            }
            page_heading(text: "Choose a new password")
            <p class="p-auth__intro">
                "Choosing a new password signs you out on every other device."
            </p>
            new_password_form(
                submit_label: "Reset password",
                problem: problem,
                asks_code: asks_code
            )
        </section>
    }
    .boxed())
}

#[component]
async fn used_up_link(cx: &Cx, message: &str) -> Result<impl View> {
    Ok(view! {
        expired_link(
            <p>(message)</p>
            <p>
                <a href=(href!(reset_password_page).resolve(cx))>
                    "Request a new link"
                </a>
            </p>
        )
    })
}

#[route(POST "/reset-password/{token}")]
pub async fn choose_password_submit(cx: &Cx, Form(input): Form<NewPassword>) -> Result<SeeOther> {
    match set_password_from_link(&env(cx), PURPOSE, token(cx), &input).await? {
        Ok(user) => {
            let session = session(cx)?;
            sign_in(cx, &session, &user, SignInMethod::PasswordReset).await?;
            set_flash(&session, Flash::PasswordReset).await?;
            Ok(see_other(ACCOUNT_PATH))
        }
        Err(problem) => Err(link_password_failed(cx, problem)),
    }
}

pub fn link_password_failed(cx: &Cx, problem: LinkPasswordProblem) -> topcoat::Error {
    let form_path = request::uri(cx).path();
    match problem {
        LinkPasswordProblem::TooManyAttempts => too_many_requests(RETRY_AFTER_SECS).into(),
        LinkPasswordProblem::ExpiredLink => redisplay(form_path, ()),
        LinkPasswordProblem::WrongCode => redisplay(form_path, CodeUsedUpLink),
        LinkPasswordProblem::Password(_) | LinkPasswordProblem::TooManyCodeAttempts => {
            redisplay(form_path, LinkPasswordFailed { problem })
        }
    }
}

#[component]
async fn reset_form(
    cx: &Cx,
    email: Option<&str>,
    problem: Option<ResetRequestProblem>,
) -> Result<impl View> {
    let is_submitting = signal(cx, || false);

    Ok(view! {
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
                value: email,
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
    })
}
