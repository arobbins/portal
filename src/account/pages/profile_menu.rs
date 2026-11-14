use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::Signal,
    view::{View, component, view},
};

use crate::account::tabs::AccountSection;
use crate::auth::pages::logout::logout;
use crate::components::avatar::avatar;
use crate::components::icon::{IconName, icon};
use crate::components::menu::{menu, menu_form_button, menu_link};

const MENU_ID: &str = "account-menu";

#[component]
pub async fn profile_menu(cx: &Cx, display_name: &Signal<String>) -> Result<impl View> {
    let logout_href = href!(logout).resolve(cx);
    let subscriptions_href = AccountSection::Subscriptions.href(cx);

    Ok(view! {
        <button popovertarget=(MENU_ID) class="p-account__profile-dropdown l-flex">
            avatar()
            <span class="p-account__profile-dropdown-name">$(display_name.get())</span>
            icon(name: IconName::ChevronDown)
        </button>
        menu(
            id: MENU_ID,
            menu_link(
                href: &subscriptions_href,
                icon(name: IconName::CreditCard)
                "Manage subscription"
            )
            menu_form_button(
                action: &logout_href,
                icon(name: IconName::Logout)
                "Sign out"
            )
        )
    })
}
