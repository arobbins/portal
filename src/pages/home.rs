use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

#[page("/")]
pub async fn home_page() -> Result<impl View> {
    Ok(view! { <h1 class="heading-xl">"*******"</h1> })
}
