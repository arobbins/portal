use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use super::styleguide_section;
use crate::components::button::button;
use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::tone::Tone;

const DEMO_TOAST: &str = "demo";

#[component]
pub async fn toaster_section(cx: &Cx) -> Result<impl View> {
    let toast = signal(cx, || NO_TOAST.to_string());

    Ok(view! {
        styleguide_section(
            title: "Toaster",
            <div class="p-styleguide__row">
                button(
                    attrs: attributes! { @click=$(|_e: Event| toast.set(DEMO_TOAST.to_owned())) },
                    "Show toast"
                )
            </div>
            toaster(
                toast: &toast,
                name: DEMO_TOAST,
                tone: Tone::Success,
                "Success! This is a toaster notice."
            )
        )
    })
}
