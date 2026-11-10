use topcoat::{
    Result,
    context::Cx,
    router::{error::not_found, page, path_param},
    runtime::signal,
    view::{View, component, view},
};

use super::account_nav::account_nav;
use super::account_placeholder::account_placeholder;
use super::account_profile::account_profile;
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
    const ALL: [Self; 5] = [
        Self::Profile,
        Self::Security,
        Self::Billing,
        Self::Settings,
        Self::Developer,
    ];

    fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|tab| tab.href().strip_prefix("/account/") == Some(slug))
    }

    pub fn heading(self) -> &'static str {
        match self {
            Self::Profile => "Profile",
            Self::Security => "Security",
            Self::Billing => "Billing",
            Self::Settings => "Settings",
            Self::Developer => "Developer",
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
    let user = require_user(cx)?;

    Ok(view! { account_layout(user: user.as_ref(), tab: AccountTab::Profile) })
}

#[page("/account/{tab}")]
pub async fn account_tab_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    let Some(tab) = AccountTab::from_slug(path_param::<Tab>(cx)) else {
        return Err(not_found().into());
    };

    Ok(view! { account_layout(user: user.as_ref(), tab: tab) })
}

#[component]
async fn account_layout(cx: &Cx, user: &User, tab: AccountTab) -> Result<impl View> {
    let display_name = signal(cx, || user.name.clone());

    Ok(view! {
        <section class="account">
            <div class="l-flex l-flex-1">
                <nav class="account__sub-nav nav">
                    <div
                        class="l-flex l-flex-space-between l-flex-align-items-center account__header"
                    >
                        <div class="l-flex l-flex-1 l-flex-align-items-center">
                            profile_menu(user: user, display_name: &display_name)
                        </div>
                    </div>
                    account_nav(active: tab)
                    <div class="account__theme">theme_switcher(compact: true)</div>
                </nav>
                <section class="account__content" id="account__content">
                    match tab {
                        AccountTab::Profile => account_profile(
                            user: user,
                            display_name: &display_name
                        ),
                        _ => account_placeholder(heading: tab.heading()),
                    }
                </section>
            </div>
        </section>
    })
}
