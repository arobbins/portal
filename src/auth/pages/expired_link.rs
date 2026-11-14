use topcoat::{
    Result,
    context::Cx,
    router::StatusCode,
    view::{Child, View, component, view},
};

use crate::layout::page_heading::page_heading;

#[component]
pub async fn expired_link(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        (StatusCode::BAD_REQUEST)
        <section class="l-contain l-skinny">
            page_heading(text: "This link has expired")
            (child)
        </section>
    })
}
