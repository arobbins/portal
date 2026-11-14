use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::avatar::{AvatarSize, avatar};
use crate::components::badge::badge;
use crate::components::tone::Tone;

#[component]
pub async fn badges_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Badges and avatars",
            <div class="p-styleguide__row">
                for tone in Tone::ALL {
                    badge(tone: tone, (tone.label()))
                }
            </div>
            <div class="p-styleguide__row">
                avatar(size: AvatarSize::Sm)
                avatar(size: AvatarSize::Md)
                avatar(size: AvatarSize::Lg)
            </div>
        )
    })
}
