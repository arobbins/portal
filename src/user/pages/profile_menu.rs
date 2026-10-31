use leptos::prelude::*;

use crate::components::{Icon, IconName};
use crate::user::api::Logout;
use crate::user::model::AccountUser;

#[component]
pub fn ProfileMenu(user: AccountUser) -> impl IntoView {
    let logout = ServerAction::<Logout>::new();

    let photo = match user.photo {
        Some(photo) => view! {
            <img
                src=photo
                alt=format!("Profile photo of {}", user.name)
                class="account__profile-dropdown-photo"
            />
        }
            .into_any(),
        None => view! {
            <div class="account__profile-dropdown-photo-fallback">
                <Icon name=IconName::User/>
            </div>
        }
            .into_any(),
    };

    view! {
        <button popovertarget="account-menu" class="account__profile-dropdown l-flex">
            {photo}
            <span class="account__profile-dropdown-name">{user.name}</span>
            <Icon name=IconName::ChevronDown/>
        </button>
        <div id="account-menu" class="account__menu" popover="">
            <ul class="nav__list nav__list--flush-left l-flex-col">
                <li class="nav__list-item">
                    <a class="nav__link" href="/account/billing#subscriptions">
                        <Icon name=IconName::CreditCard/>
                        " Manage Subscription"
                    </a>
                </li>
                <li class="nav__list-item">
                    <ActionForm action=logout>
                        <button type="submit" class="nav__link nav__link--button">
                            <Icon name=IconName::Logout/>
                            " Logout"
                        </button>
                    </ActionForm>
                </li>
            </ul>
        </div>
    }
}
