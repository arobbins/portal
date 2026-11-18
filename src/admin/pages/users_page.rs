use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page, request},
    view::{View, attributes, component, view},
};

use super::admin_access::require_admin;
use super::admin_shell::{AdminSection, admin_shell};
use super::user_page::{admin_user_page, id};
use super::user_view::UserRow;
use crate::admin::actions::{FindUsers, UserSort, list_users};
use crate::app::page_title;
use crate::components::badge::badge;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{form, form_actions, form_field};
use crate::components::pager::pager;
use crate::components::table::{
    Column, TableLayout, table, table_cell, table_row, table_row_heading,
};
use crate::flash::{Flash, flash_toast, flash_toasts, take_flash};
use crate::listing::{ListQuery, SortKey};
use crate::web::env;

const FLASHES: &[Flash] = &[Flash::UserDeleted];

fn columns() -> [Column; 6] {
    [
        Column::new("Name").sortable(UserSort::Name.as_str()),
        Column::new("Email")
            .sortable(UserSort::Email.as_str())
            .truncated(),
        Column::new("Status").sortable(UserSort::Status.as_str()),
        Column::new("Role")
            .sortable(UserSort::Role.as_str())
            .wide_only(),
        Column::new("Joined")
            .sortable(UserSort::Joined.as_str())
            .date()
            .wide_only(),
        Column::new("Last active")
            .sortable(UserSort::LastActive.as_str())
            .date(),
    ]
}

#[page("/admin/users", with(Metadata::new().title(page_title("Users"))))]
pub async fn admin_users_page(cx: &Cx) -> Result<impl View> {
    let caller = require_admin(cx).await?;
    let list: ListQuery<UserSort> = ListQuery::parse(request::uri(cx).query());
    let page = match list_users(&env(cx), &caller, &FindUsers::from(list.clone())).await? {
        Ok(page) => page,
        Err(error) => match error.for_browser()? {},
    };
    let zone = caller.user.zone();
    let rows: Vec<UserRow> = page
        .users
        .iter()
        .map(|user| UserRow::of(user, &zone))
        .collect();
    let base = href!(admin_users_page).resolve(cx);
    let previous = (page.page > 0).then(|| list.page_href(&base, page.page - 1));
    let next = page.has_next.then(|| list.page_href(&base, page.page + 1));
    let toast = flash_toast(cx, take_flash(cx, FLASHES).await?);

    Ok(view! {
        admin_shell(
            user: caller.user.as_ref(),
            section: AdminSection::Users,
            users_search(list: &list)
            users_table(rows: &rows, list: &list)
            pager(previous: previous.as_deref(), next: next.as_deref())
            flash_toasts(toast: &toast, flashes: FLASHES)
        )
    })
}

#[component]
async fn users_search(cx: &Cx, list: &ListQuery<UserSort>) -> Result<impl View> {
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
                value: Some(list.q.as_str())
            )
            <input type="hidden" name="sort" value=(list.sort.as_str())>
            <input type="hidden" name="dir" value=(list.direction.as_str())>
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
async fn users_table(cx: &Cx, rows: &[UserRow], list: &ListQuery<UserSort>) -> Result<impl View> {
    let columns = columns();
    let [name, email, status, role, joined, last_active] = columns;
    let sorting = list.sorting(&href!(admin_users_page).resolve(cx));
    let empty = rows.is_empty().then_some("No users match that search.");
    let rows: Vec<(&UserRow, String)> = rows
        .iter()
        .map(|row| {
            let link = href!(admin_user_page, id: row.id.as_str()).resolve(cx);
            (row, link)
        })
        .collect();

    Ok(view! {
        table(
            caption: "Users",
            caption_hidden: true,
            columns: &columns,
            sorting: Some(&sorting),
            empty: empty,
            layout: TableLayout::Stacked,
            #[key(row.id.as_str())]
            for (row, link) in &rows {
                table_row(
                    table_row_heading(
                        column: name,
                        href: Some(link.as_str()),
                        (row.name.as_str())
                    )
                    table_cell(
                        column: email,
                        title: Some(row.email.as_str()),
                        (row.email.as_str())
                    )
                    table_cell(
                        column: status,
                        badge(tone: row.status.1, (row.status.0))
                    )
                    table_cell(column: role, (row.role))
                    table_cell(column: joined, (row.joined.as_str()))
                    table_cell(column: last_active, (row.last_active.as_str()))
                )
            }
        )
    })
}
