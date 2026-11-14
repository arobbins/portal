use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page},
    view::{View, component, view},
};

use crate::account::tabs::AccountSection;
use crate::api::body::MAX_JSON_BODY_KB;
use crate::api::catalog::{
    Endpoint, ErrorDoc, FieldDoc, HttpMethod, TOKEN_VARIABLE, VERSION, common_errors, endpoints,
    example_problem, groups,
};
use crate::api::scope::ApiScope;
use crate::api::v1::docs::openapi_document;
use crate::app::page_title;
use crate::components::badge::badge;
use crate::components::button::{ButtonVariant, button};
use crate::components::code_block::{code_block, code_panel};
use crate::components::notice::notice;
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;
use crate::layout::docs_sidebar::{DocsNavGroup, DocsNavLink, docs_sidebar};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::rate_limit::{
    API_CALL_BURST, API_CALLS_PER_MINUTE, BAD_API_TOKEN_WINDOW_MINUTES, BAD_API_TOKENS_PER_WINDOW,
};
use crate::syntax::CodeLanguage;
use crate::web::config;

const GUIDES: &[(&str, &str)] = &[
    ("Overview", "#overview"),
    ("Authentication", "#authentication"),
    ("Scopes", "#scopes"),
    ("Errors", "#errors"),
    ("Limits", "#limits"),
];

fn method_tone(method: HttpMethod) -> Tone {
    match method {
        HttpMethod::Get => Tone::Success,
        HttpMethod::Patch => Tone::Warning,
    }
}

fn code_method_class(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "p-api-docs__code-method p-api-docs__code-method--get",
        HttpMethod::Patch => "p-api-docs__code-method p-api-docs__code-method--patch",
    }
}

fn implied_by(scope: ApiScope) -> Option<String> {
    let wider: Vec<&str> = ApiScope::ALL
        .into_iter()
        .filter(|other| *other != scope && other.implies(scope))
        .map(ApiScope::as_str)
        .collect();
    (!wider.is_empty()).then(|| format!("Also granted by {}.", wider.join(", ")))
}

