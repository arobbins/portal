use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use super::{logo::logo, nav::nav};
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::icon::{IconName, icon};
use crate::components::theme_switcher::theme_switcher;

const MENU_ID: &str = "site-menu";

#[component]
pub async fn header(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <header class="l-contain header">
            <div class="header__logo">logo()</div>
            <div class="header__menu-toggle">
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! { popovertarget=(MENU_ID) aria-label="Menu" },
                    icon(name: IconName::Menu)
                )
            </div>
            <div class="header__menu" id=(MENU_ID) popover="">
                nav()
                <div class="header__theme">theme_switcher(compact: true)</div>
            </div>
        </header>
    })
}
