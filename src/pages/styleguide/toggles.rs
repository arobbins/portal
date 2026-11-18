use topcoat::{
    Result,
    context::Cx,
    runtime::{procedure, signal},
    view::{View, component, view},
};

use super::styleguide_section;
use crate::action::FieldError;
use crate::components::setting_toggle::{InlineSave, setting_toggle};
use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::toggle::toggle;
use crate::components::tone::Tone;

const SAVED_TOAST: &str = "styleguide-setting-saved";

#[procedure]
async fn save_styleguide_setting(
    cx: &Cx,
    on: bool,
) -> topcoat::Result<std::result::Result<String, FieldError>> {
    let _ = on;
    Ok(Ok(String::new()))
}

#[component]
pub async fn toggles_section(cx: &Cx) -> Result<impl View> {
    let toast = signal(cx, || NO_TOAST.to_string());
    let save = InlineSave {
        procedure: save_styleguide_setting,
        success_toast: SAVED_TOAST,
        unreachable: "We couldn't save that setting. Please try again.",
    };

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
            setting_toggle(
                label: "Weekly summary",
                description: Some("Saves as soon as it's switched."),
                id: "styleguide-setting-toggle",
                name: "weekly_summary",
                checked: false,
                toast: &toast,
                save: save
            )
            toaster(toast: &toast, name: SAVED_TOAST, tone: Tone::Success, "Saved.")
        )
    })
}
