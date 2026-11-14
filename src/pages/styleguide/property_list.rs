use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::label::{LabelSize, label};
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;

#[component]
pub async fn property_list_section(cx: &Cx) -> Result<impl View> {
    let field_tags = vec![
        PropertyTag::new("string", Tone::Neutral),
        PropertyTag::new("required", Tone::Error),
        PropertyTag::new("1–100 characters", Tone::Neutral),
    ];
    let device_tags = vec![PropertyTag::new("This device only", Tone::Warning)];

    Ok(view! {
        styleguide_section(
            title: "Property lists",
            label(size: LabelSize::Sm, "Fields")
            property_list(
                property(
                    name: "name",
                    code: true,
                    tags: &field_tags,
                    description: Some(
                        "The display name. Leading and trailing spaces are removed.",
                    )
                )
                property(
                    name: "email",
                    code: true,
                    description: Some("The email address the account signs in with."),
                    detail: Some("Read-only.")
                )
            )
            label(size: LabelSize::Sm, "With actions")
            property_list(
                property(
                    name: "MacBook Pro",
                    tags: &device_tags,
                    detail: Some("Added Nov 13, 2026 · never used"),
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        "Rename"
                    )
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        "Remove"
                    )
                )
            )
        )
    })
}
