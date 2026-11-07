use topcoat::{
    Result,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonType, button};
use crate::components::form::{InputType, form, form_col, form_field, form_row};

#[component]
pub async fn example_form_fields(id_prefix: &str) -> Result<impl View> {
    let first_name_id = format!("{id_prefix}-first-name");
    let email_id = format!("{id_prefix}-email");
    let password_id = format!("{id_prefix}-password");
    let confirm_password_id = format!("{id_prefix}-confirm-password");

    Ok(view! {
        form_field(
            label: "First name",
            id: &first_name_id,
            name: "first_name",
            placeholder: Some("Andrew")
        )
        form_field(
            label: "Email",
            id: &email_id,
            name: "email",
            input_type: InputType::Email,
            placeholder: Some("hello@andrewrobbins.dev")
        )
        form_row(
            form_col(
                half: true,
                form_field(
                    label: "New password",
                    id: &password_id,
                    name: "password",
                    input_type: InputType::Password,
                    placeholder: Some("Enter a password"),
                    required: true
                )
            )
            form_col(
                half: true,
                form_field(
                    label: "Confirm password",
                    id: &confirm_password_id,
                    name: "confirm_password",
                    input_type: InputType::Password,
                    placeholder: Some("Confirm your password"),
                    required: true
                )
            )
        )
    })
}

#[component]
pub async fn forms_section() -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Forms",
            form(
                example_form_fields(id_prefix: "styleguide-form")
                button(button_type: ButtonType::Submit, "Submit")
            )
        )
    })
}
