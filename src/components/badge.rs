use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

use super::tone::Tone;

#[component]
pub async fn badge(
    cx: &Cx,
    #[default] tone: Tone,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = tone.class("a-badge");

    Ok(view! { <span class=(class)>(child)</span> })
}
