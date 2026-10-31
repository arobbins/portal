use leptos::prelude::*;

use crate::web::use_globals;

#[component]
fn NavItem(href: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <li class="nav__list-item">
            <a href=href class="nav__link">{label}</a>
        </li>
    }
}

#[component]
pub fn Nav() -> impl IntoView {
    let is_signed_in = use_globals().current_user.is_some();

    view! {
        <nav class="l-contain l-flex nav">
            <ul class="nav__list l-flex">
                <NavItem href="/" label="Home"/>
                <NavItem href="/about" label="About"/>
                <NavItem href="/webtransport" label="WebTransport"/>
                <NavItem href="/styleguide" label="Styleguide"/>
                {if is_signed_in {
                    view! { <NavItem href="/account" label="Account"/> }.into_any()
                } else {
                    view! {
                        <NavItem href="/register" label="Register"/>
                        <NavItem href="/login" label="Login"/>
                    }
                        .into_any()
                }}
            </ul>
        </nav>
    }
}
