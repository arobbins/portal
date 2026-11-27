use crate::api::scope::ApiScope;
use crate::env::App;
use crate::oauth::authorize::mcp_resource;
use crate::oauth::grant::challenge_of;
use crate::oauth::testing::{REDIRECT_URI, VERIFIER};
use crate::routes::OAUTH_AUTHORIZE_PATH;

pub const TOKEN_PATH: &str = "/oauth/token";
pub const REVOKE_PATH: &str = "/oauth/revoke";

fn encoded(pairs: &[(&str, &str)]) -> String {
    let mut form = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        form.append_pair(key, value);
    }
    form.finish()
}

pub fn start_path(app: &App, client_id: &str, scopes: &[ApiScope]) -> String {
    let scope: Vec<&str> = scopes.iter().map(|scope| scope.as_str()).collect();
    let challenge = challenge_of(VERIFIER);
    let resource = mcp_resource(&app.env(None));
    let query = encoded(&[
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", REDIRECT_URI),
        ("scope", &scope.join(" ")),
        ("state", "state-1"),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("resource", &resource),
    ]);
    format!("{OAUTH_AUTHORIZE_PATH}?{query}")
}

pub fn param_of(location: Option<&str>, name: &str) -> Option<String> {
    let location = location?;
    let url = url::Url::parse(location)
        .or_else(|_| url::Url::parse(&format!("http://localhost{location}")))
        .ok()?;
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

pub fn approve_form(request: &str, scopes: &[ApiScope], password: &str) -> String {
    let mut pairs = vec![("id", request), ("current_password", password)];
    pairs.extend(scopes.iter().map(|scope| ("scope", scope.as_str())));
    encoded(&pairs)
}

pub fn deny_form(request: &str) -> String {
    encoded(&[("id", request), ("decision", "deny")])
}

pub fn code_form(app: &App, client_id: &str, code: &str) -> String {
    let resource = mcp_resource(&app.env(None));
    encoded(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("client_id", client_id),
        ("code_verifier", VERIFIER),
        ("resource", &resource),
    ])
}

pub fn refresh_form(app: &App, client_id: &str, refresh_token: &str) -> String {
    let resource = mcp_resource(&app.env(None));
    encoded(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
        ("resource", &resource),
    ])
}

pub fn revoke_form(token: &str, client_id: &str) -> String {
    encoded(&[("token", token), ("client_id", client_id)])
}
