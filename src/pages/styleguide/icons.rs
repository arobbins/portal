use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::icon::{IconName, icon};
use crate::layout::logo::logo;
use crate::layout::page_heading::page_heading;

#[component]
pub async fn icons_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Icons and layout",
            <div class="styleguide__icons">
                #[key(name.label())]
                for name in IconName::ALL {
                    <div class="styleguide__icon">
                        icon(name: name)
                        (name.label())
                    </div>
                }
            </div>
            <p class="styleguide__label">"Logo"</p>
            <div class="styleguide__row">logo()</div>
            <p class="styleguide__label">"Page heading"</p>
            page_heading(text: "Page heading")
        )
    })
}
