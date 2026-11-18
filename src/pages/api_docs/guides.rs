use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::code_method_class;
use crate::api::body::MAX_JSON_BODY_KB;
use crate::api::catalog::{Endpoint, ErrorDoc, TOKEN_VARIABLE, example_problem};
use crate::api::scope::ApiScope;
use crate::components::button::{ButtonVariant, button};
use crate::components::code_block::{code_block, code_panel};
use crate::components::notice::notice;
use crate::components::property_list::{property, property_list};
use crate::components::tone::Tone;
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::rate_limit::{
    API_CALLS_PER_MINUTE, BAD_API_TOKEN_WINDOW_MINUTES, BAD_API_TOKENS_PER_WINDOW,
};
use crate::syntax::CodeLanguage;

use super::endpoints::error_list;

#[component]
pub(super) async fn overview_section(
    cx: &Cx,
    endpoints: &[Endpoint],
    tokens_url: &str,
    openapi_url: &str,
) -> Result<impl View> {
    Ok(view! {
        <section class="l-docs__section l-docs__section--first" id="overview">
            <div class="l-docs__prose">
                page_heading(
                    text: "API reference",
                    variant: PageHeadingVariant::Compact
                )
                <p class="a-lead">
                    "Use your account from your own scripts, apps and AI assistants. Every endpoint takes and returns JSON, and this page is generated from the same code the app runs, so it's always up to date."
                </p>
                <div class="p-api-docs__actions">
                    button(href: Some(tokens_url), "Create API token")
                    button(
                        variant: ButtonVariant::Secondary,
                        href: Some(openapi_url),
                        "OpenAPI spec (JSON)"
                    )
                </div>
            </div>
            <div class="l-docs__aside">
                code_panel(title: "Endpoints", endpoint_index(endpoints: endpoints))
            </div>
        </section>
    })
}

#[component]
async fn endpoint_index(cx: &Cx, endpoints: &[Endpoint]) -> Result<impl View> {
    Ok(view! {
        <ul class="p-api-docs__endpoint-list">
            #[key(endpoint.id)]
            for endpoint in endpoints {
                <li>
                    <a
                        class="p-api-docs__endpoint-link"
                        href=(format!("#{}", endpoint.id))
                    >
                        <span class=(code_method_class(endpoint.method))>
                            (endpoint.method.as_str())
                        </span>
                        <span class="p-api-docs__endpoint-path">(endpoint.path)</span>
                        <span class="p-api-docs__endpoint-summary">
                            (endpoint.summary)
                        </span>
                    </a>
                </li>
            }
        </ul>
    })
}

#[component]
pub(super) async fn authentication_section(
    cx: &Cx,
    origin: &str,
    tokens_url: &str,
) -> Result<impl View> {
    let first_request = format!(
        "export {TOKEN_VARIABLE}=wsk_…\n\ncurl {origin}/api/v1/me \\\n  -H \"Authorization: Bearer ${TOKEN_VARIABLE}\""
    );

    Ok(view! {
        <section class="l-docs__section" id="authentication">
            <div class="l-docs__prose">
                <h2 class="p-api-docs__heading">"Authentication"</h2>
                <p>
                    "Create a token under "
                    <a href=(tokens_url)>"Account → Developer → API tokens"</a>
                    " and send it in the "
                    <code>"Authorization"</code>
                    " header of every request. Tokens start with "
                    <code>"wsk_"</code>
                    ", are shown once, and expire on the date you choose."
                </p>
                <p>
                    "Base URL: "
                    <code>(origin)</code>
                </p>
            </div>
            <div class="l-docs__aside">
                code_panel(
                    title: "Your first request",
                    code_block(code: &first_request, language: CodeLanguage::Shell)
                )
            </div>
        </section>
    })
}

fn implied_by(scope: ApiScope) -> Option<String> {
    let wider: Vec<&str> = ApiScope::ALL
        .into_iter()
        .filter(|other| *other != scope && other.implies(scope))
        .map(ApiScope::as_str)
        .collect();
    (!wider.is_empty()).then(|| format!("Also granted by {}.", wider.join(", ")))
}

#[component]
pub(super) async fn scopes_section(cx: &Cx) -> Result<impl View> {
    let scope_rows: Vec<(ApiScope, Option<String>)> = ApiScope::ALL
        .into_iter()
        .map(|scope| (scope, implied_by(scope)))
        .collect();

    Ok(view! {
        <section class="l-docs__section l-docs__section--single" id="scopes">
            <div class="l-docs__prose">
                <h2 class="p-api-docs__heading">"Scopes"</h2>
                <p>"A token can only do what its scopes allow."</p>
                notice(
                    tone: Tone::Info,
                    "Changing your password, email, or passkeys is never possible with a token."
                )
                property_list(
                    #[key(scope.as_str())]
                    for (scope, note) in &scope_rows {
                        property(
                            name: scope.as_str(),
                            code: true,
                            description: Some(scope.label()),
                            detail: note.as_deref()
                        )
                    }
                )
            </div>
        </section>
    })
}

#[component]
pub(super) async fn errors_section(cx: &Cx, common: &[ErrorDoc]) -> Result<impl View> {
    let problem = example_problem();

    Ok(view! {
        <section class="l-docs__section" id="errors">
            <div class="l-docs__prose">
                <h2 class="p-api-docs__heading">"Errors"</h2>
                <p>
                    "Errors use the standard "
                    <code>"application/problem+json"</code>
                    " format. "
                    <code>"code"</code>
                    " never changes, so branch on it; "
                    <code>"detail"</code>
                    " says how to fix the request, and "
                    <code>"errors"</code>
                    " names the field at fault. Any endpoint can return these:"
                </p>
                error_list(errors: common)
            </div>
            <div class="l-docs__aside">
                code_panel(
                    title: "Example error",
                    code_block(code: &problem, language: CodeLanguage::Json)
                )
            </div>
        </section>
    })
}

#[component]
pub(super) async fn limits_section(cx: &Cx) -> Result<impl View> {
    let call_limit = format!("{API_CALLS_PER_MINUTE} requests in any rolling minute, per token.");
    let bad_token_limit = format!(
        "{BAD_API_TOKENS_PER_WINDOW} invalid tokens from one address in {BAD_API_TOKEN_WINDOW_MINUTES} minutes blocks that address for {BAD_API_TOKEN_WINDOW_MINUTES} minutes."
    );
    let body_limit = format!("Request bodies can be at most {MAX_JSON_BODY_KB} KB.");

    Ok(view! {
        <section class="l-docs__section l-docs__section--single" id="limits">
            <div class="l-docs__prose">
                <h2 class="p-api-docs__heading">"Limits"</h2>
                <ul>
                    <li>(call_limit)</li>
                    <li>(bad_token_limit)</li>
                    <li>(body_limit)</li>
                </ul>
                <p>
                    "Over a limit you get "
                    <code>"429"</code>
                    " with a "
                    <code>"Retry-After"</code>
                    " header in seconds."
                </p>
            </div>
        </section>
    })
}
