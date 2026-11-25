use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{
        error::{not_found, see_other},
        page, path_param,
    },
    runtime::signal,
    view::{View, component, view},
};

use crate::account::pages::account_shell::account_shell;
use crate::account::pages::devices_page::devices_page;
use crate::account::pages::section_header::{SubPage, section_header};
use crate::account::pages::sign_in_page::sign_in_page;
use crate::account::tabs::{AccountArea, AccountPage, AccountTab};
use crate::api_tokens::pages::developer_tab::developer_tab;
use crate::app::page_title;
use crate::auth::session::require_user;
use crate::billing::pages::invoices_page::invoices_page;
use crate::billing::pages::plan_page::plan_page;
use crate::preferences::pages::notifications_page::notifications_page;
use crate::preferences::pages::time_zone_page::time_zone_page;
use crate::profile::pages::profile_tab::profile_tab;
use crate::user::model::User;

path_param!(pub tab);
path_param!(pub sub);

#[page("/account", with(Metadata::new().title(page_title("Profile"))))]
pub async fn account_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;

    Ok(view! { account_layout(user: user.as_ref(), page: AccountPage::Profile) })
}

#[page(
    "/account/{tab}",
    with(Metadata::from_fn(
        async |cx| {
            let title = AccountPage::from_slugs(tab(cx), None).map_or(
                "Account",
                AccountPage::heading,
            );
            Ok(Metadata::new().title(page_title(title)))
        },
    ))
)]
pub async fn account_tab_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    let Some(section) = AccountTab::from_slug(tab(cx)) else {
        return Err(not_found().into());
    };
    let page = AccountPage::from_slugs(tab(cx), None);
    let Some(page) = page.filter(|page| page.path().ends_with(tab(cx))) else {
        return Err(see_other(section.path()).into());
    };

    Ok(view! { account_layout(user: user.as_ref(), page: page) })
}

#[page(
    "/account/{tab}/{sub}",
    with(Metadata::from_fn(
        async |cx| {
            let title = AccountPage::from_slugs(tab(cx), Some(sub(cx))).map_or(
                "Account",
                AccountPage::heading,
            );
            Ok(Metadata::new().title(page_title(title)))
        },
    ))
)]
pub async fn account_sub_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    let Some(page) = AccountPage::from_slugs(tab(cx), Some(sub(cx))) else {
        return Err(not_found().into());
    };

    Ok(view! { account_layout(user: user.as_ref(), page: page) })
}

#[component]
async fn account_layout(cx: &Cx, user: &User, page: AccountPage) -> Result<impl View> {
    let display_name = signal(cx, || user.name.clone());
    let section = page.tab();
    let pages: Vec<SubPage> = section
        .pages()
        .iter()
        .map(|candidate| SubPage {
            label: candidate.heading(),
            href: candidate.path(),
            current: *candidate == page,
        })
        .collect();

    Ok(view! {
        account_shell(
            user: user,
            display_name: &display_name,
            active: AccountArea::Page(page),
            section_header(title: section.heading(), pages: &pages)
            match page {
                AccountPage::Profile => profile_tab(
                    user: user,
                    display_name: &display_name
                ),
                AccountPage::SignIn => sign_in_page(),
                AccountPage::Devices => devices_page(),
                AccountPage::Developer => developer_tab(),
                AccountPage::Notifications => notifications_page(),
                AccountPage::TimeZone => time_zone_page(user: user),
                AccountPage::Plan => plan_page(),
                AccountPage::Invoices => invoices_page(),
            }
        )
    })
}
