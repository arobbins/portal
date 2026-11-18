use topcoat::{
    Result,
    context::Cx,
    router::{href, request},
    view::{View, component, view},
};

use crate::auth::session::current_user;
use crate::components::button::{ButtonVariant, button};
use crate::layout::page_heading::page_heading;
use crate::pages::home::home_page;
use crate::routes::{ACCOUNT_PATH, LOGIN_PATH};

#[component]
pub async fn not_found_page(cx: &Cx) -> Result<impl View> {
    let path = request::uri(cx).path().to_string();
    let signed_in = current_user(cx).is_some();
    let home = href!(home_page).resolve(cx);

    Ok(view! {
        <section class="l-contain l-skinny p-not-found">
            <p class="p-not-found__code">"Error 404"</p>
            page_heading(text: "Page not found")
            <p class="a-lead p-not-found__text">
                "We couldn't find "
                <span class="p-not-found__path">(path)</span>
                ". The link may be mistyped, or the page may have moved."
            </p>
            <div class="p-not-found__actions">
                if signed_in {
                    button(href: Some(ACCOUNT_PATH), "Go to your account")
                    button(
                        variant: ButtonVariant::Secondary,
                        href: Some(&home),
                        "Go to the home page"
                    )
                } else {
                    button(href: Some(&home), "Go to the home page")
                    button(
                        variant: ButtonVariant::Secondary,
                        href: Some(LOGIN_PATH),
                        "Sign in"
                    )
                }
            </div>
        </section>
    })
}
