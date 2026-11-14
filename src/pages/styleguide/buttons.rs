use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::icon::{IconName, icon};
use crate::components::label::{LabelSize, label};

const VARIANTS: [(ButtonVariant, &str); 3] = [
    (ButtonVariant::Primary, "Primary"),
    (ButtonVariant::Secondary, "Secondary"),
    (ButtonVariant::Danger, "Danger"),
];

#[component]
pub async fn buttons_section(cx: &Cx) -> Result<impl View> {
    let is_busy = signal(cx, || false);

    Ok(view! {
        styleguide_section(
            title: "Buttons",
            #[key(name)]
            for (variant, name) in VARIANTS {
                label(size: LabelSize::Sm, (name))
                <div class="p-styleguide__row">
                    button(variant: variant, (name))
                    button(variant: variant, busy: true, "Busy")
                    button(variant: variant, disabled: true, "Disabled")
                    button(
                        variant: variant,
                        icon(name: IconName::Edit)
                        "With icon"
                    )
                </div>
            }
            label(size: LabelSize::Sm, "Sizes")
            <div class="p-styleguide__row">
                button(size: ButtonSize::Sm, "Small")
                button(size: ButtonSize::Md, "Medium")
                button(size: ButtonSize::Lg, "Large")
                button(variant: ButtonVariant::Secondary, size: ButtonSize::Sm, "Small")
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Md,
                    "Medium"
                )
                button(variant: ButtonVariant::Secondary, size: ButtonSize::Lg, "Large")
            </div>
            label(size: LabelSize::Sm, "As a link")
            <div class="p-styleguide__row">
                button(href: Some("#buttons"), "Jump to buttons")
                button(
                    variant: ButtonVariant::Secondary,
                    href: Some("#forms"),
                    "Secondary link"
                )
            </div>
            label(size: LabelSize::Sm, "Busy on click")
            <div class="p-styleguide__row">
                button(
                    attrs: attributes! { @click=$(|_e: Event| is_busy.set(true)) },
                    busy: $(is_busy.get()),
                    "Click me"
                )
            </div>
        )
    })
}
