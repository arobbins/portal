use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page},
    view::{View, view},
};

use crate::app::{SITE_NAME, page_title};
use crate::auth::pages::login::login_page;
use crate::auth::pages::register::register_page;
use crate::components::button::{ButtonVariant, button};
use crate::pages::api_docs::api_docs_page;

#[page("/", with(Metadata::new().title(page_title("Home"))))]
pub async fn home_page(cx: &Cx) -> Result<impl View> {
    let register_href = href!(register_page).resolve(cx);
    let login_href = href!(login_page).resolve(cx);
    let docs_href = href!(api_docs_page).resolve(cx);

    Ok(view! {
        <section class="l-contain l-skinny p-home">
            <h1 class="p-home__title">(SITE_NAME)</h1>
            <p class="p-home__intro">
                "A starting point for your next product: accounts, passkeys, API tokens and a documented JSON API, ready to build on."
            </p>
            <div class="p-home__actions">
                button(href: Some(&register_href), "Create an account")
                button(
                    variant: ButtonVariant::Secondary,
                    href: Some(&login_href),
                    "Sign in"
                )
                button(
                    variant: ButtonVariant::Secondary,
                    href: Some(&docs_href),
                    "Read the API docs"
                )
            </div>
        </section>
    })
}
