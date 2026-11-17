use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{error::not_found, page, path_param},
    runtime::signal,
    view::{View, component, view},
};

use crate::account::pages::account_nav::account_nav;
use crate::account::pages::placeholder_tab::placeholder_tab;
use crate::account::pages::profile_menu::profile_menu;
use crate::account::pages::security_tab::security_tab;
use crate::account::tabs::AccountTab;
use crate::api_tokens::pages::developer_tab::developer_tab;
use crate::app::page_title;
use crate::auth::session::require_user;
use crate::components::theme_switcher::theme_switcher;
use crate::passkeys::webauthn::accepted_credentials;
use crate::profile::pages::profile_tab::profile_tab;
use crate::user::model::{Role, User};
use crate::web::app;

path_param!(pub tab);

#[page("/account", with(Metadata::new().title(page_title("Profile"))))]
pub async fn account_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;

    Ok(view! { account_layout(user: user.as_ref(), tab: AccountTab::Profile) })
}

#[page(
    "/account/{tab}",
    with(Metadata::from_fn(
        async |cx| {
            let title = AccountTab::from_slug(tab(cx)).map_or(
                "Account",
                AccountTab::heading,
            );
            Ok(Metadata::new().title(page_title(title)))
        },
    ))
)]
pub async fn account_tab_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    let Some(tab) = AccountTab::from_slug(tab(cx)) else {
        return Err(not_found().into());
    };

    Ok(view! { account_layout(user: user.as_ref(), tab: tab) })
}

#[component]
async fn account_layout(cx: &Cx, user: &User, tab: AccountTab) -> Result<impl View> {
    let display_name = signal(cx, || user.name.clone());
    let passkey_signals = accepted_credentials(app(cx), user).await?.to_string();

    Ok(view! {
        <section class="p-account">
            <div class="p-account__layout">
                <nav class="p-account__sub-nav l-nav">
                    <div
                        class="l-flex l-flex-space-between l-flex-align-items-center p-account__header"
                    >
                        <div class="l-flex l-flex-1 l-flex-align-items-center">
                            profile_menu(
                                display_name: &display_name,
                                is_admin: user.role == Role::Admin
                            )
                        </div>
                    </div>
                    account_nav(active: tab)
                    <div class="p-account__theme">theme_switcher(compact: true)</div>
                    <div hidden="" data-passkey-signals=(passkey_signals)></div>
                </nav>
                <section class="p-account__content">
                    match tab {
                        AccountTab::Profile => profile_tab(
                            user: user,
                            display_name: &display_name
                        ),
                        AccountTab::Security => security_tab(),
                        AccountTab::Developer => developer_tab(),
                        AccountTab::Billing | AccountTab::Settings => placeholder_tab(
                            tab: tab
                        ),
                    }
                </section>
            </div>
        </section>
    })
}
