mod buttons;
mod forms;
mod modal;
mod notices;
mod text;
mod toaster;

use topcoat::{
    Result,
    router::page,
    view::{Child, View, component, view},
};

#[page("/styleguide")]
pub async fn styleguide_page() -> Result<impl View> {
    Ok(view! {
        <section class="l-contain l-skinny styleguide">
            <h1>"Styleguide (h1)"</h1>
            text::text_section()
            buttons::buttons_section()
            notices::notices_section()
            toaster::toaster_section()
            forms::forms_section()
            modal::modal_section()
        </section>
    })
}

#[component]
async fn styleguide_section(title: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <section class="styleguide__section">
            <h2>(title)</h2>
            <hr>
            <div class="styleguide__examples">(child)</div>
        </section>
    })
}
