use serde::{Deserialize, Serialize};
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, Outcome, ProblemSpec, fail};
use crate::env::Env;
use crate::oauth::metadata_document::fetch_client_metadata;
use crate::oauth::queries::{NewClient, OAuthClient, query_get_client, query_insert_client};
use crate::oauth::redirect::{MAX_REDIRECT_URIS, is_allowed_redirect, return_host};
use crate::secret::PublicIdKind;
use crate::user::name::check_name;

const METADATA_CACHE: time::Duration = time::Duration::hours(1);
const PUBLIC_CLIENT: &str = "none";
const GRANT_TYPES: [&str; 2] = ["authorization_code", "refresh_token"];
const RESPONSE_TYPES: [&str; 1] = ["code"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationProblem {
    InvalidRedirectUri,
    InvalidMetadata,
    TooManyRegistrations,
}

impl ActionProblem for RegistrationProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::InvalidRedirectUri => ProblemSpec::new(
                "invalid_redirect_uri",
                "Send 1 to 10 redirect_uris, each https or http on a loopback host (127.0.0.1, [::1] or localhost), without a fragment.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::InvalidMetadata => ProblemSpec::new(
                "invalid_client_metadata",
                "Only public clients can register: token_endpoint_auth_method \"none\", grant_types authorization_code and refresh_token, response_types code, and a client_name of at most 100 characters.",
            )
            .with_status(StatusCode::BAD_REQUEST),
            Self::TooManyRegistrations => ProblemSpec::new(
                "too_many_requests",
                "Too many clients registered from this address. Try again in an hour.",
            )
            .with_status(StatusCode::TOO_MANY_REQUESTS),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ClientMetadata {
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub token_endpoint_auth_method: Option<String>,
    #[serde(default)]
    pub grant_types: Option<Vec<String>>,
    #[serde(default)]
    pub response_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisteredClient {
    pub client_id: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub token_endpoint_auth_method: &'static str,
    pub grant_types: [&'static str; 2],
    pub response_types: [&'static str; 1],
    pub client_id_issued_at: i64,
}

pub struct CheckedClient {
    pub name: String,
    pub redirect_uris: Vec<String>,
}

pub fn check_metadata(metadata: &ClientMetadata) -> Result<CheckedClient, RegistrationProblem> {
    let uris = &metadata.redirect_uris;
    if uris.is_empty()
        || uris.len() > MAX_REDIRECT_URIS
        || !uris.iter().all(|uri| is_allowed_redirect(uri))
    {
        return Err(RegistrationProblem::InvalidRedirectUri);
    }
    let public = metadata
        .token_endpoint_auth_method
        .as_deref()
        .is_none_or(|method| method == PUBLIC_CLIENT);
    let grants = only(&metadata.grant_types, &GRANT_TYPES);
    let responses = only(&metadata.response_types, &RESPONSE_TYPES);
    if !(public && grants && responses) {
        return Err(RegistrationProblem::InvalidMetadata);
    }
    let fallback = return_host(&uris[0]);
    let name = match metadata.client_name.as_deref() {
        Some(name) if name.chars().any(char::is_control) => {
            return Err(RegistrationProblem::InvalidMetadata);
        }
        Some(name) => check_name(name).map_err(|_| RegistrationProblem::InvalidMetadata)?,
        None => fallback.as_str(),
    };
    Ok(CheckedClient {
        name: name.to_string(),
        redirect_uris: uris.clone(),
    })
}

fn only(given: &Option<Vec<String>>, allowed: &[&str]) -> bool {
    given
        .as_ref()
        .is_none_or(|values| values.iter().all(|value| allowed.contains(&value.as_str())))
}

pub async fn register_client(
    env: &Env<'_>,
    metadata: &ClientMetadata,
) -> Outcome<RegisteredClient, RegistrationProblem> {
    if !env.limits().allow_client_registration(env.ip).await? {
        return fail(RegistrationProblem::TooManyRegistrations);
    }
    let client = match check_metadata(metadata) {
        Ok(client) => client,
        Err(problem) => return fail(problem),
    };
    let client_id = PublicIdKind::OAuthClient.generate();
    let row = NewClient {
        client_id: &client_id,
        name: &client.name,
        redirect_uris: &client.redirect_uris,
        fetched_at: None,
    };
    query_insert_client(&row, env.db()).await?;

    Ok(Ok(RegisteredClient {
        client_id,
        client_name: client.name,
        redirect_uris: client.redirect_uris,
        token_endpoint_auth_method: PUBLIC_CLIENT,
        grant_types: GRANT_TYPES,
        response_types: RESPONSE_TYPES,
        client_id_issued_at: env.now().unix_timestamp(),
    }))
}

pub async fn find_client(env: &Env<'_>, client_id: &str) -> topcoat::Result<Option<OAuthClient>> {
    if PublicIdKind::OAuthClient.is_valid(client_id) {
        return Ok(query_get_client(client_id, env.db()).await?);
    }
    if !client_id.starts_with("https://") {
        return Ok(None);
    }
    let cached = query_get_client(client_id, env.db()).await?;
    let fresh = cached
        .as_ref()
        .and_then(|client| client.fetched_at)
        .is_some_and(|at| env.now() - at < METADATA_CACHE);
    if fresh {
        return Ok(cached);
    }
    let Some(client) = fetch_client_metadata(client_id).await else {
        return Ok(cached);
    };
    let row = NewClient {
        client_id,
        name: &client.name,
        redirect_uris: &client.redirect_uris,
        fetched_at: Some(env.now()),
    };
    query_insert_client(&row, env.db()).await?;
    Ok(query_get_client(client_id, env.db()).await?)
}

#[cfg(test)]
#[test]
fn registration_messages_matches_the_limit() {
    let redirects = RegistrationProblem::InvalidRedirectUri.message();
    let metadata = RegistrationProblem::InvalidMetadata.message();
    assert!(redirects.contains(&format!("1 to {MAX_REDIRECT_URIS} redirect_uris")));
    let name_limit = crate::user::name::MAX_NAME_LENGTH;
    assert!(metadata.contains(&format!("at most {name_limit} characters")));
}

#[cfg(test)]
mod tests;
