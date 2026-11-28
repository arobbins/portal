use axum::http::{HeaderMap, HeaderValue, header::CONTENT_TYPE};
use serde_json::{Value, json};
use topcoat::router::{StatusCode, content::Json};

use crate::api::problem::{ApiResponse, no_store};

pub(super) type McpResponse = (StatusCode, HeaderMap, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RpcError {
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
}

impl RpcError {
    pub(super) fn code(self) -> i64 {
        match self {
            Self::InvalidRequest => -32600,
            Self::MethodNotFound => -32601,
            Self::InvalidParams => -32602,
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::InvalidRequest => StatusCode::BAD_REQUEST,
            Self::MethodNotFound | Self::InvalidParams => StatusCode::OK,
        }
    }
}

fn json_response(status: StatusCode, body: &Value) -> McpResponse {
    let mut headers = no_store();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    (status, headers, body.to_string())
}

pub(super) fn rpc_result(id: Value, result: Value) -> McpResponse {
    json_response(
        StatusCode::OK,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

pub(super) fn rpc_error(id: Value, error: RpcError, message: &str) -> McpResponse {
    let body = json!({ "jsonrpc": "2.0", "id": id, "error": { "code": error.code(), "message": message } });
    json_response(error.status(), &body)
}

pub(super) fn from_api((status, headers, Json(body)): ApiResponse) -> McpResponse {
    (status, headers, body.to_string())
}
