use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use super::forms::example_form_fields;
use super::styleguide_section;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::form::{form, form_actions};
use crate::components::modal::modal;

const CONFIRM_ID: &str = "styleguide-confirm";

#[component]
pub async fn modal_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Modals",
            <div class="styleguide__row">
                button(
                    attrs: attributes! { command="show-modal" commandfor="styleguide-modal" },
                    "Open modal"
                )
                button(
                    variant: ButtonVariant::Danger,
                    attrs: attributes! { command="show-modal" commandfor=(CONFIRM_ID) },
                    "Delete account"
                )
            </div>
            modal(
                id: "styleguide-modal",
                heading: "Update profile",
                form(
                    example_form_fields(id_prefix: "styleguide-modal-form")
                    form_actions(button(button_type: ButtonType::Submit, "Submit"))
                )
            )
            confirm_dialog(
                id: CONFIRM_ID,
                heading: "Delete your account?",
                confirm_label: "Delete account",
                <p>
                    "This permanently deletes your account, passkeys and data. You can't undo this."
                </p>
            )
        )
    })
}
