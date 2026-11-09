use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

use crate::components::notice::{NoticeKind, notice};
use crate::layout::page_heading::page_heading;

#[page("/webtransport")]
pub async fn webtransport_page() -> Result<impl View> {
    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "WebTransport")
            notice(
                kind: NoticeKind::Info,
                "The WebTransport demo isn't connected to a server yet."
            )
        </section>
    })
}
