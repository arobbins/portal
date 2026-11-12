use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::toggle::toggle;

#[component]
pub async fn toggles_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Toggles",
            toggle(
                label: "Product updates",
                id: "styleguide-toggle-updates",
                name: "product_updates",
                description: Some("News about features and improvements."),
                checked: true
            )
            toggle(
                label: "Security alerts",
                id: "styleguide-toggle-security",
                name: "security_alerts"
            )
            toggle(
                label: "Required notices",
                id: "styleguide-toggle-required",
                name: "required_notices",
                description: Some("Account and legal emails can't be turned off."),
                checked: true,
                disabled: true
            )
        )
    })
}
