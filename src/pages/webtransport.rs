use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{Event, signal},
    view::{View, attributes, view},
};

use crate::components::button::button;
use crate::layout::page_heading::page_heading;

#[page("/webtransport")]
pub async fn webtransport_page(cx: &Cx) -> Result<impl View> {
    let is_loading = signal(cx, || false);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "WebTransport")
            <p>"Connect to the Rust-powered WebTransport server below."</p>
            button(
                busy: $(is_loading.get()),
                disabled: $(is_loading.get()),
                attrs: attributes! { @click=$(|_e: Event| is_loading.set(true)) },
                $(if is_loading.get() { "Connecting ..." } else { "Connect" })
            )
        </section>
    })
}
