use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

#[component]
pub async fn table(
    cx: &Cx,
    #[default] caption: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="a-table">
            <table class="a-table__table">
                if let Some(caption) = caption {
                    <caption class="a-table__caption">(caption)</caption>
                }
                (child)
            </table>
        </div>
    })
}

#[component]
pub async fn table_row(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <tr class="a-table__row">(child)</tr> })
}

#[component]
pub async fn table_heading(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <th class="a-table__heading" scope="col">(child)</th> })
}

#[component]
pub async fn table_cell(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <td class="a-table__cell">(child)</td> })
}
