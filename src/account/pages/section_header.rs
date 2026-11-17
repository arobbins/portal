use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::tabs::{tab, tabs};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};

pub struct SubPage {
    pub label: &'static str,
    pub href: String,
    pub current: bool,
}

#[component]
pub async fn section_header(cx: &Cx, title: &str, pages: &[SubPage]) -> Result<impl View> {
    Ok(view! {
        <header class="p-account__section-header">
            page_heading(text: title, variant: PageHeadingVariant::Compact)
            if pages.len() > 1 {
                tabs(
                    label: title,
                    for page in pages {
                        tab(
                            href: page.href.as_str(),
                            active: page.current,
                            (page.label)
                        )
                    }
                )
            }
        </header>
    })
}
