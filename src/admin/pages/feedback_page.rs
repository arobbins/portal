use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    view::{View, component, view},
};

use super::admin_access::require_admin;
use super::admin_shell::{AdminSection, admin_shell};
use crate::app::page_title;
use crate::auth::session::viewer_zone;
use crate::components::table::{
    Column, TableLayout, table, table_cell, table_row, table_row_heading,
};
use crate::feedback::actions::{FEEDBACK_SHOWN, FeedbackView, list_api_feedback};
use crate::web::env;

#[page("/admin/feedback", with(Metadata::new().title(page_title("API feedback"))))]
pub async fn admin_feedback_page(cx: &Cx) -> Result<impl View> {
    let caller = require_admin(cx).await?;
    let feedback = match list_api_feedback(&env(cx), &caller).await? {
        Ok(feedback) => feedback,
        Err(error) => match error.for_browser()? {},
    };
    let caption =
        format!("The last {FEEDBACK_SHOWN} messages agents sent about the API, newest first");

    Ok(view! {
        admin_shell(
            user: caller.user.as_ref(),
            section: AdminSection::Feedback,
            feedback_table(feedback: &feedback, caption: &caption)
        )
    })
}

const COLUMNS: [Column; 3] = [
    Column::new("Feedback").grow(),
    Column::new("From").truncated(),
    Column::new("When").date(),
];

fn details(item: &FeedbackView) -> String {
    [
        Some(item.category.label()),
        item.endpoint.as_deref(),
        item.problem_code.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

#[component]
async fn feedback_table(cx: &Cx, feedback: &[FeedbackView], caption: &str) -> Result<impl View> {
    let zone = viewer_zone(cx);
    let [message, from, when] = COLUMNS;
    let empty = feedback.is_empty().then_some("No feedback yet.");

    Ok(view! {
        table(
            caption: caption,
            columns: &COLUMNS,
            empty: empty,
            layout: TableLayout::Stacked,
            for item in feedback {
                table_row(
                    table_row_heading(
                        column: message,
                        <span class="p-admin__feedback-message">
                            (item.message.as_str())
                        </span>
                        <small class="p-admin__feedback-details">(details(item))</small>
                    )
                    table_cell(
                        column: from,
                        title: Some(item.from.as_str()),
                        (item.from.as_str())
                    )
                    table_cell(column: when, (zone.date_time(item.sent_at)))
                )
            }
        )
    })
}
