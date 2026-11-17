use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::empty_state::empty_state;
use crate::components::icon::{IconName, icon};

#[component]
pub async fn cards_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Cards",
            card(
                card_header(
                    title: "Account info",
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        icon(name: IconName::Edit)
                        "Edit details"
                    )
                )
                card_body(
                    <p>
                        "A card groups one setting or topic, with its action in the header."
                    </p>
                )
            )
            card(
                card_header(title: "Card without an action")
                card_body(
                    <p>"The header keeps the same height when there's no button."</p>
                )
            )
            card(
                card_header(title: "API tokens")
                card_body(
                    empty_state(
                        heading: "No API tokens yet",
                        icon_name: Some(IconName::Settings),
                        <p>
                            "Create a token to access the API from scripts and other services."
                        </p>
                        button(
                            size: ButtonSize::Sm,
                            icon(name: IconName::Plus)
                            "Create API token"
                        )
                    )
                )
            )
        )
    })
}
