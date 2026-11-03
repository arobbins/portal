use topcoat::{
    Result,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::icon::{IconName, icon};

#[component]
pub async fn buttons_section() -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Buttons",
            button("Primary button")
            button(busy: true, "Busy button")
            button(disabled: true, "Disabled button")
            button(size: ButtonSize::Sm, "Small button")
            button(size: ButtonSize::Md, "Medium button")
            button(size: ButtonSize::Lg, "Large button")
            button(variant: ButtonVariant::Secondary, size: ButtonSize::Lg, "Secondary button")
            button(variant: ButtonVariant::Secondary, size: ButtonSize::Md, "Secondary button")
            button(variant: ButtonVariant::Secondary, size: ButtonSize::Sm, "Secondary button")
            button(
                size: ButtonSize::Sm,
                icon(name: IconName::Edit)
                "Button with icon"
            )
        )
    })
}
