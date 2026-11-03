use topcoat::{
    Result,
    view::{View, attributes, component, view},
};

use super::forms::example_form_fields;
use super::styleguide_section;
use crate::components::button::{ButtonType, button};
use crate::components::form::form;
use crate::components::modal::modal;

#[component]
pub async fn modal_section() -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Modal",
            button(
                attrs: attributes! { command="show-modal" commandfor="styleguide-modal" },
                "Open modal"
            )
            modal(
                id: "styleguide-modal",
                heading: "Update profile",
                form(
                    example_form_fields(id_prefix: "styleguide-modal-form")
                    button(button_type: ButtonType::Submit, "Submit")
                )
            )
        )
    })
}
