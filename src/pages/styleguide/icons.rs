use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::icon::{IconName, icon};
use crate::components::label::{LabelSize, label};
use crate::layout::logo::logo;
use crate::layout::page_heading::{PageHeadingVariant, page_heading};

#[component]
pub async fn icons_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Icons and layout",
            <div class="p-styleguide__icons">
                #[key(name.label())]
                for name in IconName::ALL {
                    <div class="p-styleguide__icon">
                        icon(name: name)
                        (name.label())
                    </div>
                }
            </div>
            label(size: LabelSize::Sm, "Logo")
            <div class="p-styleguide__row">logo()</div>
            label(size: LabelSize::Sm, "Page heading: centered (default)")
            page_heading(text: "Page heading")
            label(size: LabelSize::Sm, "Page heading: compact, for app and docs pages")
            page_heading(text: "Compact heading", variant: PageHeadingVariant::Compact)
        )
    })
}
