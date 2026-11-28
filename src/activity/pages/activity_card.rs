use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::activity_view::ActivityRow;
use crate::account::tabs::AccountSection;
use crate::action::{ForPage, require_browser_caller};
use crate::activity::actions::{ActivityView, list_recent_activity};
use crate::auth::session::viewer_zone;
use crate::components::card::{card, card_body, card_header};
use crate::components::property_list::{property, property_list};
use crate::web::env;

#[component]
pub async fn activity_card(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let activity = list_recent_activity(&env(cx), &caller).await?.for_page()?;
    Ok(view! {
        card(
            id: Some(AccountSection::Activity.anchor()),
            card_header(title: AccountSection::Activity.label())
            card_body(
                <p>"Sign-ins and changes to your account, newest first."</p>
                activity_list(activity: &activity)
            )
        )
    })
}

#[component]
pub async fn activity_list(cx: &Cx, activity: &[ActivityView]) -> Result<impl View> {
    let zone = viewer_zone(cx);
    let rows: Vec<ActivityRow> = activity
        .iter()
        .enumerate()
        .map(|(index, item)| ActivityRow::of(index, item, &zone))
        .collect();

    Ok(view! {
        if rows.is_empty() {
            <p>"No activity yet."</p>
        }
        property_list(
            #[key(row.key.as_str())]
            for row in &rows {
                property(
                    name: row.summary.as_str(),
                    tags: &row.tags,
                    detail: Some(row.detail.as_str())
                )
            }
        )
    })
}
