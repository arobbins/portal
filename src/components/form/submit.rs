use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Expr, Signal, Surrogated, TypedProcedure, signal},
    view::{Child, View, attributes, component, view},
};

use super::dialog::{DialogForm, dialog_form_error};
use super::layout::{form, form_actions};
use crate::action::FieldError;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::modal::modal;
use crate::components::toaster::NO_TOAST;

pub struct ModalSubmit<'a, P: TypedProcedure> {
    pub procedure: P,
    pub action: String,
    pub label: &'a str,
    pub success_toast: &'a str,
    pub unreachable: &'a str,
}

#[component]
pub async fn modal_form<P>(
    cx: &Cx,
    id: &str,
    heading: &str,
    form: &DialogForm,
    toast: &Signal<String>,
    submit: ModalSubmit<'_, P>,
    args: Expr<P::Args>,
    #[default] saved: Option<&Signal<String>>,
    #[default] variant: ButtonVariant,
    #[default] navigate_to: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View>
where
    P: TypedProcedure<Output = std::result::Result<String, FieldError>> + Surrogated + Copy,
    P::Args: Clone + Send + Sync + 'static,
    <P as Surrogated>::Surrogate: serde::Serialize,
{
    let (error_field, error_message) = (&form.error_field, &form.error_message);
    let is_submitting = &form.is_submitting;
    let ModalSubmit {
        procedure,
        action,
        label,
        success_toast,
        unreachable,
    } = submit;
    let saved = saved.cloned().unwrap_or_else(|| signal(cx, String::new));
    let success_toast = success_toast.to_string();
    let modal_id = id.to_string();
    let navigate_to = navigate_to.unwrap_or_default().to_string();
    let failed: std::result::Result<String, FieldError> = Err(FieldError::general(unreachable));

    Ok(view! {
        modal(
            id: id,
            heading: heading,
            form(
                attrs: attributes! {
                    method="post"
                    action=(&action)
                    @submit=$(async |e: Event| {
                        e.prevent_default();
                        is_submitting.set(true);
                        error_field.set("".to_owned());
                        error_message.set("".to_owned());
                        toast.set(NO_TOAST.to_owned());
                        let _values = args;
                        let outcome = raw!(
                            "await Promise.resolve(${procedure}.call(...${_values})).catch(() => ${failed})",
                            failed.clone(),
                        );
                        is_submitting.set(false);
                        if outcome.is_ok() {
                            saved.set(outcome.unwrap());
                            toast.set(success_toast.clone());
                            raw!(
                                "String(${navigate_to}) ? location.assign(String(${navigate_to})) : document.getElementById(String(${modal_id})).close()",
                                (),
                            );
                        } else {
                            let failure = outcome.unwrap_err();
                            error_field.set(failure.field);
                            error_message.set(failure.message);
                        }
                    })
                },
                (child)
                modal_form_footer(form: form, label: label, variant: variant)
            )
        )
    })
}

#[component]
async fn modal_form_footer(
    cx: &Cx,
    form: &DialogForm,
    label: &str,
    variant: ButtonVariant,
) -> Result<impl View> {
    let is_submitting = &form.is_submitting;

    Ok(view! {
        dialog_form_error(form: form)
        form_actions(
            button(
                variant: variant,
                button_type: ButtonType::Submit,
                busy: $(is_submitting.get()),
                (label)
            )
        )
    })
}
