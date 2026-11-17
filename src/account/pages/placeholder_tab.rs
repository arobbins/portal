use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::account::tabs::{AccountSection, AccountTab};
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};

#[component]
pub async fn placeholder_tab(cx: &Cx, tab: AccountTab) -> Result<impl View> {
    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: tab.heading(), variant: PageHeadingVariant::Compact)
            for section in tab.sections() {
                coming_soon_card(
                    section: *section,
                    action: upcoming(*section).0,
                    text: upcoming(*section).1
                )
            }
        </div>
    })
}

#[component]
pub async fn coming_soon_card(
    cx: &Cx,
    section: AccountSection,
    action: &str,
    text: &str,
) -> Result<impl View> {
    Ok(view! {
        card(
            id: Some(section.anchor()),
            card_header(
                title: section.label(),
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    disabled: true,
                    (action)
                )
            )
            card_body(<p>(text)</p>)
        )
    })
}

fn upcoming(section: AccountSection) -> (&'static str, &'static str) {
    match section {
        AccountSection::Subscriptions => {
            ("Choose a plan", "See and change your plan. Coming soon.")
        }
        AccountSection::Invoices => ("View", "Download receipts for past payments. Coming soon."),
        AccountSection::PaymentMethods => (
            "Add",
            "Keep a card on file for your subscription. Coming soon.",
        ),
        AccountSection::Notifications => {
            ("Choose", "Pick which emails you get from us. Coming soon.")
        }
        _ => ("Set up", "Coming soon."),
    }
}
