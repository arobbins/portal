use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page, parse_query_params},
    view::{View, attributes, component, view},
};

use super::admin_access::require_admin;
use super::log_page::admin_log_page;
use super::user_page::{admin_user_page, id};
use super::user_view::UserRow;
use crate::admin::actions::{FindUsers, UserPage, list_users};
use crate::app::page_title;
use crate::components::badge::badge;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::empty_state::empty_state;
use crate::components::form::{form, form_actions, form_field};
use crate::components::table::{table, table_cell, table_heading, table_row};
use crate::flash::{Flash, flash_toast, flash_toasts, take_flash};
use crate::layout::page_heading::page_heading;
use crate::web::env;

const FLASHES: &[Flash] = &[Flash::UserDeleted];

#[page("/admin/users", with(Metadata::new().title(page_title("Users"))))]
pub async fn admin_users_page(cx: &Cx) -> Result<impl View> {
    let caller = require_admin(cx).await?;
    let find: FindUsers = parse_query_params(cx).unwrap_or_default();
    let page = match list_users(&env(cx), &caller, &find).await? {
        Ok(page) => page,
        Err(error) => match error.for_browser()? {},
    };
    let rows: Vec<UserRow> = page.users.iter().map(UserRow::of).collect();
    let toast = flash_toast(cx, take_flash(cx, FLASHES).await?);

    Ok(view! {
        <section class="l-contain p-admin">
            page_heading(text: "Users")
            <a class="p-admin__back" href=(href!(admin_log_page).resolve(cx))>
                "Admin log"
            </a>
            users_search(query: &find.q)
            if rows.is_empty() {
                empty_state(heading: "No users match that search.")
            } else {
                <div class="p-admin__users">users_table(rows: &rows)</div>
            }
            pager(page: &page, query: &find.q)
            flash_toasts(toast: &toast, flashes: FLASHES)
        </section>
    })
}

#[component]
async fn users_search(cx: &Cx, query: &str) -> Result<impl View> {
    let action = href!(admin_users_page).resolve(cx);

    Ok(view! {
        form(
            attrs: attributes! {
                method="get"
                action=(action)
                role="search"
                class="p-admin__search"
            },
            form_field(
                label: "Search by email or name",
                id: "admin-user-search",
                name: "q",
                value: Some(query)
            )
            form_actions(
                button(
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Submit,
                    "Search"
                )
            )
        )
    })
}

#[component]
async fn users_table(cx: &Cx, rows: &[UserRow]) -> Result<impl View> {
    Ok(view! {
        table(
            caption: Some("Users, newest first"),
            <thead>
                table_row(
                    table_heading("Name")
                    table_heading("Email")
                    table_heading("Status")
                    table_heading("Role")
                    table_heading("Joined")
                    table_heading("Last active")
                )
            </thead>
            <tbody>
                #[key(row.id.as_str())]
                for row in rows {
                    table_row(
                        table_cell(
                            <a
                                href=(href!(admin_user_page, id: row.id.as_str()).resolve(
                                    cx,
                                ))
                            >
                                (row.name.as_str())
                            </a>
                            <span class="p-admin__detail">(row.email.as_str())</span>
                        )
                        table_cell((row.email.as_str()))
                        table_cell(badge(tone: row.status.1, (row.status.0)))
                        table_cell((row.role))
                        table_cell((row.joined.as_str()))
                        table_cell((row.last_active.as_str()))
                    )
                }
            </tbody>
        )
    })
}

#[component]
async fn pager(cx: &Cx, page: &UserPage, query: &str) -> Result<impl View> {
    let base = href!(admin_users_page).resolve(cx);
    let link = |number: u32| {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("q", query)
            .append_pair("page", &number.to_string())
            .finish();
        format!("{base}?{query}")
    };
    let newer = (page.page > 0).then(|| link(page.page - 1));
    let older = page.has_next.then(|| link(page.page + 1));

    Ok(view! {
        <nav class="p-admin__pager" aria-label="Pages">
            if let Some(newer) = newer {
                button(variant: ButtonVariant::Secondary, href: Some(&newer), "Newer")
            }
            if let Some(older) = older {
                button(variant: ButtonVariant::Secondary, href: Some(&older), "Older")
            }
        </nav>
    })
}
