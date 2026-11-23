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
    view::{View, attributes, view},
};

use super::admin_access::{
    ADMIN_IDLE_LIMIT, admin_caller, admin_destination, take_return_to, unlock_admin,
};
use crate::action::ActionProblem;
use crate::admin::actions::{ConfirmAdmin, confirm_admin};
use crate::app::page_title;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::reauth::{ReauthProblem, return_passkey_proof, take_passkey_proof};
use crate::components::button::{ButtonType, button};
use crate::components::form::{FormVariant, form, form_actions, form_error};
use crate::layout::page_heading::page_heading;
use crate::web::{env, redisplay, session};

struct AdminConfirmFailed {
    problem: ReauthProblem,
}

#[page("/admin/confirm", with(Metadata::new().title(page_title("Confirm it's you"))))]
pub async fn admin_confirm_page(cx: &Cx) -> Result<impl View> {
    admin_caller(cx)?;
    let problem = try_context::<AdminConfirmFailed>(cx).map(|failed| failed.problem);
    let action = href!(admin_confirm_submit).resolve(cx);
    let is_submitting = signal(cx, || false);
    let intro = format!(
        "Confirm it's you to open the admin area. It locks again after {} minutes without activity.",
        ADMIN_IDLE_LIMIT.whole_minutes()
    );

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Admin area")
            <p>(intro)</p>
            form(
                variant: FormVariant::Page,
                submitting: Some(&is_submitting),
                attrs: attributes! { method="post" action=(action) },
                reauth_field(id: "admin-confirm-password", suggest_passkey: false)
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Continue"
                    )
                )
            )
        </section>
    })
}

#[route(POST "/admin/confirm")]
pub async fn admin_confirm_submit(cx: &Cx, Form(input): Form<ConfirmAdmin>) -> Result<SeeOther> {
    let caller = admin_caller(cx)?;
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;

    match confirm_admin(&env, &caller, &input, passkey).await? {
        Ok(()) => {
            unlock_admin(&session, &caller.user, env.now()).await?;
            let next = take_return_to(&session).await?;
            Ok(see_other(admin_destination(cx, next)))
        }
        Err(error) => {
            return_passkey_proof(&session, passkey).await?;
            let problem = error.for_browser()?;
            let path = href!(admin_confirm_page).resolve(cx);
            Err(redisplay(path, AdminConfirmFailed { problem }))
        }
    }
}
