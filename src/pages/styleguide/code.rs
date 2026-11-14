use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;
use crate::components::code_block::{code_block, code_panel};
use crate::components::label::{LabelSize, label};
use crate::syntax::CodeLanguage;

const REQUEST: &str =
    "curl https://example.com/api/v1/me \\\n  -H \"Authorization: Bearer $WSK_TOKEN\"";
const RESPONSE: &str = "{\n  \"name\": \"Ada Lovelace\",\n  \"email\": \"ada@example.com\",\n  \"admin\": false,\n  \"tokens\": 2\n}";

#[component]
pub async fn code_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Code",
            label(size: LabelSize::Sm, "Shell")
            code_panel(
                title: "GET /api/v1/me",
                label: Some("cURL"),
                code_title: true,
                code_block(code: REQUEST, language: CodeLanguage::Shell)
            )
            label(size: LabelSize::Sm, "Plain text")
            code_panel(
                title: "Notes",
                code_block(code: "Plain text has no highlighting.")
            )
            label(size: LabelSize::Sm, "JSON with line numbers")
            code_panel(
                title: "Response",
                label: Some("JSON"),
                code_block(
                    code: RESPONSE,
                    language: CodeLanguage::Json,
                    line_numbers: true
                )
            )
        )
    })
}
