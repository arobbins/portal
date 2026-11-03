use topcoat::{
    Result,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::notice::{NoticeKind, notice};

#[component]
pub async fn notices_section() -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Notices",
            notice(kind: NoticeKind::Success, "Success! This is a notice.")
            notice(kind: NoticeKind::Warning, "Warning! This is a notice.")
            notice(kind: NoticeKind::Error, "Error! This is a notice.")
            notice(kind: NoticeKind::Info, "Info: This is a notice.")
            notice(
                kind: NoticeKind::Info,
                dismissible: true,
                "Info: This is a dismissible notice."
            )
        )
    })
}
