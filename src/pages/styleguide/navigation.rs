use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonVariant, button};
use crate::components::icon::{IconName, icon};
use crate::components::menu::{menu, menu_link};
use crate::components::tabs::{tab, tabs};

const MENU_ID: &str = "styleguide-menu";

#[component]
pub async fn navigation_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Navigation",
            tabs(
                label: "Billing",
                tab(href: "#styleguide-tabs", active: true, "Subscriptions")
                tab(href: "#styleguide-tabs", "Invoices")
                tab(href: "#styleguide-tabs", "Payment methods")
            )
            <div class="styleguide__row">
                button(
                    variant: ButtonVariant::Secondary,
                    attrs: attributes! { popovertarget=(MENU_ID) },
                    "Open menu"
                    icon(name: IconName::ChevronDown)
                )
            </div>
            menu(
                id: MENU_ID,
                menu_link(
                    href: "#styleguide-menu",
                    icon(name: IconName::User)
                    "Profile"
                )
                menu_link(
                    href: "#styleguide-menu",
                    icon(name: IconName::Settings)
                    "Settings"
                )
                menu_link(
                    href: "#styleguide-menu",
                    icon(name: IconName::Logout)
                    "Logout"
                )
            )
        )
    })
}
