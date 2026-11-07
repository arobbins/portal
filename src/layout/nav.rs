use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::web::globals;

#[component]
async fn nav_item(href: &str, label: &str) -> Result<impl View> {
    Ok(view! {
        <li class="nav__list-item"><a href=(href) class="nav__link">(label)</a></li>
    })
}

#[component]
pub async fn nav(cx: &Cx) -> Result<impl View> {
    let is_signed_in = globals(cx).current_user.is_some();

    Ok(view! {
        <nav class="l-contain l-flex nav">
            <ul class="nav__list l-flex">
                nav_item(href: "/", label: "Home")
                nav_item(href: "/about", label: "About")
                nav_item(href: "/webtransport", label: "WebTransport")
                nav_item(href: "/styleguide", label: "Styleguide")
                if is_signed_in {
                    nav_item(href: "/account", label: "Account")
                } else {
                    nav_item(href: "/register", label: "Register")
                    nav_item(href: "/login", label: "Login")
                }
            </ul>
        </nav>
    })
}
