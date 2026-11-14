use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

use super::badge::badge;
use super::tone::Tone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyTag {
    pub label: String,
    pub tone: Tone,
}

impl PropertyTag {
    pub fn new(label: impl Into<String>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
        }
    }
}

#[component]
pub async fn property_list(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <ul class="a-property-list">(child)</ul> })
}

#[component]
pub async fn property(
    cx: &Cx,
    name: &str,
    #[default] code: bool,
    #[default] tags: &[PropertyTag],
    #[default] description: Option<&str>,
    #[default] detail: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let name_class = if code {
        "a-property-list__name a-property-list__name--code"
    } else {
        "a-property-list__name"
    };

    Ok(view! {
        <li class="a-property-list__item">
            <div class="a-property-list__info">
                <p class="a-property-list__head">
                    <span class=(name_class)>(name)</span>
                    for tag in tags {
                        badge(tone: tag.tone, (tag.label.as_str()))
                    }
                </p>
                if let Some(description) = description {
                    <p class="a-property-list__description">(description)</p>
                }
                if let Some(detail) = detail {
                    <p class="a-property-list__detail">(detail)</p>
                }
            </div>
            <div class="a-property-list__actions">(child)</div>
        </li>
    })
}
