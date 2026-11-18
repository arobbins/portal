mod endpoints;
mod guides;

use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{href, page},
    view::{View, view},
};

use crate::account::tabs::AccountSection;
use crate::api::catalog::{Endpoint, HttpMethod, VERSION, common_errors, endpoints, groups};
use crate::api::v1::docs::openapi_document;
use crate::app::page_title;
use crate::components::tone::Tone;
use crate::layout::docs_sidebar::{DocsNavGroup, DocsNavLink, docs_sidebar};
use crate::web::config;
use endpoints::endpoint_section;
use guides::{
    authentication_section, errors_section, limits_section, overview_section, scopes_section,
};

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
        HttpMethod::Post => Tone::Info,
        HttpMethod::Patch => Tone::Warning,
    }
}

fn code_method_class(method: HttpMethod) -> &'static str {
    match method {
        HttpMethod::Get => "p-api-docs__code-method p-api-docs__code-method--get",
        HttpMethod::Post => "p-api-docs__code-method p-api-docs__code-method--post",
        HttpMethod::Patch => "p-api-docs__code-method p-api-docs__code-method--patch",
    }
}

fn nav_groups(endpoints: &[Endpoint]) -> Vec<DocsNavGroup> {
    let guides = DocsNavGroup {
        name: "Getting started".to_string(),
        links: GUIDES
            .iter()
            .map(|(label, href)| DocsNavLink::new(*href, *label))
            .collect(),
    };
    let reference = groups(endpoints)
        .into_iter()
        .map(|(group, members)| DocsNavGroup {
            name: group.to_string(),
            links: members
                .iter()
                .map(|link| {
                    DocsNavLink::new(link.href.as_str(), link.summary)
                        .with_badge(link.method.as_str(), method_tone(link.method))
                })
                .collect(),
        });
    std::iter::once(guides).chain(reference).collect()
}

#[page("/docs/api", with(Metadata::new().title(page_title("API reference"))))]
pub async fn api_docs_page(cx: &Cx) -> Result<impl View> {
    let origin = config(cx).origin().to_string();
    let tokens_url = AccountSection::ApiTokens.path();
    let openapi_url = href!(openapi_document).resolve(cx);
    let endpoints = endpoints();
    let nav_groups = nav_groups(&endpoints);
    let common = common_errors();
    let version = format!("Version {VERSION}");

    Ok(view! {
        <div class="l-docs p-api-docs">
            docs_sidebar(
                title: "API reference",
                subtitle: &version,
                nav_label: "API reference",
                groups: &nav_groups
            )
            <div class="l-docs__main">
                overview_section(
                    endpoints: &endpoints,
                    tokens_url: &tokens_url,
                    openapi_url: &openapi_url
                )
                authentication_section(origin: &origin, tokens_url: &tokens_url)
                scopes_section()
                errors_section(common: &common)
                limits_section()
                #[key(endpoint.id)]
                for endpoint in &endpoints {
                    endpoint_section(endpoint: endpoint, origin: &origin)
                }
            </div>
        </div>
    })
}
