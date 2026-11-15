use axum::http::{
    HeaderMap, HeaderValue,
    header::{CACHE_CONTROL, CONTENT_TYPE, WWW_AUTHENTICATE},
};
use serde::Serialize;
use serde_json::{Map, Value, json};
use topcoat::router::{StatusCode, content::Json};

use crate::action::{ActionError, ActionProblem, Denied, ProblemSpec};

pub const REALM: &str = "api";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiFailure {
    Forbidden,
    InsufficientScope,
    BrowserOnly,
    NotFound,
    Internal,
}

impl ActionProblem for ApiFailure {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Forbidden => ProblemSpec::new(
                "forbidden",
                "Your role in this workspace doesn't allow this.",
            )
            .with_status(StatusCode::FORBIDDEN),
            Self::InsufficientScope => ProblemSpec::new(
                "insufficient_scope",
                "This API token doesn't have the scope this endpoint needs. Create a token with that access.",
            )
            .with_status(StatusCode::FORBIDDEN),
            Self::BrowserOnly => ProblemSpec::new(
                "browser_only",
                "This can only be done in the app, not with an API token.",
            )
            .with_status(StatusCode::FORBIDDEN),
            Self::NotFound => ProblemSpec::new(
                "not_found",
                "There's no API endpoint for this method and path. The API lives under /api/v1.",
            )
            .with_status(StatusCode::NOT_FOUND),
            Self::Internal => ProblemSpec::new(
                "internal_error",
                "Something went wrong on our side. Try again in a moment.",
            )
            .with_status(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

pub type ApiResponse = (StatusCode, HeaderMap, Json<Value>);

pub struct ApiProblem {
    status: StatusCode,
    code: &'static str,
    detail: String,
    field: Option<&'static str>,
    extra: Map<String, Value>,
    headers: HeaderMap,
}

impl ApiProblem {
    pub fn new(status: StatusCode, code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status,
            code,
            detail: detail.into(),
            field: None,
            extra: Map::new(),
            headers: no_store(),
        }
    }

    pub fn of<P: ActionProblem>(problem: P) -> Self {
        Self {
            field: problem.field(),
            ..Self::new(problem.status(), problem.code(), problem.message())
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    pub fn extra(mut self, key: &str, value: Value) -> Self {
        self.extra.insert(key.to_string(), value);
        self
    }

    pub fn header(mut self, name: axum::http::HeaderName, value: &str) -> Self {
        if let Ok(value) = HeaderValue::from_str(value) {
            self.headers.insert(name, value);
        }
        self
    }

    pub fn response(self) -> ApiResponse {
        let mut body = json!({
            "type": "about:blank",
            "title": self.status.canonical_reason().unwrap_or("Error"),
            "status": self.status.as_u16(),
            "detail": self.detail,
            "code": self.code,
        });
        if let Some(field) = self.field {
            body["errors"] = json!([{ "field": field, "code": self.code, "message": self.detail }]);
        }
        for (key, value) in self.extra {
            body[key] = value;
        }
        let mut headers = self.headers;
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );

        (self.status, headers, Json(body))
    }
}

pub fn no_store() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers
}

pub fn ok(body: &impl Serialize) -> ApiResponse {
    let body = serde_json::to_value(body).unwrap_or(Value::Null);
    (StatusCode::OK, no_store(), Json(body))
}

pub fn denied(denied: Denied) -> ApiResponse {
    match denied {
        Denied::MissingScope(scope) => ApiProblem::of(ApiFailure::InsufficientScope)
            .detail(format!(
                "This API token needs the {} scope. Create a token with that access.",
                scope.as_str()
            ))
            .header(
                WWW_AUTHENTICATE,
                &format!(
                    "Bearer realm=\"{REALM}\", error=\"insufficient_scope\", scope=\"{}\"",
                    scope.as_str()
                ),
            )
            .response(),
        Denied::BrowserOnly => ApiProblem::of(ApiFailure::BrowserOnly).response(),
        Denied::NotPermitted(_) => ApiProblem::of(ApiFailure::Forbidden).response(),
    }
}

pub fn action_failed<P: ActionProblem>(error: ActionError<P>) -> ApiResponse {
    match error {
        ActionError::Denied(reason) => denied(reason),
        ActionError::Problem(problem) => ApiProblem::of(problem).response(),
    }
}

pub fn respond<T: Serialize, P: ActionProblem>(
    outcome: std::result::Result<T, ActionError<P>>,
) -> ApiResponse {
    match outcome {
        Ok(value) => ok(&value),
        Err(error) => action_failed(error),
    }
}

pub fn or_internal_error(result: topcoat::Result<ApiResponse>) -> ApiResponse {
    result.unwrap_or_else(|error| {
        tracing::error!("API request failed: {error}");
        ApiProblem::of(ApiFailure::Internal).response()
    })
}
