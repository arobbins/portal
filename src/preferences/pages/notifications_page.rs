use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, attributes, component, view},
};

use super::newsletters_card::{newsletters_card, newsletters_saved_toast};
use super::notifications_routes::{save_notifications, update_notifications_submit};
use crate::account::tabs::AccountSection;
use crate::action::{ForPage, require_browser_caller};
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{form, form_actions};
use crate::components::setting_toggle::{InlineSave, setting_toggle};
use crate::flash::{Flash, flash_toast, flash_toasts, take_flash};
use crate::preferences::actions::{SIGN_IN_ALERTS_FIELD, get_newsletters, get_notifications};
use crate::web::env;

const FLASHES: &[Flash] = &[Flash::NotificationsSaved, Flash::NewslettersSaved];
const PLAIN_FLASHES: &[Flash] = &[Flash::NotificationsSaved];
const UNREACHABLE: &str = "We couldn't save that setting. Please try again.";

#[component]
pub async fn notifications_page(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let settings = get_notifications(&env(cx), &caller).await?.for_page()?;
    let newsletters = get_newsletters(&env(cx), &caller).await?.for_page()?;
    let toast = flash_toast(cx, take_flash(cx, FLASHES).await?);
    let action = href!(update_notifications_submit).resolve(cx);
    let save = InlineSave {
        procedure: save_notifications,
        success_toast: Flash::NotificationsSaved.name(),
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        <div class="p-account__sub-page-content">
            card(
                id: Some(AccountSection::Notifications.anchor()),
                card_header(title: AccountSection::Notifications.label())
                card_body(
                    <p>
                        "Security emails are always on: we email you whenever your password, email, passkeys, API tokens or two-factor settings change."
                    </p>
                    form(
                        attrs: attributes! { method="post" action=(action) },
                        setting_toggle(
                            label: "New sign-in alerts",
                            description: Some(
                                "Email me when someone signs in from a browser or device we haven't seen before.",
                            ),
                            id: "sign-in-alerts",
                            name: SIGN_IN_ALERTS_FIELD,
                            checked: settings.sign_in_alerts,
                            toast: &toast,
                            save: save
                        )
                        <noscript>
                            form_actions(
                                button(
                                    variant: ButtonVariant::Secondary,
                                    button_type: ButtonType::Submit,
                                    "Save notification settings"
                                )
                            )
                        </noscript>
                    )
                )
            )
            newsletters_card(settings: newsletters, toast: &toast)
            flash_toasts(toast: &toast, flashes: PLAIN_FLASHES)
            newsletters_saved_toast(toast: &toast)
        </div>
    })
}
