use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::avatar::{AvatarSize, avatar};

#[component]
pub async fn avatars_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Avatars",
            <div class="p-styleguide__row">
                avatar(size: AvatarSize::Sm)
                avatar(size: AvatarSize::Md)
                avatar(size: AvatarSize::Lg)
            </div>
        )
    })
}
