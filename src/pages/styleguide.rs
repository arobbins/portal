mod buttons;
mod forms;
mod modal;
mod notices;
mod text;
mod toaster;

use leptos::prelude::*;

#[component]
pub fn StyleguidePage() -> impl IntoView {
    view! {
        <section class="l-contain l-skinny styleguide">
            <h1>"Styleguide (h1)"</h1>
            <text::TextSection/>
            <buttons::ButtonsSection/>
            <notices::NoticesSection/>
            <toaster::ToasterSection/>
            <forms::FormsSection/>
            <modal::ModalSection/>
        </section>
    }
}

#[component]
fn StyleguideSection(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <section class="styleguide__section">
            <h2>{title}</h2>
            <hr/>
            <div class="styleguide__examples">{children()}</div>
        </section>
    }
}
