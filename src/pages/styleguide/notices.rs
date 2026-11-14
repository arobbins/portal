use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::notice::notice;
use crate::components::tone::Tone;

#[component]
pub async fn notices_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Notices",
            notice(tone: Tone::Success, "Success! This is a notice.")
            notice(tone: Tone::Warning, "Warning! This is a notice.")
            notice(tone: Tone::Error, "Error! This is a notice.")
            notice(tone: Tone::Info, "Info: This is a notice.")
            notice("Neutral: This is a notice.")
            notice(
                tone: Tone::Info,
                dismissible: true,
                "Info: This is a dismissible notice."
            )
            notice(
                tone: Tone::Warning,
                "This passkey only exists on one device. Add a second passkey so you can still sign in if you lose it. Longer messages wrap onto several lines, and the icon stays next to the first line."
            )
        )
    })
}
