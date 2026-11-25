use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::account::tabs::AccountSection;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};

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
