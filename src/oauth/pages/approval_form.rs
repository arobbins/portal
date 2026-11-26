use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::signal,
    view::{View, attributes, component, view},
};

use super::authorize_page::{DECISION_FIELD, DENY, SCOPE_FIELD, authorize_submit};
use crate::action::{ActionProblem, ID_FIELD};
use crate::api::scope::ApiScope;
use crate::auth::pages::reauth_field::reauth_field;
use crate::components::badge::badge;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{
    Choice, FormVariant, checkbox_group, form, form_actions, form_error,
};
use crate::components::tone::Tone;
use crate::layout::page_heading::page_heading;
use crate::oauth::actions::ConnectProblem;

const DENY_FORM: &str = "deny-app";

pub struct Approval {
    pub request: String,
    pub name: String,
    pub return_host: String,
    pub scopes: Vec<ApiScope>,
}

#[component]
pub async fn approval_form(
    cx: &Cx,
    approval: &Approval,
    problem: Option<ConnectProblem>,
) -> Result<impl View> {
    let action = href!(authorize_submit).resolve(cx);
    let is_submitting = signal(cx, || false);
    let options: Vec<Choice<'_>> = approval
        .scopes
        .iter()
        .map(|scope| Choice {
            value: scope.as_str(),
            label: scope.choice_label(),
        })
        .collect();
    let checked: Vec<&str> = approval.scopes.iter().map(|scope| scope.as_str()).collect();
    let writes = approval.scopes.iter().any(|scope| scope.can_write());

    Ok(view! {
        <section class="l-contain l-skinny p-authorize">
            app_identity(approval: approval)
            form(
                submitting: Some(&is_submitting),
                variant: FormVariant::Page,
                attrs: attributes! { method="post" action=(action.as_str()) },
                <input type="hidden" name=(ID_FIELD) value=(approval.request.as_str())>
                if options.is_empty() {
                    <p>"It isn't asking to read or change anything in your account."</p>
                } else {
                    checkbox_group(
                        legend: "Allow it to",
                        id: "scopes",
                        name: SCOPE_FIELD,
                        options: &options,
                        checked: &checked
                    )
                }
                if writes {
                    reauth_field(id: "current-password", suggest_passkey: false)
                }
                if let Some(problem) = problem {
                    form_error((problem.message()))
                }
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Allow access"
                    )
                    button(
                        variant: ButtonVariant::Secondary,
                        button_type: ButtonType::Submit,
                        attrs: attributes! { form=(DENY_FORM) },
                        "Deny"
                    )
                )
            )
            <form id=(DENY_FORM) method="post" action=(action.as_str()) hidden="">
                <input type="hidden" name=(ID_FIELD) value=(approval.request.as_str())>
                <input type="hidden" name=(DECISION_FIELD) value=(DENY)>
            </form>
        </section>
    })
}

#[component]
async fn app_identity(cx: &Cx, approval: &Approval) -> Result<impl View> {
    let heading = format!("Connect {}", approval.name);

    Ok(view! {
        page_heading(text: &heading)
        <p class="p-authorize__app">
            <strong>(approval.name.as_str())</strong>
            badge(tone: Tone::Warning, "Unverified name")
        </p>
        <p class="p-authorize__return">
            "Returns to "
            <strong>(approval.return_host.as_str())</strong>
        </p>
        <p>
            "Only allow apps you trust. It can use your account within what you allow until you disconnect it on the Developer tab."
        </p>
    })
}
