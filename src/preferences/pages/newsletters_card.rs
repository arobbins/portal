use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::Signal,
    view::{View, attributes, component, view},
};

use super::newsletters_routes::{
    save_general_newsletter, save_product_updates, update_newsletters_submit,
};
use crate::account::tabs::AccountSection;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{form, form_actions};
use crate::components::setting_toggle::{InlineSave, setting_toggle};
use crate::components::toaster::toaster;
use crate::components::tone::Tone;
use crate::flash::Flash;
use crate::preferences::actions::NewsletterSettings;
use crate::preferences::newsletters::Newsletter;

const UNREACHABLE: &str = "We couldn't save your newsletter preferences. Please try again.";

#[component]
pub async fn newsletters_card(
    cx: &Cx,
    settings: NewsletterSettings,
    toast: &Signal<String>,
) -> Result<impl View> {
    let action = href!(update_newsletters_submit).resolve(cx);
    let general = Newsletter::GeneralNewsletter;
    let product = Newsletter::ProductUpdates;

    Ok(view! {
        card(
            id: Some(AccountSection::Newsletters.anchor()),
            card_header(title: AccountSection::Newsletters.label())
            card_body(
                <p>
                    "The General newsletter brings news about the app now and then; Product updates tell you about new features and changes. Change your mind at any time."
                </p>
                form(
                    attrs: attributes! { method="post" action=(action) },
                    setting_toggle(
                        label: general.label(),
                        id: "general-newsletter",
                        name: general.as_str(),
                        checked: settings.general_newsletter,
                        toast: toast,
                        save: InlineSave {
                            procedure: save_general_newsletter,
                            success_toast: Flash::NewslettersSaved.name(),
                            unreachable: UNREACHABLE,
                        }
                    )
                    setting_toggle(
                        label: product.label(),
                        id: "product-updates",
                        name: product.as_str(),
                        checked: settings.product_updates,
                        toast: toast,
                        save: InlineSave {
                            procedure: save_product_updates,
                            success_toast: Flash::NewslettersSaved.name(),
                            unreachable: UNREACHABLE,
                        }
                    )
                    <noscript>
                        form_actions(
                            button(
                                variant: ButtonVariant::Secondary,
                                button_type: ButtonType::Submit,
                                "Save newsletter preferences"
                            )
                        )
                    </noscript>
                )
            )
        )
    })
}

#[component]
pub async fn newsletters_saved_toast(cx: &Cx, toast: &Signal<String>) -> Result<impl View> {
    let name = Flash::NewslettersSaved.name();
    let message = Flash::NewslettersSaved.message();

    Ok(view! {
        toaster(
            toast: toast,
            name: name,
            tone: Tone::Success,
            $(if toast.get() == name { message } else { "" })
        )
    })
}
