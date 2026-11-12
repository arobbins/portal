mod badges;
mod buttons;
mod cards;
mod forms;
mod icons;
mod modal;
mod navigation;
mod notices;
mod text;
mod theme_switcher;
mod toaster;
mod toggles;

use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    view::{Child, View, component, view},
};

use crate::app::page_title;

#[page("/styleguide", with(Metadata::new().title(page_title("Styleguide"))))]
pub async fn styleguide_page(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <section class="l-contain l-skinny styleguide">
            <h1>"Styleguide"</h1>
            text::text_section()
            buttons::buttons_section()
            badges::badges_section()
            notices::notices_section()
            toaster::toaster_section()
            forms::forms_section()
            toggles::toggles_section()
            cards::cards_section()
            navigation::navigation_section()
            modal::modal_section()
            icons::icons_section()
            theme_switcher::theme_switcher_section()
        </section>
    })
}

#[component]
async fn styleguide_section(
    cx: &Cx,
    title: &str,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <section class="styleguide__section">
            <h2>(title)</h2>
            <hr>
            <div class="styleguide__examples">(child)</div>
        </section>
    })
}
