use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::avatar::{AvatarSize, avatar};
use crate::components::badge::{BadgeKind, badge};

#[component]
pub async fn badges_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Badges and avatars",
            <div class="styleguide__row">
                badge("Neutral")
                badge(kind: BadgeKind::Success, "Active")
                badge(kind: BadgeKind::Warning, "This device only")
                badge(kind: BadgeKind::Error, "Expired")
                badge(kind: BadgeKind::Info, "Beta")
            </div>
            <div class="styleguide__row">
                avatar(size: AvatarSize::Sm)
                avatar(size: AvatarSize::Md)
                avatar(size: AvatarSize::Lg)
            </div>
        )
    })
}
