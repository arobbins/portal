use crate::routes::LOGOUT_PATH;
use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::Signal,
    view::{View, component, view},
};

use crate::account::tabs::AccountSection;
use crate::admin::pages::users_page::admin_users_page;
use crate::components::avatar::avatar;
use crate::components::icon::{IconName, icon};
use crate::components::menu::{menu, menu_form_button, menu_link};

const MENU_ID: &str = "account-menu";

#[component]
pub async fn profile_menu(
    cx: &Cx,
    display_name: &Signal<String>,
    #[default] is_admin: bool,
) -> Result<impl View> {
    let logout_href = LOGOUT_PATH.to_string();
    let subscriptions_href = AccountSection::Subscriptions.path();
    let admin_href = href!(admin_users_page).resolve(cx);

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
            if is_admin {
                menu_link(
                    href: &admin_href,
                    icon(name: IconName::Settings)
                    "Admin"
                )
            }
            menu_form_button(
                action: &logout_href,
                icon(name: IconName::Logout)
                "Sign out"
            )
        )
    })
}
