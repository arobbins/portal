use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::theme_switcher::theme_switcher;

#[component]
pub async fn theme_switcher_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Theme switcher",
            <div class="styleguide__row">
                theme_switcher()
                theme_switcher(compact: true)
            </div>
        )
    })
}
