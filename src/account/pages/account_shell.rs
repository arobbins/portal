use topcoat::{
    Result,
    context::Cx,
    runtime::Signal,
    view::{Child, View, component, view},
};

use crate::account::pages::account_nav::account_nav;
use crate::account::pages::profile_menu::profile_menu;
use crate::account::tabs::AccountArea;
use crate::components::theme_switcher::theme_switcher;
use crate::passkeys::webauthn::accepted_credentials;
use crate::user::model::{Role, User};
use crate::web::app;

#[component]
pub async fn account_shell(
    cx: &Cx,
    user: &User,
    display_name: &Signal<String>,
    active: AccountArea,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let passkey_signals = accepted_credentials(app(cx), user).await?.to_string();
    let is_admin = user.role == Role::Admin;

    Ok(view! {
        <section class="p-account">
            <div class="p-account__layout">
                <nav class="p-account__sub-nav l-nav" aria-label="Account">
                    <div
                        class="l-flex l-flex-space-between l-flex-align-items-center p-account__header"
                    >
                        <div class="l-flex l-flex-1 l-flex-align-items-center">
                            profile_menu(display_name: display_name)
                        </div>
                    </div>
                    account_nav(active: active, is_admin: is_admin)
                    <div class="p-account__theme">theme_switcher(compact: true)</div>
                    <div hidden="" data-passkey-signals=(passkey_signals)></div>
                </nav>
                <section class="p-account__content">(child)</section>
            </div>
        </section>
    })
}
