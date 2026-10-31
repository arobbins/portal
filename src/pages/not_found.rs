use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    let path = use_location().pathname.get_untracked();

    view! {
        <h1>"404"</h1>
        <p>"You requested " {path} " but we don't have that page!!"</p>
    }
}
