use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::Signal,
    view::{View, component, view},
};

use super::logout::logout;
use crate::components::avatar::avatar;
use crate::components::icon::{IconName, icon};
use crate::components::menu::{menu, menu_form_button, menu_link};
use crate::user::model::User;

const MENU_ID: &str = "account-menu";

#[component]
pub async fn profile_menu(
    cx: &Cx,
    user: &User,
    display_name: &Signal<String>,
) -> Result<impl View> {
    let logout_href = href!(logout).resolve(cx);

    Ok(view! {
        <button popovertarget=(MENU_ID) class="account__profile-dropdown l-flex">
            avatar(photo: user.photo.as_deref())
            <span class="account__profile-dropdown-name">$(display_name.get())</span>
            icon(name: IconName::ChevronDown)
        </button>
        menu(
            id: MENU_ID,
            menu_link(
                href: "/account/billing#subscriptions",
                icon(name: IconName::CreditCard)
                "Manage Subscription"
            )
            menu_form_button(
                action: &logout_href,
                icon(name: IconName::Logout)
                "Logout"
            )
        )
    })
}
