use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, View, component, view},
};

#[component]
pub async fn toggle(
    cx: &Cx,
    label: &str,
    id: &str,
    name: &str,
    #[default] description: Option<&str>,
    #[default] checked: bool,
    #[default] disabled: bool,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <label class="a-toggle" for=(id)>
            <span class="a-toggle__text">
                <span class="a-toggle__label">(label)</span>
                if let Some(description) = description {
                    <span class="a-toggle__description">(description)</span>
                }
            </span>
            <input
                class="a-toggle__input"
                type="checkbox"
                role="switch"
                id=(id)
                name=(name)
                checked=(checked)
                disabled=(disabled)
                (attrs)
            >
        </label>
    })
}
