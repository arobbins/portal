use topcoat::{
    Result,
    context::Cx,
    router::{href, route},
};

use crate::account::tabs::AccountSection;
use crate::api::catalog::{TITLE, endpoints};
use crate::api::v1::docs::openapi_document;
use crate::app::SITE_NAME;
use crate::mcp::{PROTOCOL_VERSION, mcp_endpoint};
use crate::pages::api_docs::api_docs_page;
use crate::web::app;

#[route(GET "/llms.txt")]
pub async fn llms_txt(cx: &Cx) -> Result<String> {
    let openapi = app(cx).url(&href!(openapi_document).resolve(cx));
    let docs = app(cx).url(&href!(api_docs_page).resolve(cx));
    let tokens = app(cx).url(&AccountSection::ApiTokens.path());
    let mcp = app(cx).url(&href!(mcp_endpoint).resolve(cx));
    let endpoints: Vec<String> = endpoints()
        .iter()
        .map(|endpoint| {
            format!(
                "- `{} {}`: {} (scope `{}`)",
                endpoint.method.as_str(),
                endpoint.path,
                endpoint.summary,
                endpoint.scope.as_str()
            )
        })
        .collect();

    Ok(format!(
        "# {SITE_NAME}\n\n\
         > {TITLE}: a JSON API for acting on a user's account with a personal API token.\n\n\
         - [OpenAPI document]({openapi}): every endpoint, schema, scope and error code\n\
         - [API reference]({docs}): the same, for people\n\
         - [Create a token]({tokens}): tokens are created by the account owner in the app\n\n\
         ## Using the API\n\n\
         Send `Authorization: Bearer <token>`. Errors are `application/problem+json` with a \
         stable `code`, a `detail` that says how to fix the request and, for field problems, \
         `errors[].field`. A missing scope is a 403 `insufficient_scope` naming the scope.\n\n\
         ## Endpoints\n\n{}\n\n\
         ## MCP\n\n\
         The same actions are MCP tools at `{mcp}` (Streamable HTTP, JSON-RPC 2.0, \
         protocol `{PROTOCOL_VERSION}`). Create a token with \"Used by: AI assistants (MCP)\" \
         and send it as a Bearer token; `tools/list` shows only the tools the token's scopes allow.\n",
        endpoints.join("\n")
    ))
}
