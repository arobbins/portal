use leptos::prelude::*;

#[component]
pub fn PageHeading(#[prop(into)] text: String) -> impl IntoView {
    view! { <h1 class="page-heading">{text}</h1> }
}
