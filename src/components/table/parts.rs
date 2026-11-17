use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, ViewExt, component, view},
};

use super::column::{Column, TableDensity, TableLayout};
use crate::components::empty_state::empty_state;
use crate::components::icon::{IconName, icon};
use crate::listing::{SortDirection, TableSorting};

#[component]
pub async fn table(
    cx: &Cx,
    caption: &str,
    columns: &[Column],
    #[default] caption_hidden: bool,
    #[default] sorting: Option<&TableSorting>,
    #[default] empty: Option<&str>,
    #[default] density: TableDensity,
    #[default] layout: TableLayout,
    #[default] sticky_header: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let mut class = format!("a-table {} {}", density.modifier(), layout.modifier());
    if sticky_header {
        class.push_str(" a-table--sticky");
    }
    let caption_class = if caption_hidden {
        "a-table__caption u-visually-hidden"
    } else {
        "a-table__caption"
    };
    let head_class = if columns.iter().any(|column| column.sort_key.is_some()) {
        "a-table__head"
    } else {
        "a-table__head a-table__head--static"
    };
    let span = columns.len().to_string();
    debug_assert!(
        sorting.is_none_or(|sorting| columns
            .iter()
            .filter_map(|column| column.sort_key)
            .all(|key| sorting.link(key).is_some())),
        "every sortable column needs a sort link"
    );

    Ok(view! {
        <div class=(class) role="region" aria-label=(caption) tabindex="0">
            <table class="a-table__table" role="table">
                <caption class=(caption_class)>(caption)</caption>
                <thead class=(head_class) role="rowgroup">
                    <tr class="a-table__row a-table__row--head" role="row">
                        for column in columns {
                            column_heading(column: *column, sorting: sorting)
                        }
                    </tr>
                </thead>
                <tbody class="a-table__body" role="rowgroup">
                    if let Some(empty) = empty {
                        <tr class="a-table__row" role="row">
                            <td
                                role="cell"
                                class="a-table__cell a-table__cell--empty"
                                colspan=(span.as_str())
                            >
                                empty_state(heading: empty)
                            </td>
                        </tr>
                    } else {
                        (child)
                    }
                </tbody>
            </table>
        </div>
    })
}

#[component]
async fn column_heading(
    cx: &Cx,
    column: Column,
    sorting: Option<&TableSorting>,
) -> Result<impl View> {
    let link = column
        .sort_key
        .and_then(|key| sorting.and_then(|sorting| sorting.link(key)));
    let sorted = column
        .sort_key
        .and_then(|key| sorting.and_then(|sorting| sorting.direction_of(key)));
    let mut class = column.classes("a-table__heading");
    if link.is_none() {
        class.push_str(" a-table__heading--static");
    }

    let Some(link) = link else {
        return Ok(view! {
            <th class=(class) scope="col" role="columnheader">(column.label)</th>
        }
        .boxed());
    };
    let link_class = match sorted {
        Some(SortDirection::Ascending) => "a-table__sort a-table__sort--ascending",
        Some(SortDirection::Descending) => "a-table__sort a-table__sort--descending",
        None => "a-table__sort",
    };

    Ok(view! {
        <th
            class=(class)
            scope="col"
            role="columnheader"
            aria-sort=(sorted.map(SortDirection::aria))
        >
            <a class=(link_class) href=(link)>
                (column.label)
                if sorted.is_some() {
                    <span class="a-table__sort-icon" aria-hidden="true">
                        icon(name: IconName::ChevronDown)
                    </span>
                }
            </a>
        </th>
    }
    .boxed())
}

#[component]
pub async fn table_row(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <tr class="a-table__row" role="row">(child)</tr> })
}

#[component]
pub async fn table_row_heading(
    cx: &Cx,
    column: Column,
    #[default] href: Option<&str>,
    #[default] title: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let mut class = format!("{} a-table__cell--heading", column.classes("a-table__cell"));
    if href.is_some() {
        class.push_str(" a-table__cell--link");
    }

    Ok(view! {
        <th
            class=(class)
            scope="row"
            role="rowheader"
            title=(title)
            data-label=(column.label)
        >
            if let Some(href) = href {
                <a class="a-table__cell-link" href=(href)>(child)</a>
            } else {
                (child)
            }
        </th>
    })
}

#[component]
pub async fn table_cell(
    cx: &Cx,
    column: Column,
    #[default] title: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = column.classes("a-table__cell");

    Ok(view! {
        <td class=(class) role="cell" title=(title) data-label=(column.label)>
            (child)
        </td>
    })
}
