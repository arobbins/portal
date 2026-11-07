use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::icon::{IconName, icon};

#[component]
pub async fn buttons_section(cx: &Cx) -> Result<impl View> {
    let is_busy = signal(cx, || false);

    Ok(view! {
        styleguide_section(
            title: "Buttons",
            button(
                attrs: attributes! { @click=$(|_e: Event| is_busy.set(true)) },
                busy: $(is_busy.get()),
                "Primary button"
            )
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
