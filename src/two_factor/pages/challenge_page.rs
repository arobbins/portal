use schemars::JsonSchema;
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

use crate::action::{ActionProblem, page_status};
use crate::app::page_title;
use crate::auth::session::sign_in;
use crate::components::button::{ButtonType, button};
use crate::components::form::{
    Autocomplete, FormVariant, form, form_actions, form_error, form_field,
};
use crate::devices::lifecycle::SignInMethod;
use crate::flash::{Flash, set_flash};
use crate::layout::page_heading::page_heading;
use crate::passkeys::pages::passkey_setup::after_password_sign_in;
use crate::routes::{ACCOUNT_PATH, LOGIN_PATH};
use crate::two_factor::actions::{CODE_FIELD, SecondFactorProblem, verify_second_factor};
use crate::two_factor::challenge::{end_second_factor, pending_user};
use crate::web::{env, redisplay, session};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SecondFactor {
    #[serde(default)]
    pub code: String,
}

struct SecondFactorFailed {
    problem: SecondFactorProblem,
}

#[page(
    "/login/two-factor",
    with(Metadata::new().title(page_title("Two-factor authentication")))
)]
pub async fn two_factor_page(cx: &Cx) -> Result<impl View> {
    if pending_user(&env(cx), &session(cx)?).await?.is_none() {
        return Err(see_other(LOGIN_PATH).into());
    }
    let problem = try_context::<SecondFactorFailed>(cx).map(|failed| failed.problem);
    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
            }
            page_heading(text: "Enter your code")
            code_form(problem: problem)
        </section>
    })
}

#[component]
async fn code_form(cx: &Cx, problem: Option<SecondFactorProblem>) -> Result<impl View> {
    let is_submitting = signal(cx, || false);

    Ok(view! {
        form(
            submitting: Some(&is_submitting),
            variant: FormVariant::Page,
            attrs: attributes! { method="post" action=(href!(two_factor_submit)) },
            <p>
                "Open your authenticator app and enter the 6-digit code for this account. No phone? Enter one of your recovery codes."
            </p>
            form_field(
                label: "Code",
                id: "two-factor-code",
                name: CODE_FIELD,
                autocomplete: Some(Autocomplete::OneTimeCode),
                required: true,
                invalid: problem.is_some(),
                attrs: attributes! { autofocus="" autocapitalize="off" spellcheck="false" }
            )
            if let Some(problem) = problem {
                form_error((problem.message()))
            }
            form_actions(
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    "Verify code"
                )
                <a href=(LOGIN_PATH)>"Cancel"</a>
            )
        )
    })
}

#[route(POST "/login/two-factor")]
pub async fn two_factor_submit(cx: &Cx, Form(input): Form<SecondFactor>) -> Result<SeeOther> {
    let env = env(cx);
    let session = session(cx)?;
    let Some((user, method)) = pending_user(&env, &session).await? else {
        return Ok(see_other(LOGIN_PATH));
    };

    match verify_second_factor(&env, &user, &input.code).await? {
        Ok(()) => {
            end_second_factor(&session).await?;
            sign_in(cx, &session, &user, method).await?;
            if method == SignInMethod::PasswordReset {
                set_flash(&session, Flash::PasswordReset).await?;
                return Ok(see_other(ACCOUNT_PATH));
            }
            Ok(see_other(
                after_password_sign_in(cx, &session, &user).await?,
            ))
        }
        Err(problem) => Err(redisplay(
            href!(two_factor_page).resolve(cx),
            SecondFactorFailed { problem },
        )),
    }
}
