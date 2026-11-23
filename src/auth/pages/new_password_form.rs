use topcoat::{
    Result,
    context::Cx,
    runtime::signal,
    view::{Attributes, View, attributes, component, view},
};

use crate::action::ActionProblem;
use crate::auth::password::{
    CONFIRMATION_FIELD, MIN_PASSWORD_LENGTH, PASSWORD_FIELD, PasswordProblem,
};
use crate::components::button::{ButtonType, button};
use crate::components::form::{
    Autocomplete, FormVariant, InputType, form, form_actions, form_error, form_field,
};

#[component]
pub async fn new_password_fields(
    cx: &Cx,
    #[default] problem: Option<PasswordProblem>,
    #[default] password_attrs: Attributes,
    #[default] confirmation_attrs: Attributes,
) -> Result<impl View> {
    let length_hint = format!("At least {MIN_PASSWORD_LENGTH} characters");

    Ok(view! {
        form_field(
            label: "New password",
            id: "new-password",
            autocomplete: Some(Autocomplete::NewPassword),
            name: PASSWORD_FIELD,
            input_type: InputType::Password,
            hint: Some(&length_hint),
            reveal: true,
            required: true,
            invalid: problem.is_some_and(PasswordProblem::invalid_password),
            attrs: password_attrs
        )
        form_field(
            label: "Confirm new password",
            id: "confirm-password",
            autocomplete: Some(Autocomplete::NewPassword),
            name: CONFIRMATION_FIELD,
            input_type: InputType::Password,
            reveal: true,
            required: true,
            invalid: problem.is_some_and(PasswordProblem::invalid_confirmation),
            attrs: confirmation_attrs
        )
    })
}

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
            new_password_fields(problem: problem)
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
