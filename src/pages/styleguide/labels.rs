use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::form::form_input;
use crate::components::label::{LabelSize, label};

#[component]
pub async fn labels_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Labels",
            label(size: LabelSize::Sm, "Small: above lists, demos and sidebar groups")
            label(size: LabelSize::Md, "Medium: above form fields and settings")
            <div>
                label(size: LabelSize::Sm, "Above a list")
                <ul>
                    <li>"Small labels name a group of items"</li>
                    <li>"without adding a heading to the page outline"</li>
                </ul>
            </div>
            <div>
                label(for_id: Some("styleguide-label-demo"), "Medium, tied to a field")
                form_input(id: "styleguide-label-demo", name: "label_demo")
            </div>
        )
    })
}
