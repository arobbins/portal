use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use super::account_nav::AccountNav;
use super::account_profile::AccountProfile;
use super::account_security::AccountSecurity;
use super::profile_menu::ProfileMenu;
use crate::components::{Notice, NoticeKind};
use crate::user::api::get_account_user;
use crate::user::model::AccountUser;
use crate::web::error_message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTab {
    Profile,
    Security,
    Billing,
    Settings,
    Developer,
}

impl AccountTab {
    fn from_slug(slug: Option<&str>) -> Self {
        match slug {
            Some("security") => Self::Security,
            Some("billing") => Self::Billing,
            Some("settings") => Self::Settings,
            Some("developer") => Self::Developer,
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

#[component]
pub fn AccountPage() -> impl IntoView {
    let slug = use_params_map().read_untracked().get("page");
    let tab = AccountTab::from_slug(slug.as_deref());

    view! {
        <Await future=get_account_user() blocking=true let:user>
            {match user.clone() {
                Ok(Some(user)) => view! { <AccountLayout user=user tab=tab/> }.into_any(),
                Ok(None) => ().into_any(),
                Err(error) => {
                    view! { <Notice kind=NoticeKind::Error>{error_message(&error)}</Notice> }
                        .into_any()
                }
            }}
        </Await>
    }
}

#[component]
fn AccountLayout(user: AccountUser, tab: AccountTab) -> impl IntoView {
    let content = match tab {
        AccountTab::Security => view! { <AccountSecurity/> }.into_any(),
        _ => view! { <AccountProfile user=user.clone()/> }.into_any(),
    };

    view! {
        <section class="account">
            <div class="l-flex l-flex-1">
                <nav class="account__sub-nav nav">
                    <div class="l-flex l-flex-space-between l-flex-align-items-center account__header">
                        <div class="l-flex l-flex-1 l-flex-align-items-center">
                            <ProfileMenu user=user/>
                        </div>
                    </div>
                    <AccountNav active=tab/>
                </nav>
                <section class="account__content" id="account__content">
                    {content}
                </section>
            </div>
        </section>
    }
    .into_any()
}
