use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

use super::icon::{IconName, icon};

#[component]
pub async fn empty_state(
    cx: &Cx,
    heading: &str,
    #[default] icon_name: Option<IconName>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class="a-empty-state">
            if let Some(name) = icon_name {
                <span class="a-empty-state__icon">icon(name: name)</span>
            }
            <p class="a-empty-state__heading">(heading)</p>
            <div class="a-empty-state__body">(child)</div>
        </div>
    })
}
