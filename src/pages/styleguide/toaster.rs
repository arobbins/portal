use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{View, attributes, component, view},
};

use super::styleguide_section;
use crate::components::button::button;
use crate::components::notice::NoticeKind;
use crate::components::toaster::toaster;

#[component]
pub async fn toaster_section(cx: &Cx) -> Result<impl View> {
    let is_open = signal(cx, || false);

    Ok(view! {
        styleguide_section(
            title: "Toaster",
            button(
                attrs: attributes! { @click=$(|_e: Event| is_open.set(true)) },
                "Show toast"
            )
            toaster(
                open: &is_open,
                kind: NoticeKind::Success,
                "Success! This is a toaster notice."
            )
        )
    })
}
