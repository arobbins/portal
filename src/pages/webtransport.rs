use leptos::prelude::*;

use crate::components::Button;
use crate::layout::PageHeading;

#[component]
pub fn WebTransportPage() -> impl IntoView {
    view! {
        <section class="l-contain l-skinny">
            <PageHeading text="WebTransport"/>
            <p>"Connect to the Rust-powered WebTransport server below."</p>
            <ConnectButton/>
        </section>
    }
}

#[island]
fn ConnectButton() -> impl IntoView {
    let is_loading = RwSignal::new(false);

    view! {
        <Button busy=is_loading disabled=is_loading on:click=move |_| is_loading.set(true)>
            {move || if is_loading.get() { "Connecting ..." } else { "Connect" }}
        </Button>
    }
}
