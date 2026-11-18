use schemars::Schema;
use serde_json::{Map, Value, json};
use topcoat::router::{StatusCode, content::Json};

use crate::api::catalog::{Endpoint, ErrorDoc, TITLE, VERSION, common_errors, endpoints};
use crate::api::problem::ApiProblem;
use crate::api::scope::ApiScope;

pub fn openapi(base_url: &str) -> Value {
    let common = common_errors();
    let mut paths = Map::new();
    for endpoint in endpoints() {
        let path = paths
            .entry(endpoint.path.to_string())
            .or_insert_with(|| json!({}));
        path[endpoint.method.openapi_key()] = operation(&endpoint, &common);
    }
    document(base_url, paths)
}

fn document(base_url: &str, paths: Map<String, Value>) -> Value {
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

fn operation(endpoint: &Endpoint, common: &[ErrorDoc]) -> Value {
    let mut operation = json!({
        "operationId": endpoint.id,
        "summary": endpoint.summary,
        "description": format!("{} {}", endpoint.description, endpoint.access_note()),
        "security": [{ "bearer": [] }],
        "x-required-scope": endpoint.scope_name(),
        "responses": responses(endpoint, common),
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
    operation
}

fn responses(endpoint: &Endpoint, common: &[ErrorDoc]) -> Map<String, Value> {
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
    let errors: Vec<ErrorDoc> = endpoint.errors.iter().chain(common).cloned().collect();
    error_responses(&errors, &mut responses);
    responses
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::name::NameProblem;

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
            assert_eq!(operation["x-required-scope"], endpoint.scope_name());
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
