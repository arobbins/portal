use schemars::schema_for;
use serde::Serialize;
use serde_json::Value;

use crate::action::ListedProblem;
use crate::api::action::ApiAction;
use crate::api::body::BodyProblem;
use crate::api::catalog::{Endpoint, ErrorDoc, HttpMethod};
use crate::api::problem::ApiFailure;
use crate::permission::TokenAccess;
use crate::server::tools::tools;

pub fn endpoints() -> Vec<Endpoint> {
    tools().into_iter().map(|tool| tool.endpoint).collect()
}

fn has_body(method: HttpMethod) -> bool {
    method != HttpMethod::Get
}

pub fn endpoint<A: ApiAction>() -> Endpoint {
    let body = has_body(A::METHOD);
    let public = A::PERMISSION.token_access() == TokenAccess::Public;
    let mut errors = Vec::new();
    if !public {
        errors.push(ErrorDoc::of(ApiFailure::InsufficientScope));
    }
    if body {
        errors.extend(BodyProblem::ALL.into_iter().map(ErrorDoc::of));
    }
    errors.extend(A::Problem::all().iter().copied().map(ErrorDoc::of));

    Endpoint {
        id: A::ID,
        group: A::GROUP,
        method: A::METHOD,
        path: A::PATH,
        summary: A::SUMMARY,
        description: A::DESCRIPTION,
        public,
        success: A::SUCCESS.as_u16(),
        scope: match A::PERMISSION.token_access() {
            TokenAccess::Scope(scope) => Some(scope),
            TokenAccess::AnyToken | TokenAccess::Public => None,
            TokenAccess::BrowserOnly => panic!("an ApiAction's permission must allow tokens"),
        },
        request: body.then(|| schema_for!(A::Input)),
        response: schema_for!(A::Output),
        example_request: body
            .then(A::example_input)
            .flatten()
            .map(|input| to_value(&input)),
        example_response: to_value(&A::example_output()),
        errors,
    }
}

pub(super) fn to_value<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_ids_are_unique_and_each_method_and_path_is_used_once() {
        let endpoints = endpoints();
        for endpoint in &endpoints {
            assert_eq!(
                endpoints
                    .iter()
                    .filter(|other| other.id == endpoint.id)
                    .count(),
                1,
                "{}",
                endpoint.id
            );
            assert_eq!(
                endpoints
                    .iter()
                    .filter(|other| other.method == endpoint.method && other.path == endpoint.path)
                    .count(),
                1,
                "{} {}",
                endpoint.method.as_str(),
                endpoint.path
            );
            assert!(endpoint.path.starts_with("/api/v1/"));
        }
    }

    #[test]
    fn only_requests_with_a_body_document_one() {
        for endpoint in endpoints() {
            assert_eq!(endpoint.request.is_some(), has_body(endpoint.method));
            assert_eq!(
                endpoint.example_request.is_some(),
                has_body(endpoint.method)
            );
        }
    }
}
