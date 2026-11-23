use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};
use tower_sessions::Session;

use super::switch_link::switch_link;
use crate::components::notice::notice;
use crate::components::tone::Tone;
use crate::web::session;

const LINK_SENT_TO: &str = "link_sent_to";

pub(super) async fn remember_link_sent_to(session: &Session, email: &str) -> Result<()> {
    session
        .insert(LINK_SENT_TO, email.trim().to_string())
        .await?;
    Ok(())
}

pub(super) async fn take_link_sent_to(cx: &Cx) -> Result<Option<String>> {
    Ok(session(cx)?.remove::<String>(LINK_SENT_TO).await?)
}

#[component]
pub(super) async fn link_sent_notice(
    cx: &Cx,
    before: &str,
    email: &str,
    after: &str,
    again_href: &str,
) -> Result<impl View> {
    Ok(view! {
        <div role="status" class="p-auth__sent">
            notice(
                tone: Tone::Success,
                <p>
                    <strong>"Check your inbox."</strong>
                    " "
                    (before)
                    <strong>(email)</strong>
                    (after)
                </p>
            )
        </div>
        switch_link(
            prompt: "Wrong address? ",
            href: again_href,
            label: "Use a different email"
        )
    })
}
