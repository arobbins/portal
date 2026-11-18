use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use crate::components::copy_field::copy_field;
use crate::components::form::{InputType, LabelLink, code_field, form, form_field};

#[component]
pub(super) async fn special_fields_demo(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h3 class="p-styleguide__subheading">"Disabled"</h3>
        form(
            form_field(
                label: "Email",
                id: "styleguide-disabled-email",
                name: "email",
                value: Some("hello@andrewrobbins.dev"),
                attrs: attributes! { disabled=(true) }
            )
        )
        <h3 class="p-styleguide__subheading">"Read-only"</h3>
        form(
            form_field(
                label: "Current email",
                id: "styleguide-readonly-email",
                name: "current_email",
                input_type: InputType::Email,
                value: Some("hello@andrewrobbins.dev"),
                attrs: attributes! { readonly="" }
            )
        )
        <h3 class="p-styleguide__subheading">"Label link"</h3>
        form(
            form_field(
                label: "Password",
                id: "styleguide-label-link-password",
                name: "password",
                input_type: InputType::Password,
                placeholder: Some("Enter password"),
                label_link: Some(
                    LabelLink {
                        href: "#styleguide-label-link-password",
                        text: "Forgot password?",
                    },
                )
            )
        )
        <h3 class="p-styleguide__subheading">"Hidden label"</h3>
        form(
            form_field(
                label: "Search people",
                hide_label: true,
                placeholder: Some("Search people"),
                id: "styleguide-search",
                name: "q"
            )
        )
        <h3 class="p-styleguide__subheading">"One-time code"</h3>
        form(
            code_field(
                label: "Authentication code",
                id: "styleguide-code",
                name: "code"
            )
        )
        <h3 class="p-styleguide__subheading">"Copy field"</h3>
        form(
            copy_field(
                label: "API token",
                id: "styleguide-token",
                value: "wsk_live_4f9a2c7e1b8d3f60"
            )
        )
    })
}
