use topcoat::{
    Result,
    view::{View, component, view},
};

use super::{logo::logo, nav::nav};
use crate::components::theme_switcher::theme_switcher;

#[component]
pub async fn header() -> Result<impl View> {
    Ok(view! {
        <header class="l-contain l-flex header">
            <div class="header__logo l-flex l-flex-center-col l-flex-1">
                logo()
            </div>
            <div class="header__nav">
                nav()
            </div>
            <div class="header__theme l-flex l-flex-center-col">
                theme_switcher(compact: true)
            </div>
        </header>
    })
}
