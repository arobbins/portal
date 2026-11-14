use axum::http::{HeaderMap, HeaderValue, header::CONTENT_TYPE};
use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, StatusCode, content::Json, route},
};

use crate::action::Caller;
use crate::api::auth::authenticate;
use crate::api::body::{MAX_JSON_BODY, read_json};
use crate::api::catalog::without_meta;
use crate::api::problem::{ApiResponse, no_store, or_internal_error};
use crate::api::registry::{Tool, ToolOutcome, tools};
use crate::api::scope::TokenAudience;
use crate::app::SITE_NAME;

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const SUPPORTED_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26"];

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

type McpResponse = (StatusCode, HeaderMap, String);

#[route(POST "/mcp", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn mcp_endpoint(cx: &Cx, body: Body) -> Result<McpResponse> {
    let caller = match authenticate(cx, TokenAudience::Mcp).await {
        Ok(Ok(caller)) => caller,
        Ok(Err(problem)) => return Ok(from_api(problem.response())),
        Err(error) => return Ok(from_api(or_internal_error(Err(error)))),
    };
    let message: Value = match read_json(cx, body).await? {
        Ok(message) => message,
        Err(response) => return Ok(from_api(response)),
    };

    handle(cx, &caller, message).await
}

async fn handle(cx: &Cx, caller: &Caller, message: Value) -> Result<McpResponse> {
    let Some(request) = message.as_object() else {
        return Ok(rpc_error(
            Value::Null,
            INVALID_REQUEST,
            "Send one JSON-RPC 2.0 object.",
        ));
    };
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str);
    let (Some("2.0"), Some(method)) = (request.get("jsonrpc").and_then(Value::as_str), method)
    else {
        return Ok(rpc_error(
            id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "A request needs \"jsonrpc\": \"2.0\" and a \"method\".",
        ));
    };
    let Some(id) = id else {
        return Ok((StatusCode::ACCEPTED, no_store(), String::new()));
    };
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "initialize" => Ok(rpc_result(id, initialize(&params))),
        "ping" => Ok(rpc_result(id, json!({}))),
        "tools/list" => Ok(rpc_result(id, list_tools(caller))),
        "tools/call" => call_tool(cx, caller, id, &params).await,
        other => Ok(rpc_error(
            id,
            METHOD_NOT_FOUND,
            &format!(
                "Unknown method `{other}`. This server supports initialize, ping, tools/list and tools/call."
            ),
        )),
    }
}

fn initialize(params: &Value) -> Value {
    let requested = params["protocolVersion"]
        .as_str()
        .unwrap_or(PROTOCOL_VERSION);
    let version = if SUPPORTED_VERSIONS.contains(&requested) {
        requested
    } else {
        PROTOCOL_VERSION
    };

    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": SITE_NAME, "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Tools act on the account that owns the token, within the token's scopes. Errors come back as problem JSON with a stable `code` and a `detail` that says how to fix the call.",
    })
}

fn usable(caller: &Caller, tool: &Tool) -> bool {
    caller.require(tool.permission).is_ok()
}

fn describe(tool: &Tool) -> Value {
    let endpoint = &tool.endpoint;
    let input = endpoint.request.as_ref().map(without_meta).unwrap_or_else(
        || json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    );

    json!({
        "name": tool.name(),
        "title": endpoint.summary,
        "description": format!("{} Needs the `{}` scope.", endpoint.description, endpoint.scope.as_str()),
        "inputSchema": input,
        "outputSchema": without_meta(&endpoint.response),
    })
}

fn list_tools(caller: &Caller) -> Value {
    let tools: Vec<Value> = tools()
        .iter()
        .filter(|tool| usable(caller, tool))
        .map(describe)
        .collect();

    json!({ "tools": tools })
}

