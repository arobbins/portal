use leptos::prelude::*;

use crate::web::use_globals;

#[component]
pub fn Footer() -> impl IntoView {
    let year = use_globals().year;

    view! {
        <footer class="l-contain footer">
            <p>"© " {year} " Andrew Robbins / Github / Email"</p>
        </footer>
    }
}
