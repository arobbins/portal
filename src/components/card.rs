use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

#[component]
pub async fn card(
    cx: &Cx,
    #[default] id: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <section class="a-card" id=(id)>(child)</section> })
}

#[component]
pub async fn card_header(cx: &Cx, title: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <header class="a-card__header">
            <h3 class="a-card__title">(title)</h3>
            (child)
        </header>
    })
}

#[component]
pub async fn card_body(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class="a-card__body">(child)</div> })
}
