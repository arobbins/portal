use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        error::{SeeOther, see_other},
        page, path_param, request, route,
    },
    runtime::signal,
    view::{View, ViewExt, attributes, component, view},
};

use crate::account::tabs::AccountTab;
use crate::action::{ActionProblem, page_status};
use crate::app::page_title;
use crate::auth::actions::{ConfirmEmailProblem, confirm_email_change, pending_email_change};
use crate::auth::links::TokenPurpose;
use crate::auth::pages::expired_link::expired_link;
use crate::auth::session::{current_user, stay_signed_in};
use crate::components::button::{ButtonType, button};
use crate::components::form::{FormVariant, form, form_actions, form_error};
use crate::flash::{Flash, set_flash};
use crate::layout::page_heading::page_heading;
use crate::web::{app, env, redisplay, session};

path_param!(token);

struct ConfirmEmailFailed {
    problem: ConfirmEmailProblem,
}

#[page(
    "/confirm-email/{token}",
    with(Metadata::new().title(page_title("Confirm your new email")))
)]
pub async fn confirm_email_page(cx: &Cx) -> Result<impl View> {
    let Some(new_email) = pending_email_change(&env(cx), token(cx)).await? else {
        return Ok(view! { spent_link() }.boxed());
    };
    let problem = try_context::<ConfirmEmailFailed>(cx).map(|failed| failed.problem);

    Ok(view! { confirm_email_form(new_email: &new_email, problem: problem) }.boxed())
}

#[component]
async fn spent_link(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        expired_link(
            <p>
                "This confirmation link is invalid or has already been used. Links work once and expire after "
                (TokenPurpose::EmailChange.lifetime_text())
                "."
            </p>
            <p>"To get a new one, change your email again from your profile."</p>
        )
    })
}

#[component]
async fn confirm_email_form(
    cx: &Cx,
    new_email: &str,
    problem: Option<ConfirmEmailProblem>,
) -> Result<impl View> {
    let is_submitting = signal(cx, || false);
    let action = request::uri(cx).path().to_string();

    Ok(view! {
        <section class="l-contain l-skinny">
            if let Some(problem) = problem {
                (page_status(problem))
            }
            page_heading(text: "Confirm your new email")
            <p class="p-auth__intro">
                "From now on you'll sign in with "
                <strong>(new_email)</strong>
                "."
            </p>
            form(
                submitting: Some(&is_submitting),
                variant: FormVariant::Page,
                attrs: attributes! { method="post" action=(&action) },
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Confirm email"
                    )
                )
            )
        </section>
    })
}

#[route(POST "/confirm-email/{token}")]
pub async fn confirm_email_submit(cx: &Cx) -> Result<SeeOther> {
    match confirm_email_change(&env(cx), token(cx)).await? {
        Ok(user) => {
            let session = session(cx)?;
            if current_user(cx).is_some_and(|signed_in| signed_in.id == user.id) {
                stay_signed_in(app(cx), &session, &user).await?;
            }
            set_flash(&session, Flash::EmailChanged).await?;
            Ok(see_other(AccountTab::Profile.path()))
        }
        Err(problem) => Err(redisplay(
            request::uri(cx).path(),
            ConfirmEmailFailed { problem },
        )),
    }
}
