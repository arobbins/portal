use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use super::styleguide_section;
use crate::components::button::button;
use crate::components::notice::NoticeKind;
use crate::components::toaster::{NO_TOAST, toaster};

const DEMO_TOAST: &str = "demo";

#[component]
pub async fn toaster_section(cx: &Cx) -> Result<impl View> {
    let toast = signal(cx, || NO_TOAST.to_string());

    Ok(view! {
        styleguide_section(
            title: "Toaster",
            button(
                attrs: attributes! { @click=$(|_e: Event| toast.set(DEMO_TOAST.to_owned())) },
                "Show toast"
            )
            toaster(
                toast: &toast,
                name: DEMO_TOAST,
                kind: NoticeKind::Success,
                "Success! This is a toaster notice."
            )
        )
    })
}
