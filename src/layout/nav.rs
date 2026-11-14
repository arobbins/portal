use crate::routes::account_url;
use crate::routes::api_docs_url;
use crate::routes::login_url;
use crate::routes::register_url;
use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, component, view},
};

use crate::pages::about::about_page;
use crate::pages::home::home_page;
use crate::pages::styleguide::styleguide_page;
use crate::pages::webtransport::webtransport_page;
use crate::web::globals;

#[component]
async fn nav_item(cx: &Cx, href: String, label: &str) -> Result<impl View> {
    Ok(view! {
        <li class="l-nav__list-item">
            <a href=(href.as_str()) class="l-nav__link">(label)</a>
        </li>
    })
}

#[component]
pub async fn nav(cx: &Cx) -> Result<impl View> {
    let is_signed_in = globals(cx).current_user.is_some();

    Ok(view! {
        <nav class="l-nav l-nav--site" aria-label="Main">
            <ul class="l-nav__list l-nav__list--site">
                nav_item(href: href!(home_page).resolve(cx), label: "Home")
                nav_item(href: href!(about_page).resolve(cx), label: "About")
                nav_item(
                    href: href!(webtransport_page).resolve(cx),
                    label: "WebTransport"
                )
                nav_item(href: href!(styleguide_page).resolve(cx), label: "Styleguide")
                nav_item(href: api_docs_url(cx), label: "API docs")
                if is_signed_in {
                    nav_item(href: account_url(cx), label: "Account")
                } else {
                    nav_item(href: register_url(cx), label: "Register")
                    nav_item(href: login_url(cx), label: "Sign in")
                }
            </ul>
        </nav>
    })
}
