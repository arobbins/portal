use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page},
    view::{View, component, view},
};

use super::admin_access::require_admin;
use super::admin_shell::{AdminSection, admin_shell};
use super::user_page::{admin_user_page, id};
use crate::admin::actions::{LOG_ENTRIES_SHOWN, list_admin_log};
use crate::admin::queries::LogEntry;
use crate::app::page_title;
use crate::components::table::{
    Column, TableLayout, table, table_cell, table_row, table_row_heading,
};
use crate::utils::format_date_time;
use crate::web::env;

const SERVER_CONSOLE: &str = "Server console";

#[page("/admin/log", with(Metadata::new().title(page_title("Admin log"))))]
pub async fn admin_log_page(cx: &Cx) -> Result<impl View> {
    let caller = require_admin(cx).await?;
    let entries = match list_admin_log(&env(cx), &caller).await? {
        Ok(entries) => entries,
        Err(error) => match error.for_browser()? {},
    };
    let caption = format!("The last {LOG_ENTRIES_SHOWN} admin actions, newest first");

    Ok(view! {
        admin_shell(
            user: caller.user.as_ref(),
            section: AdminSection::Log,
            log_table(entries: &entries, caption: &caption)
        )
    })
}

const COLUMNS: [Column; 4] = [
    Column::new("Action"),
    Column::new("When").date(),
    Column::new("Admin").truncated(),
    Column::new("User").truncated(),
];

#[component]
async fn log_table(cx: &Cx, entries: &[LogEntry], caption: &str) -> Result<impl View> {
    let [action, when, admin, user] = COLUMNS;
    let empty = entries.is_empty().then_some("No admin actions yet.");

    Ok(view! {
        table(
            caption: caption,
            columns: &COLUMNS,
            empty: empty,
            layout: TableLayout::Stacked,
            for entry in entries {
                table_row(
                    table_row_heading(column: action, (entry.action.label()))
                    table_cell(column: when, (format_date_time(entry.created_at)))
                    table_cell(
                        column: admin,
                        title: Some(admin_of(entry)),
                        (admin_of(entry))
                    )
                    table_cell(
                        column: user,
                        title: Some(entry.target_email.as_str()),
                        log_target(entry: entry)
                    )
                )
            }
        )
    })
}

fn admin_of(entry: &LogEntry) -> &str {
    entry.admin_email.as_deref().unwrap_or(SERVER_CONSOLE)
}

#[component]
async fn log_target(cx: &Cx, entry: &LogEntry) -> Result<impl View> {
    let link = entry
        .target_exists
        .then(|| href!(admin_user_page, id: entry.target_public_id.as_str()).resolve(cx));

    Ok(view! {
        if let Some(link) = link {
            <a href=(link)>(entry.target_email.as_str())</a>
        } else {
            (entry.target_email.as_str())
        }
    })
}
