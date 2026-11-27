use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

use super::badge::badge;
use super::tone::Tone;

#[component]
pub async fn card(
    cx: &Cx,
    #[default] id: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <section class="a-card" id=(id)>(child)</section> })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CardHeadingLevel {
    #[default]
    H2,
    H3,
}

#[component]
pub async fn card_header(
    cx: &Cx,
    title: &str,
    #[default] level: CardHeadingLevel,
    #[default] status: Option<(&str, Tone)>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <header class="a-card__header">
            <div class="a-card__heading">
                if level == CardHeadingLevel::H2 {
                    <h2 class="a-card__title">(title)</h2>
                } else {
                    <h3 class="a-card__title">(title)</h3>
                }
                if let Some((label, tone)) = status {
                    " "
                    badge(tone: tone, (label))
                }
            </div>
            (child)
        </header>
    })
}

#[component]
pub async fn card_body(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class="a-card__body">(child)</div> })
}
