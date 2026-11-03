use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
pub async fn page_heading(text: &str) -> Result<impl View> {
    Ok(view! { <h1 class="page-heading">(text)</h1> })
}
