use topcoat::{
    Result,
    context::Cx,
    runtime::signal,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::segmented::{Segment, segmented};

const MONTHLY: &str = "monthly";

#[component]
pub async fn segmented_section(cx: &Cx) -> Result<impl View> {
    let selected = signal(cx, || MONTHLY.to_string());
    let segments = vec![
        Segment {
            value: MONTHLY.to_string(),
            href: "#styleguide-segmented".to_string(),
            label: "Monthly".to_string(),
            note: None,
        },
        Segment {
            value: "yearly".to_string(),
            href: "#styleguide-segmented-yearly".to_string(),
            label: "Yearly".to_string(),
            note: Some("Save 16%".to_string()),
        },
    ];

    Ok(view! {
        styleguide_section(
            title: "Segmented links",
            <div class="p-styleguide__row" id="styleguide-segmented">
                segmented(
                    label: "Billing period",
                    segments: &segments,
                    selected: &selected
                )
            </div>
        )
    })
}
