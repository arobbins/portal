use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PageHeadingVariant {
    #[default]
    Centered,
    Compact,
}

impl PageHeadingVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Centered => "l-page-heading",
            Self::Compact => "l-page-heading l-page-heading--compact",
        }
    }
}

#[component]
pub async fn page_heading(
    cx: &Cx,
    text: &str,
    #[default] variant: PageHeadingVariant,
) -> Result<impl View> {
    Ok(view! { <h1 class=(variant.class())>(text)</h1> })
}
