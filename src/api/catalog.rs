use schemars::Schema;
use serde_json::Value;
use topcoat::router::content::Json;

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
    Post,
    Patch,
}

impl HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Patch => "PATCH",
        }
    }

    pub fn openapi_key(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
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
    pub scope: Option<ApiScope>,
    pub request: Option<Schema>,
    pub response: Schema,
    pub example_request: Option<Value>,
    pub example_response: Value,
    pub errors: Vec<ErrorDoc>,
}

impl Endpoint {
    pub fn access_note(&self) -> String {
        match self.scope {
            Some(scope) => format!("Requires a token with the `{}` scope.", scope.as_str()),
            None => "Works with any valid token, whatever its scopes.".to_string(),
        }
    }

    pub fn scope_name(&self) -> &'static str {
        self.scope.map_or("any", ApiScope::as_str)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