#[page("/docs/api", with(Metadata::new().title(page_title("API reference"))))]
pub async fn api_docs_page(cx: &Cx) -> Result<impl View> {
    let origin = config(cx).origin().to_string();
    let tokens_url = AccountSection::ApiTokens.href(cx);
    let openapi_url = href!(openapi_document).resolve(cx);
    let endpoints = endpoints();
    let groups = groups(&endpoints);
    let common = common_errors();
    let version = format!("Version {VERSION}");
    let scope_rows: Vec<(ApiScope, Option<String>)> = ApiScope::ALL
        .into_iter()
        .map(|scope| (scope, implied_by(scope)))
        .collect();
    let first_request = format!(
        "export {TOKEN_VARIABLE}=wsk_…\n\ncurl {origin}/api/v1/me \\\n  -H \"Authorization: Bearer ${TOKEN_VARIABLE}\""
    );
    let problem = example_problem();
    let mut nav_groups = vec![DocsNavGroup {
        name: "Getting started".to_string(),
        links: GUIDES
            .iter()
            .map(|(label, href)| DocsNavLink::new(*href, *label))
            .collect(),
    }];
    nav_groups.extend(groups.iter().map(|(group, members)| {
        DocsNavGroup {
            name: group.to_string(),
            links: members
                .iter()
                .map(|link| {
                    DocsNavLink::new(link.href.as_str(), link.summary)
                        .with_badge(link.method.as_str(), method_tone(link.method))
                })
                .collect(),
        }
    }));
    let call_limit = format!(
        "{API_CALLS_PER_MINUTE} requests a minute per token, in bursts of up to {API_CALL_BURST}."
    );
    let bad_token_limit = format!(
        "{BAD_API_TOKENS_PER_WINDOW} invalid tokens from one address in {BAD_API_TOKEN_WINDOW_MINUTES} minutes blocks that address for {BAD_API_TOKEN_WINDOW_MINUTES} minutes."
    );
    let body_limit = format!("Request bodies can be at most {MAX_JSON_BODY_KB} KB.");

    Ok(view! {
        <div class="l-docs p-api-docs">
            docs_sidebar(
                title: "API reference",
                subtitle: &version,
                nav_label: "API reference",
                groups: &nav_groups
            )

            <div class="l-docs__main">
                <section class="l-docs__section l-docs__section--first" id="overview">
                    <div class="l-docs__prose">
                        page_heading(
                            text: "API reference",
                            variant: PageHeadingVariant::Compact
                        )
                        <p class="p-api-docs__lead">
                            "Use your account from your own scripts, apps and AI assistants. Every endpoint takes and returns JSON, and this page is generated from the same code the app runs, so it's always up to date."
                        </p>
                        <div class="p-api-docs__actions">
                            button(href: Some(&tokens_url), "Create API token")
                            button(
                                variant: ButtonVariant::Secondary,
                                href: Some(&openapi_url),
                                "OpenAPI spec (JSON)"
                            )
                        </div>
                    </div>
                    <div class="l-docs__aside">
                        code_panel(
                            title: "Endpoints",
                            <ul class="p-api-docs__endpoint-list">
                                #[key(endpoint.id)]
                                for endpoint in &endpoints {
                                    <li>
                                        <a
                                            class="p-api-docs__endpoint-link"
                                            href=(format!("#{}", endpoint.id))
                                        >
                                            <span class=(code_method_class(endpoint.method))>
                                                (endpoint.method.as_str())
                                            </span>
                                            <span class="p-api-docs__endpoint-path">
                                                (endpoint.path)
                                            </span>
                                            <span class="p-api-docs__endpoint-summary">
                                                (endpoint.summary)
                                            </span>
                                        </a>
                                    </li>
                                }
                            </ul>
                        )
                    </div>
                </section>

                <section class="l-docs__section" id="authentication">
                    <div class="l-docs__prose">
                        <h2 class="p-api-docs__heading">"Authentication"</h2>
                        <p>
                            "Create a token under "
                            <a href=(tokens_url.as_str())>
                                "Account → Developer → API tokens"
                            </a>
                            " and send it in the "
                            <code>"Authorization"</code>
                            " header of every request. Tokens start with "
                            <code>"wsk_"</code>
                            ", are shown once, and expire on the date you choose."
                        </p>
                        <p>
                            "Base URL: "
                            <code>(origin.as_str())</code>
                        </p>
                    </div>
                    <div class="l-docs__aside">
                        code_panel(
                            title: "Your first request",
                            code_block(
                                code: &first_request,
                                language: CodeLanguage::Shell
                            )
                        )
                    </div>
                </section>

                <section class="l-docs__section l-docs__section--single" id="scopes">
                    <div class="l-docs__prose">
                        <h2 class="p-api-docs__heading">"Scopes"</h2>
                        <p>"A token can only do what its scopes allow."</p>
                        notice(
                            tone: Tone::Info,
                            "Changing your password, email, passkeys or API tokens is never possible with a token."
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
                        error_list(errors: &common)
                    </div>
                    <div class="l-docs__aside">
                        code_panel(
                            title: "Example error",
                            code_block(code: &problem, language: CodeLanguage::Json)
                        )
                    </div>
                </section>

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

                #[key(endpoint.id)]
                for endpoint in &endpoints {
                    endpoint_section(endpoint: endpoint, origin: &origin)
                }
            </div>
        </div>
    })
}

#[component]
async fn endpoint_section(cx: &Cx, endpoint: &Endpoint, origin: &str) -> Result<impl View> {
    let request_fields = endpoint.request_fields();
    let response_fields = endpoint.response_fields();
    let curl = endpoint.curl(origin);
    let response = endpoint.example_response_text();
    let request_title = format!("{} {}", endpoint.method.as_str(), endpoint.path);
    let heading_id = format!("{}-heading", endpoint.id);

    Ok(view! {
        <section
            class="l-docs__section"
            id=(endpoint.id)
            aria-labelledby=(heading_id.as_str())
        >
            <div class="l-docs__prose">
                <h2 class="p-api-docs__heading" id=(heading_id.as_str())>
                    (endpoint.summary)
                </h2>
                <p class="p-api-docs__route">
                    badge(
                        tone: method_tone(endpoint.method),
                        (endpoint.method.as_str())
                    )
                    <span class="p-api-docs__route-path">(endpoint.path)</span>
                </p>
                <p>(endpoint.description)</p>
                <p class="p-api-docs__scope">
                    "Requires the "
                    <code>(endpoint.scope.as_str())</code>
                    " scope."
                </p>
                <h3 class="p-api-docs__subheading">"Request body"</h3>
                if request_fields.is_empty() {
                    <p class="p-api-docs__none">"None."</p>
                } else {
                    field_list(fields: &request_fields)
                }
                <h3 class="p-api-docs__subheading">"Response"</h3>
                field_list(fields: &response_fields)
                <h3 class="p-api-docs__subheading">"Errors"</h3>
                error_list(errors: &endpoint.errors)
            </div>
            <div class="l-docs__aside">
                code_panel(
                    title: &request_title,
                    label: Some("cURL"),
                    code_title: true,
                    code_block(code: &curl, language: CodeLanguage::Shell)
                )
                code_panel(
                    title: "Response",
                    label: Some("JSON"),
                    code_block(code: &response, language: CodeLanguage::Json)
                )
            </div>
        </section>
    })
}

fn field_tags(field: &FieldDoc) -> Vec<PropertyTag> {
    let mut tags = vec![PropertyTag::new(field.kind.as_str(), Tone::Neutral)];
    if field.required {
        tags.push(PropertyTag::new("required", Tone::Error));
    }
    if !field.rules.is_empty() {
        tags.push(PropertyTag::new(field.rules.as_str(), Tone::Neutral));
    }
    tags
}

#[component]
async fn field_list(cx: &Cx, fields: &[FieldDoc]) -> Result<impl View> {
    let rows: Vec<(&FieldDoc, Vec<PropertyTag>)> = fields
        .iter()
        .map(|field| (field, field_tags(field)))
        .collect();

    Ok(view! {
        property_list(
            #[key(field.name.as_str())]
            for (field, tags) in &rows {
                property(
                    name: field.name.as_str(),
                    code: true,
                    tags: tags,
                    description: Some(field.description.as_str())
                )
            }
        )
    })
}

#[component]
async fn error_list(cx: &Cx, errors: &[ErrorDoc]) -> Result<impl View> {
    let rows: Vec<(&ErrorDoc, Vec<PropertyTag>)> = errors
        .iter()
        .map(|error| {
            (
                error,
                vec![PropertyTag::new(error.status.to_string(), Tone::Neutral)],
            )
        })
        .collect();

    Ok(view! {
        property_list(
            #[key(error.code)]
            for (error, tags) in &rows {
                property(
                    name: error.code,
                    code: true,
                    tags: tags,
                    description: Some(error.message)
                )
            }
        )
    })
}
