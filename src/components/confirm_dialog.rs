use topcoat::{
    Result,
    context::Cx,
    runtime::Signal,
    view::{Child, View, attributes, component, view},
};

use super::button::{ButtonType, ButtonVariant, button};
use super::form::{form, form_actions, form_error};
use super::modal::modal;

#[component]
pub async fn confirm_dialog(
    cx: &Cx,
    id: &str,
    heading: &str,
    confirm_label: &str,
    #[default] action: Option<&str>,
    #[default] submitting: Option<&Signal<bool>>,
    #[default] error: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let method = if action.is_some() { "post" } else { "dialog" };

    Ok(view! {
        modal(
            id: id,
            heading: heading,
            form(
                submitting: submitting,
                attrs: attributes! { method=(method) action=(action) },
                <div class="a-confirm-dialog__body">(child)</div>
                if let Some(error) = error {
                    form_error((error))
                }
                form_actions(
                    button(
                        variant: ButtonVariant::Secondary,
                        attrs: attributes! { command="close" commandfor=(id) },
                        "Cancel"
                    )
                    if let Some(submitting) = submitting {
                        button(
                            variant: ButtonVariant::Danger,
                            button_type: ButtonType::Submit,
                            busy: $(submitting.get()),
                            (confirm_label)
                        )
                    } else {
                        button(
                            variant: ButtonVariant::Danger,
                            button_type: ButtonType::Submit,
                            (confirm_label)
                        )
                    }
                )
            )
        )
    })
}
