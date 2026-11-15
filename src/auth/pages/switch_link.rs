use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

#[component]
pub(super) async fn switch_link(
    cx: &Cx,
    prompt: &str,
    href: &str,
    label: &str,
) -> Result<impl View> {
    Ok(view! {
        <p class="p-auth__switch">
            (prompt)
            <a class="p-auth__switch-link" href=(href)>(label)</a>
        </p>
    })
}
