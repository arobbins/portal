use topcoat::{
    Result,
    context::Cx,
    router::request,
    view::{View, component, view},
};

#[component]
pub async fn not_found_page(cx: &Cx) -> Result<impl View> {
    let path = request::uri(cx).path().to_string();

    Ok(view! {
        <h1>"404"</h1>
        <p>
            "You requested "
            (path)
            " but we don't have that page!!"
        </p>
    })
}