async fn call_tool(cx: &Cx, caller: &Caller, id: Value, params: &Value) -> Result<McpResponse> {
    let Some(name) = params["name"].as_str() else {
        return Ok(rpc_error(
            id,
            INVALID_PARAMS,
            "tools/call needs a \"name\".",
        ));
    };
    let all = tools();
    let Some(tool) = all.iter().find(|tool| tool.name() == name) else {
        return Ok(rpc_error(
            id,
            INVALID_PARAMS,
            &format!("Unknown tool `{name}`. Call tools/list to see what this token can use."),
        ));
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let result = match (tool.run)(cx, caller, arguments).await? {
        ToolOutcome::Success(output) => json!({
            "content": [{ "type": "text", "text": pretty(&output) }],
            "structuredContent": output,
            "isError": false,
        }),
        ToolOutcome::Failed(problem) => json!({
            "content": [{ "type": "text", "text": pretty(&problem) }],
            "isError": true,
        }),
        ToolOutcome::InvalidArguments(reason) => {
            return Ok(rpc_error(
                id,
                INVALID_PARAMS,
                &format!("The arguments don't match the tool's inputSchema: {reason}"),
            ));
        }
    };

    Ok(rpc_result(id, result))
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

fn json_response(status: StatusCode, body: &Value) -> McpResponse {
    let mut headers = no_store();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    (status, headers, body.to_string())
}

fn rpc_result(id: Value, result: Value) -> McpResponse {
    json_response(
        StatusCode::OK,
        &json!({ "jsonrpc": "2.0", "id": id, "result": result }),
    )
}

fn rpc_error(id: Value, code: i64, message: &str) -> McpResponse {
    let status = if code == PARSE_ERROR || code == INVALID_REQUEST {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::OK
    };
    json_response(
        status,
        &json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
    )
}

fn from_api((status, headers, Json(body)): ApiResponse) -> McpResponse {
    (status, headers, body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::scope::ApiScope;
    use crate::user::model::User;
    use std::sync::Arc;

    fn caller(scopes: Vec<ApiScope>) -> Caller {
        Caller::api_token(Arc::new(User::default_for_tests()), 1, scopes)
    }

    fn body(response: &McpResponse) -> Value {
        serde_json::from_str(&response.2).unwrap()
    }

    #[test]
    fn a_token_only_sees_the_tools_its_scopes_allow() {
        let names = |scopes| {
            list_tools(&caller(scopes))["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tool| tool["name"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };

        assert_eq!(names(vec![ApiScope::ProfileRead]), vec!["get_profile"]);
        assert_eq!(
            names(vec![ApiScope::ProfileWrite]),
            vec!["get_profile", "update_profile"]
        );
        assert!(names(vec![]).is_empty());
    }

    #[test]
    fn every_tool_has_an_object_input_schema() {
        for tool in tools() {
            assert_eq!(
                describe(&tool)["inputSchema"]["type"],
                "object",
                "{}",
                tool.name()
            );
        }
    }

    #[test]
    fn initialize_agrees_on_a_supported_version() {
        assert_eq!(
            initialize(&json!({ "protocolVersion": "2025-03-26" }))["protocolVersion"],
            "2025-03-26"
        );
        assert_eq!(
            initialize(&json!({ "protocolVersion": "1999-01-01" }))["protocolVersion"],
            PROTOCOL_VERSION
        );
    }

    #[tokio::test]
    async fn requests_are_validated_and_notifications_get_no_reply() {
        let cx = Cx::new();
        let reader = caller(vec![ApiScope::ProfileRead]);

        let notification = handle(
            &cx,
            &reader,
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )
        .await
        .unwrap();
        assert_eq!(notification.0, StatusCode::ACCEPTED);

        let unknown = handle(
            &cx,
            &reader,
            json!({ "jsonrpc": "2.0", "id": 1, "method": "nope" }),
        )
        .await
        .unwrap();
        assert_eq!(body(&unknown)["error"]["code"], METHOD_NOT_FOUND);

        let not_rpc = handle(&cx, &reader, json!({ "id": 2 })).await.unwrap();
        assert_eq!(not_rpc.0, StatusCode::BAD_REQUEST);

        let call = handle(
            &cx,
            &reader,
            json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "get_profile" } }),
        )
        .await
        .unwrap();
        assert_eq!(
            body(&call)["result"]["structuredContent"]["email"],
            "tester@example.test"
        );
    }
}
