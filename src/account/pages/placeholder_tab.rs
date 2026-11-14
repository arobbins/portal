use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::empty_state::empty_state;
use crate::components::icon::IconName;
use crate::layout::page_heading::{PageHeadingVariant, page_heading};

#[component]
pub async fn placeholder_tab(cx: &Cx, heading: &str) -> Result<impl View> {
    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: heading, variant: PageHeadingVariant::Compact)
            empty_state(
                heading: "Coming soon",
                icon_name: Some(IconName::Settings),
                <p>"This section isn't available yet."</p>
            )
        </div>
    })
}
