use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    view::{View, view},
};

use crate::app::page_title;

#[page("/", with(Metadata::new().title(page_title("Home"))))]
pub async fn home_page(cx: &Cx) -> Result<impl View> {
    Ok(view! { <h1 class="heading-xl">"*******"</h1> })
}
