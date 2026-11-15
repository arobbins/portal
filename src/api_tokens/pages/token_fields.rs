use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use crate::api::scope::{ApiScope, TokenAudience};
use crate::api_tokens::lifetime::TokenLifetime;
use crate::components::form::{Choice, select_field};

const DEFAULT_ACCESS: ApiScope = ApiScope::ProfileRead;

pub(super) struct TokenDrafts {
    pub blank: Signal<String>,
    pub name: Signal<String>,
    pub access: Signal<String>,
    pub lifetime: Signal<String>,
    pub audience: Signal<String>,
    pub password: Signal<String>,
    pub created: Signal<String>,
}

impl TokenDrafts {
    pub fn new(cx: &Cx) -> Self {
        Self {
            blank: signal(cx, String::new),
            name: signal(cx, String::new),
            access: signal(cx, || DEFAULT_ACCESS.as_str().to_string()),
            lifetime: signal(cx, || TokenLifetime::DEFAULT.value().to_string()),
            audience: signal(cx, || TokenAudience::DEFAULT.as_str().to_string()),
            password: signal(cx, String::new),
            created: signal(cx, String::new),
        }
    }
}

fn access_choices() -> Vec<Choice<'static>> {
    ApiScope::ALL
        .into_iter()
        .map(|scope| Choice {
            value: scope.as_str(),
            label: scope.choice_label(),
        })
        .collect()
}

fn audience_choices() -> Vec<Choice<'static>> {
    TokenAudience::ALL
        .into_iter()
        .map(|audience| Choice {
            value: audience.as_str(),
            label: audience.label(),
        })
        .collect()
}

fn lifetime_choices() -> Vec<Choice<'static>> {
    TokenLifetime::ALL
        .into_iter()
        .map(|lifetime| Choice {
            value: lifetime.value(),
            label: lifetime.label(),
        })
        .collect()
}

#[component]
pub(super) async fn choice_fields(cx: &Cx, drafts: &TokenDrafts) -> Result<impl View> {
    let TokenDrafts {
        access,
        lifetime,
        audience,
        ..
    } = drafts;
    let (access_choices, audience_choices, lifetime_choices) =
        (access_choices(), audience_choices(), lifetime_choices());

    Ok(view! {
        select_field(
            label: "Access",
            id: "api-token-access",
            name: "access",
            options: &access_choices,
            selected: Some(DEFAULT_ACCESS.as_str()),
            required: true,
            attrs: attributes! { @change=$(|e: Event| access.set(e.target.value)) }
        )
        select_field(
            label: "Used by",
            id: "api-token-audience",
            name: "audience",
            options: &audience_choices,
            selected: Some(TokenAudience::DEFAULT.as_str()),
            required: true,
            attrs: attributes! { @change=$(|e: Event| audience.set(e.target.value)) }
        )
        select_field(
            label: "Expires after",
            id: "api-token-lifetime",
            name: "lifetime",
            options: &lifetime_choices,
            selected: Some(TokenLifetime::DEFAULT.value()),
            required: true,
            attrs: attributes! { @change=$(|e: Event| lifetime.set(e.target.value)) }
        )
    })
}
