use topcoat::{
    Result,
    context::Cx,
    runtime::signal,
    view::{View, attributes, component, view},
};

use crate::components::button::{ButtonType, button};
use crate::components::form::{Autocomplete, InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
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
            attrs: attributes! { class="form form--skinny" method="post" },
            form_field(
                label: "New password",
                id: "new-password",
                autocomplete: Some(Autocomplete::NewPassword),
                name: "password",
                input_type: InputType::Password,
                placeholder: Some("At least 8 characters"),
                required: true
            )
            form_field(
                label: "Confirm new password",
                id: "confirm-password",
                autocomplete: Some(Autocomplete::NewPassword),
                name: "confirm_password",
                input_type: InputType::Password,
                placeholder: Some("Confirm password"),
                required: true
            )
            button(
                button_type: ButtonType::Submit,
                busy: $(is_submitting.get()),
                disabled: $(is_submitting.get()),
                (submit_label)
            )
            if let Some(problem) = problem {
                notice(kind: NoticeKind::Error, (problem.message()))
            }
        )
    })
}
