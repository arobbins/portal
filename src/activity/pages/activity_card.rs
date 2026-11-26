use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::account::tabs::AccountSection;
use crate::action::require_browser_caller;
use crate::activity::actions::{ActivityView, list_recent_activity};
use crate::auth::session::viewer_zone;
use crate::components::card::{card, card_body, card_header};
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;
use crate::time_zone::Zone;
use crate::web::env;

struct ActivityRow {
    key: String,
    summary: String,
    detail: String,
    tags: Vec<PropertyTag>,
}

impl ActivityRow {
    fn of(index: usize, item: &ActivityView, zone: &Zone) -> Self {
        let mut parts = vec![zone.date_time(item.at)];
        parts.extend(item.device.clone());
        parts.extend(item.ip.clone());
        let tags = item
            .by
            .as_deref()
            .map(|by| PropertyTag::new(by, Tone::Neutral))
            .into_iter()
            .collect();

        Self {
            key: index.to_string(),
            summary: item.summary.clone(),
            detail: parts.join(" · "),
            tags,
        }
    }
}

#[component]
pub async fn activity_card(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let activity = match list_recent_activity(&env(cx), &caller).await? {
        Ok(activity) => activity,
        Err(error) => match error.for_browser()? {},
    };
    Ok(view! {
        card(
            id: Some(AccountSection::Activity.anchor()),
            card_header(title: "Recent activity")
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
