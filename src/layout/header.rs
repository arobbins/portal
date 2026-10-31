use leptos::prelude::*;

use super::{Logo, Nav};

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="l-contain l-flex header">
            <div class="header__logo l-flex l-flex-center-col l-flex-1">
                <Logo/>
            </div>
            <div class="header__nav">
                <Nav/>
            </div>
        </header>
    }
}
