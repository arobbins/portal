use std::time::Duration;

use leptos::prelude::*;

use super::notice::{Notice, NoticeKind};

#[component]
pub fn Toaster(
    #[prop(optional)] kind: NoticeKind,
    #[prop(optional)] event: Option<&'static str>,
    #[prop(optional)] duration_ms: Option<u64>,
    children: Children,
) -> impl IntoView {
    view! {
        <ToasterIsland
            kind=kind
            event=event.unwrap_or("toast").to_string()
            duration_ms=duration_ms.unwrap_or(4000)
        >
            {children()}
        </ToasterIsland>
    }
}

#[island]
fn ToasterIsland(kind: NoticeKind, event: String, duration_ms: u64, children: Children) -> impl IntoView {
    let is_open = RwSignal::new(false);
    let timer = StoredValue::new(None::<TimeoutHandle>);

    let show = move || {
        is_open.set(true);
        if let Some(handle) = timer.get_value() {
            handle.clear();
        }
        let handle = set_timeout_with_handle(
            move || is_open.set(false),
            Duration::from_millis(duration_ms),
        )
        .ok();
        timer.set_value(handle);
    };

    let listener = window_event_listener_untyped(&event, move |_| show());
    on_cleanup(move || listener.remove());

    view! {
        <div
            class=move || if is_open.get() { "a-toaster a-toaster--is-open" } else { "a-toaster" }
            on:click=move |_| is_open.set(false)
        >
            <Notice kind=kind>{children()}</Notice>
        </div>
    }
}

pub fn dispatch_toast(event: &str) {
    #[cfg(feature = "hydrate")]
    if let Ok(event) = web_sys::Event::new(event) {
        let _ = window().dispatch_event(&event);
    }

    #[cfg(not(feature = "hydrate"))]
    let _ = event;
}
