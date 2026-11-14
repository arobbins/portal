use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::icon::{IconName, icon};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AvatarSize {
    #[default]
    Sm,
    Md,
    Lg,
}

impl AvatarSize {
    fn class(self) -> &'static str {
        match self {
            Self::Sm => "a-avatar a-avatar--sm",
            Self::Md => "a-avatar a-avatar--md",
            Self::Lg => "a-avatar a-avatar--lg",
        }
    }
}

#[component]
pub async fn avatar(cx: &Cx, #[default] size: AvatarSize) -> Result<impl View> {
    Ok(view! { <span class=(size.class())>icon(name: IconName::User)</span> })
}
