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

use super::switch_link::switch_link;
use crate::action::{ActionProblem, page_status};
use crate::app::page_title;
use crate::auth::actions::{EMAIL_FIELD, SignIn, SignInProblem, sign_in_with_password};
use crate::auth::handoff::{handoff_param, remember_handoff, with_handoff};
use crate::auth::pages::register::register_page;
use crate::auth::pages::reset_password::reset_password_page;
use crate::auth::password::PASSWORD_FIELD;
use crate::auth::session::{redirect_signed_in_users, sign_in};
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, LabelLink, form, form_actions, form_error, form_field,
};
use crate::devices::lifecycle::SignInMethod;
use crate::flash::{Flash, flash_toast, flash_toasts, take_flash};
use crate::layout::page_heading::page_heading;
use crate::passkeys::pages::passkey_setup::after_password_sign_in;
use crate::two_factor::actions::needs_second_factor;
use crate::two_factor::challenge::{begin_second_factor, end_second_factor};
use crate::two_factor::pages::challenge_page::two_factor_page;
use crate::web::{env, redisplay, session};

const PASSKEY_ERROR_ID: &str = "passkey-error";
const FLASHES: &[Flash] = &[Flash::AccountDeleted];

struct LoginFailed {
    email: String,
    problem: SignInProblem,
}

#[page("/login", with(Metadata::new().title(page_title("Sign in"))))]
pub async fn login_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;
    let session = session(cx)?;
    end_second_factor(&session).await?;
    let handoff = handoff_param(cx);
    let register_href = with_handoff(&href!(register_page).resolve(cx), handoff.as_ref());
    remember_handoff(&env(cx), &session, handoff).await?;

    let flash = take_flash(cx, FLASHES).await?;
    let toast = flash_toast(cx, flash);
    let failed = try_context::<LoginFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let problem = failed.map(|failed| failed.problem);
    let is_submitting = signal(cx, || false);
    let invalid_credentials = problem == Some(SignInProblem::InvalidCredentials);
    let reset_href = href!(reset_password_page).resolve(cx);

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
            }
            page_heading(text: "Sign in")
            flash_toasts(toast: &toast, flashes: FLASHES)
            form(
                submitting: Some(&is_submitting),
                variant: FormVariant::Page,
                attrs: attributes! { method="post" action=(href!(login_submit)) },
                form_field(
                    label: "Email",
                    id: "email",
                    autocomplete: Some(Autocomplete::UsernameWebauthn),
                    name: EMAIL_FIELD,
                    input_type: InputType::Email,
                    placeholder: Some("Enter email"),
                    value: email.as_deref(),
                    invalid: invalid_credentials
                )
                form_field(
                    label: "Password",
                    id: "password",
                    autocomplete: Some(Autocomplete::CurrentPassword),
                    name: PASSWORD_FIELD,
                    input_type: InputType::Password,
                    placeholder: Some("Enter password"),
                    invalid: invalid_credentials,
                    label_link: Some(
                        LabelLink { href: &reset_href, text: "Forgot password?" },
                    )
                )
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Sign in"
                    )
                )
            )
            passkey_sign_in()
            switch_link(
                prompt: "Don't have an account yet? ",
                href: &register_href,
                label: "Register"
            )
        </section>
    })
}

#[component]
async fn passkey_sign_in(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <div class="p-auth__passkey">
            <p class="p-auth__passkey-divider">"or"</p>
            button(
                variant: ButtonVariant::Secondary,
                attrs: attributes! { data-passkey-login="" },
                "Sign in with a passkey"
            )
            form_error(id: Some(PASSKEY_ERROR_ID), hidden: true, "")
        </div>
    })
}

#[route(POST "/login")]
pub async fn login_submit(cx: &Cx, Form(input): Form<SignIn>) -> Result<SeeOther> {
    match sign_in_with_password(&env(cx), &input).await? {
        Ok(user) => {
            let session = session(cx)?;
            let env = env(cx);
            if needs_second_factor(&env, &user).await? {
                begin_second_factor(&session, &user, SignInMethod::Password, env.now()).await?;
                return Ok(see_other(href!(two_factor_page).resolve(cx)));
            }
            sign_in(cx, &session, &user, SignInMethod::Password).await?;
            Ok(see_other(
                after_password_sign_in(cx, &session, &user).await?,
            ))
        }
        Err(problem) => Err(redisplay(
            href!(login_page).resolve(cx),
            LoginFailed {
                email: input.email,
                problem,
            },
        )),
    }
}
