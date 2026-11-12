use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeKind {
    #[default]
    Neutral,
    Success,
    Warning,
    Error,
    Info,
}

impl BadgeKind {
    fn class(self) -> &'static str {
        match self {
            Self::Neutral => "a-badge",
            Self::Success => "a-badge a-badge--success",
            Self::Warning => "a-badge a-badge--warning",
            Self::Error => "a-badge a-badge--error",
            Self::Info => "a-badge a-badge--info",
        }
    }
}

#[component]
pub async fn badge(
    cx: &Cx,
    #[default] kind: BadgeKind,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <span class=(kind.class())>(child)</span> })
}
