use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::badge::badge;
use crate::components::pager::pager;
use crate::components::table::{
    Column, TableDensity, TableLayout, table, table_cell, table_row, table_row_heading,
};
use crate::components::tone::Tone;
use crate::listing::{SortDirection, TableSorting};

const INVOICE_COLUMNS: [Column; 4] = [
    Column::new("Date").sortable("date").date(),
    Column::new("Plan").grow(),
    Column::new("Status").wide_only(),
    Column::new("Amount").sortable("amount").number(),
];

const INVOICES: [(&str, &str, &str, &str); 3] = [
    ("#invoice-2026-11-01", "Nov 1, 2026", "Pro plan", "$29.00"),
    ("#invoice-2026-10-01", "Oct 1, 2026", "Pro plan", "$29.00"),
    (
        "#invoice-2026-09-01",
        "Sep 1, 2026",
        "Starter plan",
        "$9.00",
    ),
];

const EVENT_COLUMNS: [Column; 3] = [
    Column::new("When").date(),
    Column::new("Event").truncated(),
    Column::new("Count").number(),
];

const REFUND_COLUMNS: [Column; 2] = [Column::new("Date"), Column::new("Amount").number()];

#[component]
pub async fn tables_section(cx: &Cx) -> Result<impl View> {
    let sorting = TableSorting {
        key: "date",
        direction: SortDirection::Descending,
        links: vec![
            ("date", "#tables".to_string()),
            ("amount", "#tables".to_string()),
        ],
    };
    let [date, plan, status, amount] = INVOICE_COLUMNS;

    Ok(view! {
        styleguide_section(
            title: "Tables",
            table(
                caption: "Invoices",
                columns: &INVOICE_COLUMNS,
                sorting: Some(&sorting),
                layout: TableLayout::Stacked,
                #[key(day)]
                for (link, day, name, total) in INVOICES {
                    table_row(
                        table_row_heading(column: date, href: Some(link), (day))
                        table_cell(column: plan, (name))
                        table_cell(column: status, badge(tone: Tone::Success, "Paid"))
                        table_cell(column: amount, (total))
                    )
                }
            )
            pager(previous: Some("#tables"), next: Some("#tables"))
            events_table()
            table(
                caption: "Refunds",
                columns: &REFUND_COLUMNS,
                empty: Some("No refunds yet.")
            )
        )
    })
}

#[component]
async fn events_table(cx: &Cx) -> Result<impl View> {
    let [when, event, count] = EVENT_COLUMNS;
    let rows: Vec<(String, String, String)> = (1..=24)
        .map(|hour| {
            (
                format!("Nov 17, {hour:02}:00 UTC"),
                format!("Nightly import of customer records from the warehouse, batch {hour}"),
                (hour * 137).to_string(),
            )
        })
        .collect();

    Ok(view! {
        table(
            caption: "Recent events",
            columns: &EVENT_COLUMNS,
            density: TableDensity::Compact,
            sticky_header: true,
            #[key(time.as_str())]
            for (time, text, total) in &rows {
                table_row(
                    table_row_heading(column: when, (time.as_str()))
                    table_cell(
                        column: event,
                        title: Some(text.as_str()),
                        (text.as_str())
                    )
                    table_cell(column: count, (total.as_str()))
                )
            }
        )
    })
}
