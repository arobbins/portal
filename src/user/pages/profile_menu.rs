use topcoat::{
    Result,
    router::href,
    view::{View, component, view},
};

use super::logout::logout;
use crate::components::icon::{IconName, icon};
use crate::user::model::User;

#[component]
pub async fn profile_menu(user: &User) -> Result<impl View> {
    let photo_alt = format!("Profile photo of {}", user.name);

    Ok(view! {
        <button popovertarget="account-menu" class="account__profile-dropdown l-flex">
            match &user.photo {
                Some(photo) => <img src=(photo) alt=(photo_alt) class="account__profile-dropdown-photo">,
                None => {
                    <div class="account__profile-dropdown-photo-fallback">
                        icon(name: IconName::User)
                    </div>
                },
            }
            <span class="account__profile-dropdown-name">(&user.name)</span>
            icon(name: IconName::ChevronDown)
        </button>
        <div id="account-menu" class="account__menu" popover="">
            <ul class="nav__list nav__list--flush-left l-flex-col">
                <li class="nav__list-item">
                    <a class="nav__link" href="/account/billing#subscriptions">
                        icon(name: IconName::CreditCard)
                        " Manage Subscription"
                    </a>
                </li>
                <li class="nav__list-item">
                    <form method="post" action=(href!(logout))>
                        <button type="submit" class="nav__link nav__link--button">
                            icon(name: IconName::Logout)
                            " Logout"
                        </button>
                    </form>
                </li>
            </ul>
        </div>
    })
}
