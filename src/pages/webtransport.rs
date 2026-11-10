use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    view::{View, view},
};

use crate::app::page_title;
use crate::components::notice::{NoticeKind, notice};
use crate::layout::page_heading::page_heading;

#[page("/webtransport", with(Metadata::new().title(page_title("WebTransport"))))]
pub async fn webtransport_page(cx: &Cx) -> Result<impl View> {
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
