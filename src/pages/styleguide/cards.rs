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
use crate::components::table::{table, table_cell, table_heading, table_row};

const INVOICES: [(&str, &str, &str); 3] = [
    ("Nov 1, 2026", "Pro plan", "$29.00"),
    ("Oct 1, 2026", "Pro plan", "$29.00"),
    ("Sep 1, 2026", "Starter plan", "$9.00"),
];

#[component]
pub async fn cards_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Cards and tables",
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
            table(
                caption: Some("Invoices"),
                <thead>
                    table_row(
                        table_heading("Date")
                        table_heading("Description")
                        table_heading("Amount")
                    )
                </thead>
                <tbody>
                    #[key(date)]
                    for (date, description, amount) in INVOICES {
                        table_row(
                            table_cell((date))
                            table_cell((description))
                            table_cell((amount))
                        )
                    }
                </tbody>
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
                            "Create token"
                        )
                    )
                )
            )
        )
    })
}
