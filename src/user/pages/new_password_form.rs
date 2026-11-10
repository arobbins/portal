use topcoat::{
    Result,
    context::Cx,
    runtime::signal,
    view::{View, attributes, component, view},
};

use crate::components::button::{ButtonType, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, form, form_actions, form_error, form_field,
};
use crate::user::password::PasswordProblem;

#[component]
pub async fn new_password_form(
    cx: &Cx,
    submit_label: &str,
    #[default] problem: Option<PasswordProblem>,
) -> Result<impl View> {
    let is_submitting = signal(cx, || false);

    Ok(view! {
        form(
            submitting: Some(&is_submitting),
            variant: FormVariant::Page,
            attrs: attributes! { method="post" },
            form_field(
                label: "New password",
                id: "new-password",
                autocomplete: Some(Autocomplete::NewPassword),
                name: "password",
                input_type: InputType::Password,
                placeholder: Some("At least 8 characters"),
                required: true,
                invalid: problem.is_some_and(PasswordProblem::invalid_password)
            )
            form_field(
                label: "Confirm new password",
                id: "confirm-password",
                autocomplete: Some(Autocomplete::NewPassword),
                name: "confirm_password",
                input_type: InputType::Password,
                placeholder: Some("Confirm password"),
                required: true,
                invalid: problem.is_some_and(PasswordProblem::invalid_confirmation)
            )
            if let Some(problem) = problem {
                form_error((problem.message()))
            }
            form_actions(
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    (submit_label)
                )
            )
        )
    })
}
