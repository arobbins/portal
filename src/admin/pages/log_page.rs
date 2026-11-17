use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page},
    view::{View, component, view},
};

use super::admin_access::require_admin;
use super::user_page::{admin_user_page, id};
use super::users_page::admin_users_page;
use crate::admin::actions::{LOG_ENTRIES_SHOWN, list_admin_log};
use crate::admin::queries::LogEntry;
use crate::app::page_title;
use crate::components::empty_state::empty_state;
use crate::components::table::{table, table_cell, table_heading, table_row};
use crate::layout::page_heading::page_heading;
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
    let back = href!(admin_users_page).resolve(cx);
    let caption = format!("The last {LOG_ENTRIES_SHOWN} admin actions, newest first");

    Ok(view! {
        <section class="l-contain p-admin">
            <a class="p-admin__back" href=(back)>"All users"</a>
            page_heading(text: "Admin log")
            if entries.is_empty() {
                empty_state(heading: "No admin actions yet.")
            } else {
                <div class="p-admin__log">
                    log_table(entries: &entries, caption: &caption)
                </div>
            }
        </section>
    })
}

#[component]
async fn log_table(cx: &Cx, entries: &[LogEntry], caption: &str) -> Result<impl View> {
    Ok(view! {
        table(
            caption: Some(caption),
            <thead>
                table_row(
                    table_heading("When")
                    table_heading("Admin")
                    table_heading("Action")
                    table_heading("User")
                )
            </thead>
            <tbody>
                for entry in entries {
                    table_row(
                        table_cell((format_date_time(entry.created_at)))
                        table_cell((admin_of(entry)))
                        table_cell(
                            (entry.action.label())
                            <span class="p-admin__detail">
                                (format!(
                                    "{} · {}",
                                    format_date_time(entry.created_at),
                                    admin_of(entry),
                                ))
                            </span>
                        )
                        table_cell(log_target(entry: entry))
                    )
                }
            </tbody>
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
