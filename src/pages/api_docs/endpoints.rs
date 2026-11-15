use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::method_tone;
use crate::api::catalog::{Endpoint, ErrorDoc, FieldDoc};
use crate::components::badge::badge;
use crate::components::code_block::{code_block, code_panel};
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;
use crate::syntax::CodeLanguage;

#[component]
pub(super) async fn endpoint_section(
    cx: &Cx,
    endpoint: &Endpoint,
    origin: &str,
) -> Result<impl View> {
    let request_fields = endpoint.request_fields();
    let response_fields = endpoint.response_fields();
    let curl = endpoint.curl(origin);
    let response = endpoint.example_response_text();
    let request_title = format!("{} {}", endpoint.method.as_str(), endpoint.path);
    let heading_id = format!("{}-heading", endpoint.id);

    Ok(view! {
        <section
            class="l-docs__section"
            id=(endpoint.id)
            aria-labelledby=(heading_id.as_str())
        >
            <div class="l-docs__prose">
                <h2 class="p-api-docs__heading" id=(heading_id.as_str())>
                    (endpoint.summary)
                </h2>
                <p class="p-api-docs__route">
                    badge(
                        tone: method_tone(endpoint.method),
                        (endpoint.method.as_str())
                    )
                    <span class="p-api-docs__route-path">(endpoint.path)</span>
                </p>
                <p>(endpoint.description)</p>
                <p class="p-api-docs__scope">
                    "Requires the "
                    <code>(endpoint.scope.as_str())</code>
                    " scope."
                </p>
                <h3 class="p-api-docs__subheading">"Request body"</h3>
                if request_fields.is_empty() {
                    <p class="p-api-docs__none">"None."</p>
                } else {
                    field_list(fields: &request_fields)
                }
                <h3 class="p-api-docs__subheading">"Response"</h3>
                field_list(fields: &response_fields)
                <h3 class="p-api-docs__subheading">"Errors"</h3>
                error_list(errors: &endpoint.errors)
            </div>
            <div class="l-docs__aside">
                code_panel(
                    title: &request_title,
                    label: Some("cURL"),
                    code_title: true,
                    code_block(code: &curl, language: CodeLanguage::Shell)
                )
                code_panel(
                    title: "Response",
                    label: Some("JSON"),
                    code_block(code: &response, language: CodeLanguage::Json)
                )
            </div>
        </section>
    })
}

fn field_tags(field: &FieldDoc) -> Vec<PropertyTag> {
    let mut tags = vec![PropertyTag::new(field.kind.as_str(), Tone::Neutral)];
    if field.required {
        tags.push(PropertyTag::new("required", Tone::Error));
    }
    if !field.rules.is_empty() {
        tags.push(PropertyTag::new(field.rules.as_str(), Tone::Neutral));
    }
    tags
}

#[component]
async fn field_list(cx: &Cx, fields: &[FieldDoc]) -> Result<impl View> {
    let rows: Vec<(&FieldDoc, Vec<PropertyTag>)> = fields
        .iter()
        .map(|field| (field, field_tags(field)))
        .collect();

    Ok(view! {
        property_list(
            #[key(field.name.as_str())]
            for (field, tags) in &rows {
                property(
                    name: field.name.as_str(),
                    code: true,
                    tags: tags,
                    description: Some(field.description.as_str())
                )
            }
        )
    })
}

#[component]
pub(super) async fn error_list(cx: &Cx, errors: &[ErrorDoc]) -> Result<impl View> {
    let rows: Vec<(&ErrorDoc, Vec<PropertyTag>)> = errors
        .iter()
        .map(|error| {
            (
                error,
                vec![PropertyTag::new(error.status.to_string(), Tone::Neutral)],
            )
        })
        .collect();

    Ok(view! {
        property_list(
            #[key(error.code)]
            for (error, tags) in &rows {
                property(
                    name: error.code,
                    code: true,
                    tags: tags,
                    description: Some(error.message)
                )
            }
        )
    })
}
