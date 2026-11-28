use axum::http::{HeaderValue, header::WWW_AUTHENTICATE};
use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, StatusCode, route},
};

use crate::action::Caller;
use crate::api::auth::{ApiAuthProblem, authenticate};
use crate::api::body::{MAX_JSON_BODY, read_json};
use crate::api::openapi::without_meta;
use crate::api::problem::{no_store, or_internal_error};
use crate::api::scope::TokenAudience;
use crate::api::tool::{Tool, ToolOutcome};
use crate::app::SITE_NAME;
use crate::env::Env;
use crate::routes::PROTECTED_RESOURCE_METADATA_PATH;
use crate::server::tools::{feedback_tool, tools};
use crate::web::env;
use rpc::{McpResponse, RpcError, from_api, rpc_error, rpc_result};

mod rpc;

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const SUPPORTED_VERSIONS: &[&str] = &[PROTOCOL_VERSION, "2025-03-26"];

#[route(POST "/mcp", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn mcp_endpoint(cx: &Cx, body: Body) -> Result<McpResponse> {
    let env = env(cx);
    let caller = match authenticate(cx, TokenAudience::Mcp).await {
        Ok(Ok(caller)) => Some(caller),
        Ok(Err(ApiAuthProblem::MissingToken)) => None,
        Ok(Err(problem)) => return Ok(unauthorized(&env, problem)),
        Err(error) => return Ok(from_api(or_internal_error(Err(error)))),
    };
    let message: Value = match read_json(cx, body).await? {
        Ok(message) => message,
        Err(response) => return Ok(from_api(response)),
    };

    handle(&env, caller.as_ref(), message).await
}

fn unauthorized(env: &Env<'_>, problem: ApiAuthProblem) -> McpResponse {
    let (status, mut headers, body) = from_api(problem.response());
    let metadata = env.app.url(PROTECTED_RESOURCE_METADATA_PATH);
    let challenge = match problem {
        ApiAuthProblem::InvalidToken => {
            format!("Bearer resource_metadata=\"{metadata}\", error=\"invalid_token\"")
        }
        _ => format!("Bearer resource_metadata=\"{metadata}\""),
    };
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        headers.insert(WWW_AUTHENTICATE, value);
    }
    (status, headers, body)
}

async fn handle(env: &Env<'_>, caller: Option<&Caller>, message: Value) -> Result<McpResponse> {
    let Some(request) = message.as_object() else {
        return Ok(rpc_error(
            Value::Null,
            RpcError::InvalidRequest,
            "Send one JSON-RPC 2.0 object.",
        ));
    };
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str);
    let (Some("2.0"), Some(method)) = (request.get("jsonrpc").and_then(Value::as_str), method)
    else {
        return Ok(rpc_error(
            id.unwrap_or(Value::Null),
            RpcError::InvalidRequest,
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
        "tools/call" => call_tool(env, caller, id, &params).await,
        other => Ok(rpc_error(
            id,
            RpcError::MethodNotFound,
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
        "instructions": format!(
            "Tools act on the account that owns the token, within the token's scopes. Errors come back as problem JSON with a stable `code` and a `detail` that says how to fix the call. If a tool, its schema or an error got in your way, call `{}` to tell the people who run this API; they use it to fix the API and its docs.",
            feedback_tool().name()
        ),
    })
}

fn usable(caller: Option<&Caller>, tool: &Tool) -> bool {
    match caller {
        Some(caller) => caller.require(tool.permission).is_ok(),
        None => tool.run_public.is_some(),
    }
}

fn describe(tool: &Tool) -> Value {
    let endpoint = &tool.endpoint;
    let input = endpoint.request.as_ref().map(without_meta).unwrap_or_else(
        || json!({ "type": "object", "properties": {}, "additionalProperties": false }),
    );

    json!({
        "name": tool.name(),
        "title": endpoint.summary,
        "description": format!("{} {}", endpoint.description, endpoint.access_note()),
        "inputSchema": input,
        "outputSchema": without_meta(&endpoint.response),
    })
}

fn list_tools(caller: Option<&Caller>) -> Value {
    let tools: Vec<Value> = tools()
        .iter()
        .filter(|tool| usable(caller, tool))
        .map(describe)
        .collect();

    json!({ "tools": tools })
}

async fn call_tool(
    env: &Env<'_>,
    caller: Option<&Caller>,
    id: Value,
    params: &Value,
) -> Result<McpResponse> {
    let Some(name) = params["name"].as_str() else {
        return Ok(rpc_error(
            id,
            RpcError::InvalidParams,
            "tools/call needs a \"name\".",
        ));
    };
    let all = tools();
    let Some(tool) = all.iter().find(|tool| tool.name() == name) else {
        return Ok(rpc_error(
            id,
            RpcError::InvalidParams,
            &format!("Unknown tool `{name}`. Call tools/list to see what this token can use."),
        ));
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let outcome = match (caller, tool.run_public) {
        (Some(caller), _) => (tool.run)(env, caller, arguments).await?,
        (None, Some(run_public)) => run_public(env, arguments).await?,
        (None, None) => return Ok(unauthorized(env, ApiAuthProblem::MissingToken)),
    };
    Ok(tool_response(id, outcome))
}

fn tool_response(id: Value, outcome: ToolOutcome) -> McpResponse {
    let result = match outcome {
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
            let message = format!("The arguments don't match the tool's inputSchema: {reason}");
            return rpc_error(id, RpcError::InvalidParams, &message);
        }
    };
    rpc_result(id, result)
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
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
            list_tools(Some(&caller(scopes)))["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tool| tool["name"].as_str().unwrap().to_string())
                .collect::<Vec<_>>()
        };

        assert_eq!(
            names(vec![ApiScope::ProfileRead]),
            vec![
                "get_profile",
                "get_newsletters",
                "send_feedback",
                "list_plans"
            ]
        );
        assert_eq!(
            names(vec![ApiScope::ProfileWrite]),
            vec![
                "get_profile",
                "update_profile",
                "get_newsletters",
                "update_newsletters",
                "send_feedback",
                "list_plans"
            ]
        );
        assert_eq!(names(vec![]), vec!["send_feedback", "list_plans"]);
        assert_eq!(
            names(vec![ApiScope::BillingWrite]),
            vec![
                "send_feedback",
                "list_plans",
                "get_billing",
                "start_checkout"
            ]
        );
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
        let db = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap();
        let app = crate::env::App::for_tests(db);
        let env = app.env(None);
        let reader = caller(vec![ApiScope::ProfileRead]);

        let notification = handle(
            &env,
            Some(&reader),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )
        .await
        .unwrap();
        assert_eq!(notification.0, StatusCode::ACCEPTED);

        let unknown = handle(
            &env,
            Some(&reader),
            json!({ "jsonrpc": "2.0", "id": 1, "method": "nope" }),
        )
        .await
        .unwrap();
        assert_eq!(
            body(&unknown)["error"]["code"],
            RpcError::MethodNotFound.code()
        );

        let not_rpc = handle(&env, Some(&reader), json!({ "id": 2 }))
            .await
            .unwrap();
        assert_eq!(not_rpc.0, StatusCode::BAD_REQUEST);

        let call = handle(
            &env,
            Some(&reader),
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
