use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param},
    runtime::signal,
    view::{View, component, view},
};

use super::account_nav::account_nav;
use super::account_profile::account_profile;
use super::account_security::account_security;
use super::profile_menu::profile_menu;
use crate::components::theme_switcher::theme_switcher;
use crate::user::auth::require_user;
use crate::user::model::User;

path_param!(tab);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTab {
    Profile,
    Security,
    Billing,
    Settings,
    Developer,
}

impl AccountTab {
    fn from_slug(slug: &str) -> Self {
        match slug {
            "security" => Self::Security,
            "billing" => Self::Billing,
            "settings" => Self::Settings,
            "developer" => Self::Developer,
            _ => Self::Profile,
        }
    }

    pub fn href(self) -> &'static str {
        match self {
            Self::Profile => "/account/profile",
            Self::Security => "/account/security",
            Self::Billing => "/account/billing",
            Self::Settings => "/account/settings",
            Self::Developer => "/account/developer",
        }
    }
}

#[page("/account")]
pub async fn account_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;

    Ok(view! { account_layout(user: &user, tab: AccountTab::Profile) })
}

#[page("/account/{tab}")]
pub async fn account_tab_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let tab = AccountTab::from_slug(path_param::<Tab>(cx));

    Ok(view! { account_layout(user: &user, tab: tab) })
}

#[component]
async fn account_layout(cx: &Cx, user: &User, tab: AccountTab) -> Result<impl View> {
    let display_name = signal(cx, || user.name.clone());

    Ok(view! {
        <section class="account">
            <div class="l-flex l-flex-1">
                <nav class="account__sub-nav nav">
                    <div class="l-flex l-flex-space-between l-flex-align-items-center account__header">
                        <div class="l-flex l-flex-1 l-flex-align-items-center">
                            profile_menu(user: user, display_name: &display_name)
                        </div>
                    </div>
                    account_nav(active: tab)
                    <div class="account__theme">
                        theme_switcher(compact: true)
                    </div>
                </nav>
                <section class="account__content" id="account__content">
                    match tab {
                        AccountTab::Security => account_security(),
                        _ => account_profile(user: user, display_name: &display_name),
                    }
                </section>
            </div>
        </section>
    })
}
