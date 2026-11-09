use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
pub async fn account_placeholder(heading: &str) -> Result<impl View> {
    Ok(view! {
        <div class="account__sub-page-content">
            <h2 class="account__sub-page-heading">(heading)</h2>
            <p>"This section isn't available yet."</p>
        </div>
    })
}
