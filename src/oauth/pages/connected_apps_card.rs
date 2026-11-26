use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, attributes, component, view},
};

use super::disconnect_route::disconnect_app_submit;
use crate::account::tabs::AccountSection;
use crate::action::{ID_FIELD, require_browser_caller};
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;
use crate::oauth::actions::{Connection, list_connected_apps};
use crate::oauth::app_label;
use crate::time_zone::Zone;
use crate::web::env;

const DISCONNECT_MODAL: &str = "disconnect-app";

struct AppRow {
    id: String,
    name: String,
    label: String,
    tags: Vec<PropertyTag>,
    detail: String,
}

impl AppRow {
    fn new(connection: &Connection, zone: &Zone) -> Self {
        let last_used = connection.last_used_at.map_or_else(
            || "never used".to_string(),
            |at| format!("last used {}", zone.date(at)),
        );
        Self {
            id: connection.id.clone(),
            name: connection.name.clone(),
            label: app_label(&connection.name, &connection.return_host),
            tags: connection
                .scopes
                .iter()
                .map(|scope| PropertyTag::new(scope.label(), Tone::Neutral))
                .collect(),
            detail: format!(
                "Returns to {} · Connected {} · {last_used}",
                connection.return_host,
                zone.date(connection.connected_at),
            ),
        }
    }
}

#[component]
pub async fn connected_apps_card(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let connections = match list_connected_apps(&env(cx), &caller).await? {
        Ok(connections) => connections,
        Err(error) => match error.for_browser()? {},
    };
    let zone = caller.user.zone();
    let rows: Vec<AppRow> = connections
        .iter()
        .map(|connection| AppRow::new(connection, &zone))
        .collect();

    Ok(view! {
        card(
            id: Some(AccountSection::ConnectedApps.anchor()),
            card_header(title: "Connected apps")
            card_body(
                <p>
                    "AI assistants you connected by signing in. They can use your account within what you allowed until you disconnect them."
                </p>
                if rows.is_empty() {
                    <p>"No apps connected."</p>
                } else {
                    property_list(
                        #[key(row.id.as_str())]
                        for row in &rows {
                            app_item(row: row)
                        }
                    )
                }
            )
        )
        #[key(row.id.as_str())]
        for row in &rows {
            disconnect_dialog(row: row)
        }
    })
}

#[component]
async fn app_item(cx: &Cx, row: &AppRow) -> Result<impl View> {
    let modal = format!("{DISCONNECT_MODAL}-{}", row.id);

    Ok(view! {
        property(
            name: row.name.as_str(),
            tags: &row.tags,
            detail: Some(row.detail.as_str()),
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                attrs: attributes! { command="show-modal" commandfor=(modal.as_str()) },
                "Disconnect app"
            )
        )
    })
}

#[component]
async fn disconnect_dialog(cx: &Cx, row: &AppRow) -> Result<impl View> {
    let modal = format!("{DISCONNECT_MODAL}-{}", row.id);
    let action = href!(disconnect_app_submit).resolve(cx);

    Ok(view! {
        confirm_dialog(
            id: &modal,
            heading: "Disconnect app",
            confirm_label: "Disconnect app",
            action: Some(&action),
            <input type="hidden" name=(ID_FIELD) value=(row.id.as_str())>
            <p>
                "Disconnect "
                <strong>(row.label.as_str())</strong>
                "? It will stop working right away and has to sign in again to reconnect."
            </p>
        )
    })
}
