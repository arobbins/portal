use schemars::Schema;
use serde_json::{Map, Value, json};
use topcoat::router::{StatusCode, content::Json};

use crate::action::ActionProblem;
use crate::api::auth::ApiAuthProblem;
use crate::api::problem::{ApiFailure, ApiProblem};
use crate::api::scope::ApiScope;
use crate::user::name::NameProblem;

pub const TITLE: &str = "Webserver API";
pub const VERSION: &str = "1";
pub const TOKEN_VARIABLE: &str = "WSK_TOKEN";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Patch,
}

impl HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Patch => "PATCH",
        }
    }

    fn openapi_key(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Patch => "patch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDoc {
    pub status: u16,
    pub code: &'static str,
    pub message: &'static str,
}

impl ErrorDoc {
    pub fn of<P: ActionProblem>(problem: P) -> Self {
        Self {
            status: problem.status().as_u16(),
            code: problem.code(),
            message: problem.message(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDoc {
    pub name: String,
    pub kind: String,
    pub required: bool,
    pub rules: String,
    pub description: String,
}

pub struct Endpoint {
    pub id: &'static str,
    pub group: &'static str,
    pub method: HttpMethod,
    pub path: &'static str,
    pub summary: &'static str,
    pub description: &'static str,
    pub scope: ApiScope,
    pub request: Option<Schema>,
    pub response: Schema,
    pub example_request: Option<Value>,
    pub example_response: Value,
    pub errors: Vec<ErrorDoc>,
}

impl Endpoint {
    pub fn request_fields(&self) -> Vec<FieldDoc> {
        self.request.as_ref().map(fields).unwrap_or_default()
    }

    pub fn response_fields(&self) -> Vec<FieldDoc> {
        fields(&self.response)
    }

    pub fn curl(&self, base_url: &str) -> String {
        let mut command = format!("curl {base_url}{}", self.path);
        if self.method != HttpMethod::Get {
            command.push_str(&format!(" \\\n  -X {}", self.method.as_str()));
        }
        command.push_str(&format!(
            " \\\n  -H \"Authorization: Bearer ${TOKEN_VARIABLE}\""
        ));
        if let Some(body) = &self.example_request {
            command.push_str(&format!(
                " \\\n  -H \"Content-Type: application/json\" \\\n  -d '{body}'"
            ));
        }
        command
    }

    pub fn example_response_text(&self) -> String {
        serde_json::to_string_pretty(&self.example_response).unwrap_or_default()
    }
}

pub use crate::api::registry::endpoints;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavLink {
    pub href: String,
    pub method: HttpMethod,
    pub summary: &'static str,
}

pub fn groups(endpoints: &[Endpoint]) -> Vec<(&'static str, Vec<NavLink>)> {
    let mut groups: Vec<(&'static str, Vec<NavLink>)> = Vec::new();
    for endpoint in endpoints {
        let link = NavLink {
            href: format!("#{}", endpoint.id),
            method: endpoint.method,
            summary: endpoint.summary,
        };
        match groups.iter_mut().find(|(name, _)| *name == endpoint.group) {
            Some((_, links)) => links.push(link),
            None => groups.push((endpoint.group, vec![link])),
        }
    }
    groups
}

pub fn common_errors() -> Vec<ErrorDoc> {
    ApiAuthProblem::ALL
        .into_iter()
        .map(ErrorDoc::of)
        .chain([
            ErrorDoc::of(ApiFailure::NotFound),
            ErrorDoc::of(ApiFailure::Internal),
        ])
        .collect()
}

pub fn example_problem() -> String {
    let (_, _, Json(body)) = ApiProblem::of(NameProblem::TooLong).response();
    serde_json::to_string_pretty(&body).unwrap_or_default()
}

pub fn fields(schema: &Schema) -> Vec<FieldDoc> {
    let value = schema.as_value();
    let required: Vec<&str> = value["required"]
        .as_array()
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let Some(properties) = value["properties"].as_object() else {
        return Vec::new();
    };

    properties
        .iter()
        .map(|(name, property)| FieldDoc {
            name: name.clone(),
            kind: kind(property),
            required: required.contains(&name.as_str()),
            rules: rules(property),
            description: property["description"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
        .collect()
}

fn kind(property: &Value) -> String {
    match &property["type"] {
        Value::String(kind) => kind.clone(),
        Value::Array(kinds) => kinds
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" or "),
        _ => "object".to_string(),
    }
}

fn rules(property: &Value) -> String {
    let mut rules = Vec::new();
    match (
        property["minLength"].as_u64(),
        property["maxLength"].as_u64(),
    ) {
        (Some(min), Some(max)) => rules.push(format!("{min}–{max} characters")),
        (Some(min), None) => rules.push(format!("at least {min} characters")),
        (None, Some(max)) => rules.push(format!("at most {max} characters")),
        (None, None) => {}
    }
    rules.join(", ")
}

pub fn without_meta(schema: &Schema) -> Value {
    let mut value = schema.as_value().clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("$schema");
    }
    value
}

fn problem_schema() -> Value {
    json!({
        "type": "object",
        "description": "An RFC 9457 problem: every API error has this shape.",
        "required": ["type", "title", "status", "detail", "code"],
        "properties": {
            "type": { "type": "string", "const": "about:blank" },
            "title": { "type": "string", "description": "The HTTP status text." },
            "status": { "type": "integer" },
            "detail": { "type": "string", "description": "What went wrong and how to fix it." },
            "code": { "type": "string", "description": "Stable, machine-readable error code." },
            "errors": {
                "type": "array",
                "description": "Present when a specific field caused the error.",
                "items": {
                    "type": "object",
                    "required": ["field", "code", "message"],
                    "properties": {
                        "field": { "type": "string" },
                        "code": { "type": "string" },
                        "message": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn error_responses(errors: &[ErrorDoc], responses: &mut Map<String, Value>) {
    let mut by_status: Vec<(u16, Vec<&ErrorDoc>)> = Vec::new();
    for error in errors {
        match by_status
            .iter_mut()
            .find(|(status, _)| *status == error.status)
        {
            Some((_, group)) => group.push(error),
            None => by_status.push((error.status, vec![error])),
        }
    }
    for (status, group) in by_status {
        let description = group
            .iter()
            .map(|error| format!("`{}`: {}", error.code, error.message))
            .collect::<Vec<_>>()
            .join("\n");
        let examples: Map<String, Value> = group
            .iter()
            .map(|error| (error.code.to_string(), error_example(error)))
            .collect();
        responses.insert(
            status.to_string(),
            json!({
                "description": description,
                "content": { "application/problem+json": {
                    "schema": { "$ref": "#/components/schemas/Problem" },
                    "examples": examples,
                } }
            }),
        );
    }
}

fn error_example(error: &ErrorDoc) -> Value {
    let status = StatusCode::from_u16(error.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let (_, _, Json(body)) = ApiProblem::new(status, error.code, error.message).response();

    json!({ "summary": error.message, "value": body })
}

pub fn openapi(base_url: &str) -> Value {
    let common = common_errors();
    let mut paths = Map::new();
    for endpoint in endpoints() {
        let mut responses = Map::new();
        responses.insert(
            "200".to_string(),
            json!({
                "description": "Success",
                "content": { "application/json": {
                    "schema": without_meta(&endpoint.response),
                    "example": endpoint.example_response,
                } }
            }),
        );
        let errors: Vec<ErrorDoc> = endpoint
            .errors
            .iter()
            .cloned()
            .chain(common.iter().cloned())
            .collect();
        error_responses(&errors, &mut responses);

        let mut operation = json!({
            "operationId": endpoint.id,
            "summary": endpoint.summary,
            "description": format!(
                "{} Requires a token with the `{}` scope.",
                endpoint.description,
                endpoint.scope.as_str()
            ),
            "security": [{ "bearer": [] }],
            "x-required-scope": endpoint.scope.as_str(),
            "responses": responses,
        });
        if let Some(request) = &endpoint.request {
            operation["requestBody"] = json!({
                "required": true,
                "content": { "application/json": {
                    "schema": without_meta(request),
                    "example": endpoint.example_request,
                } }
            });
        }

        let path = paths
            .entry(endpoint.path.to_string())
            .or_insert_with(|| json!({}));
        path[endpoint.method.openapi_key()] = operation;
    }

    let scopes: Map<String, Value> = ApiScope::ALL
        .into_iter()
        .map(|scope| (scope.as_str().to_string(), json!(scope.label())))
        .collect();

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": TITLE,
            "version": VERSION,
            "description": "Personal API tokens are created under Account → Developer → API tokens. Send them as `Authorization: Bearer <token>`.",
        },
        "servers": [{ "url": base_url }],
        "security": [{ "bearer": [] }],
        "paths": paths,
        "components": {
            "securitySchemes": {
                "bearer": {
                    "type": "http",
                    "scheme": "bearer",
                    "description": "A personal API token (wsk_…).",
                }
            },
            "schemas": { "Problem": problem_schema() },
        },
        "x-scopes": scopes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_documented_error_has_an_example_problem() {
        let document = openapi("http://localhost");
        for (path, operations) in document["paths"].as_object().unwrap() {
            for (method, operation) in operations.as_object().unwrap() {
                for (status, response) in operation["responses"].as_object().unwrap() {
                    if status == "200" {
                        continue;
                    }
                    let examples = &response["content"]["application/problem+json"]["examples"];
                    let examples = examples.as_object().unwrap();
                    assert!(!examples.is_empty(), "{method} {path} {status}");
                    for (code, example) in examples {
                        assert_eq!(example["value"]["code"], code.as_str());
                        assert_eq!(example["value"]["status"].to_string(), *status);
                    }
                }
            }
        }
    }

    #[test]
    fn request_fields_come_from_the_action_types() {
        let update = endpoints()
            .into_iter()
            .find(|endpoint| endpoint.id == "update-profile")
            .unwrap();

        assert_eq!(
            update.request_fields(),
            vec![FieldDoc {
                name: "name".to_string(),
                kind: "string".to_string(),
                required: true,
                rules: "1–100 characters".to_string(),
                description:
                    "New display name; must not be blank. Leading and trailing spaces are removed."
                        .to_string(),
            }]
        );
        assert!(
            update
                .errors
                .iter()
                .any(|error| error.code == "name_too_long")
        );
    }

    #[test]
    fn real_error_responses_match_the_documented_problem_shape() {
        let (_, _, Json(body)) = ApiProblem::of(NameProblem::TooLong).response();
        let schema = problem_schema();

        for key in schema["required"].as_array().unwrap() {
            assert!(body.get(key.as_str().unwrap()).is_some(), "missing {key}");
        }
        for key in body.as_object().unwrap().keys() {
            assert!(
                schema["properties"].get(key).is_some(),
                "undocumented {key}"
            );
        }
    }

    #[test]
    fn the_openapi_document_lists_every_endpoint_with_its_scope() {
        let spec = openapi("https://example.test");

        assert_eq!(spec["openapi"], "3.1.0");
        for endpoint in endpoints() {
            let operation = &spec["paths"][endpoint.path][endpoint.method.openapi_key()];
            assert_eq!(operation["operationId"], endpoint.id);
            assert_eq!(operation["x-required-scope"], endpoint.scope.as_str());
            assert!(operation["responses"]["401"].is_object());
        }
        assert!(spec["paths"]["/api/v1/me"]["patch"]["requestBody"].is_object());
        assert!(
            spec["paths"]["/api/v1/me"]["patch"]["responses"]["200"]["content"]["application/json"]
                ["schema"]
                .get("$schema")
                .is_none()
        );
    }
}
