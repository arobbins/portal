use std::time::Duration;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use topcoat::{
    Error, Result,
    context::Cx,
    router::{StatusCode, content::Json, request::client_ip, route},
};
use tower_sessions::Session;
use uuid::Uuid;
use webauthn_rs::prelude::{
    Credential, CredentialID, DiscoverableAuthentication, Passkey, PasskeyRegistration,
    PublicKeyCredential, RegisterPublicKeyCredential, Url, Webauthn, WebauthnBuilder,
};
use webauthn_rs_core::proto::ResidentKeyRequirement;

use crate::app::SITE_NAME;
use crate::config::Config;
use crate::flash::{Flash, set_flash};
use crate::user::auth::{current_user, password_matches, sign_in};
use crate::user::model::{AccountStatus, User};
use crate::user::queries::{
    NewPasskey, query_find_passkey, query_get_user_by_webauthn_id, query_insert_passkey,
    query_list_passkeys, query_record_passkey_use,
};
use crate::user::service::passkey_added_email;
use crate::web::{app_url, config, db, mailer, rate_limits, session, webauthn};

const CEREMONY_LIFETIME: Duration = Duration::from_secs(10 * 60);
const REGISTRATION_STATE: &str = "passkey_registration";
const AUTHENTICATION_STATE: &str = "passkey_authentication";
const AAGUID_START: usize = 37;
const AAGUID_END: usize = 53;

const SESSION_EXPIRED: &str = "Your session has expired. Please log in again.";
const WRONG_PASSWORD: &str = "Your current password is incorrect.";
const TOO_MANY_ATTEMPTS: &str = "Too many attempts. Please wait a few minutes and try again.";
const CEREMONY_EXPIRED: &str = "That passkey request expired. Please try again.";
const NOT_VERIFIED: &str = "We couldn't verify that passkey. Please try again.";
const UNKNOWN_PASSKEY: &str =
    "That passkey isn't registered with this site anymore. Sign in another way.";

const PROVIDERS: &[(&str, &str)] = &[
    ("fbfc3007-154e-4ecc-8c0b-6e020557d7bd", "iCloud Keychain"),
    (
        "ea9b8d66-4d01-1d21-3ce4-b6b48cb575d4",
        "Google Password Manager",
    ),
    ("adce0002-35bc-c60a-648b-0b25f1f05503", "Chrome on Mac"),
    ("08987058-cadc-4b81-b6e1-30de50dcbe96", "Windows Hello"),
    ("9ddd1817-af5a-4672-a2b9-3e3dd95000a9", "Windows Hello"),
    ("6028b017-b1d4-4c02-b4b3-afcdafc96bb2", "Windows Hello"),
    ("bada5566-a7aa-401f-bd96-45619a55120d", "1Password"),
    ("d548826e-79b4-db40-a3d8-11116f7e8349", "Bitwarden"),
    ("531126d6-e717-415c-9320-3d9aa6981239", "Dashlane"),
    ("50726f74-6f6e-5061-7373-50726f746f6e", "Proton Pass"),
    ("fdb141b2-5d84-443e-8a35-4698c205a502", "KeePassXC"),
    ("53414d53-554e-4700-0000-000000000000", "Samsung Pass"),
];

pub fn relying_party(config: &Config) -> Webauthn {
    let origin = Url::parse(config.origin()).expect("Config validated APP_BASE_URL");

    WebauthnBuilder::new(&config.passkey_domain, &origin)
        .and_then(|builder| {
            builder
                .rp_name(SITE_NAME)
                .timeout(CEREMONY_LIFETIME)
                .build()
        })
        .expect("Config validated the passkey domain and origin")
}

pub fn provider_name(aaguid: Option<Uuid>) -> &'static str {
    aaguid
        .and_then(|aaguid| {
            let aaguid = aaguid.to_string();
            PROVIDERS
                .iter()
                .find(|(id, _)| *id == aaguid)
                .map(|(_, name)| *name)
        })
        .unwrap_or("Passkey")
}

fn aaguid_from_attestation(attestation_object: &[u8]) -> Option<Uuid> {
    let value: serde_cbor_2::Value = serde_cbor_2::from_slice(attestation_object).ok()?;
    let serde_cbor_2::Value::Map(entries) = value else {
        return None;
    };
    let auth_data = entries.iter().find_map(|(key, value)| match (key, value) {
        (serde_cbor_2::Value::Text(key), serde_cbor_2::Value::Bytes(bytes))
            if key == "authData" =>
        {
            Some(bytes)
        }
        _ => None,
    })?;
    let aaguid = Uuid::from_slice(auth_data.get(AAGUID_START..AAGUID_END)?).ok()?;

    (!aaguid.is_nil()).then_some(aaguid)
}

#[derive(Serialize, Deserialize)]
struct Ceremony<T> {
    state: T,
    started_at: i64,
}

async fn store_ceremony<T: Serialize>(session: &Session, key: &str, state: T) -> Result<()> {
    let ceremony = Ceremony {
        state,
        started_at: time::OffsetDateTime::now_utc().unix_timestamp(),
    };
    session.insert(key, ceremony).await?;
    Ok(())
}

async fn take_ceremony<T: DeserializeOwned>(session: &Session, key: &str) -> Result<Option<T>> {
    let Some(ceremony) = session.remove::<Ceremony<T>>(key).await? else {
        return Ok(None);
    };
    let age = time::OffsetDateTime::now_utc().unix_timestamp() - ceremony.started_at;

    Ok((0..=CEREMONY_LIFETIME.as_secs() as i64)
        .contains(&age)
        .then_some(ceremony.state))
}

fn reply(status: StatusCode, body: Value) -> Result<(StatusCode, Json<Value>)> {
    Ok((status, Json(body)))
}

fn failure(status: StatusCode, message: &str) -> Result<(StatusCode, Json<Value>)> {
    reply(status, json!({ "error": message }))
}

pub fn encode_binary(bytes: &[u8]) -> Value {
    serde_json::to_value(CredentialID::from(bytes.to_vec())).unwrap_or(Value::Null)
}

pub async fn accepted_credentials(user: &User, cx: &Cx) -> Result<Value> {
    let passkeys = query_list_passkeys(user.id, db(cx)).await?;
    let ids: Vec<Value> = passkeys
        .iter()
        .map(|passkey| encode_binary(&passkey.credential_id))
        .collect();

    Ok(json!({
        "rpId": config(cx).passkey_domain,
        "userId": encode_binary(user.webauthn_id.as_bytes()),
        "allAcceptedCredentialIds": ids,
        "name": user.email,
        "displayName": user.name,
    }))
}

#[derive(Deserialize)]
pub struct RegistrationStart {
    current_password: String,
}

#[route(POST "/passkeys/register/start")]
pub async fn start_registration(
    cx: &Cx,
    Json(input): Json<RegistrationStart>,
) -> Result<(StatusCode, Json<Value>)> {
    let Some(user) = current_user(cx) else {
        return failure(StatusCode::UNAUTHORIZED, SESSION_EXPIRED);
    };
    let limits = rate_limits(cx);
    let ip = client_ip(cx);

    if !limits.allow_password_submission(ip) || limits.login_blocked(&user.email, ip) {
        return failure(StatusCode::TOO_MANY_REQUESTS, TOO_MANY_ATTEMPTS);
    }

    let password_hash = match user.status() {
        AccountStatus::Active { password_hash } => Some(password_hash),
        AccountStatus::AwaitingPassword => None,
    };
    if !password_matches(password_hash, &input.current_password) {
        limits.record_failed_login(&user.email, ip);
        return reply(
            StatusCode::BAD_REQUEST,
            json!({ "error": WRONG_PASSWORD, "field": "current_password" }),
        );
    }

    let existing: Vec<CredentialID> = query_list_passkeys(user.id, db(cx))
        .await?
        .iter()
        .map(|passkey| passkey.passkey.cred_id().clone())
        .collect();
    let (mut options, state) = webauthn(cx)
        .start_passkey_registration(user.webauthn_id, &user.email, &user.name, Some(existing))
        .map_err(|e| Error::msg(e.to_string()))?;
    if let Some(selection) = options.public_key.authenticator_selection.as_mut() {
        selection.resident_key = Some(ResidentKeyRequirement::Required);
        selection.require_resident_key = true;
    }

    store_ceremony::<PasskeyRegistration>(&session(cx)?, REGISTRATION_STATE, state).await?;

    reply(
        StatusCode::OK,
        json!({ "options": options, "origin": config(cx).origin() }),
    )
}

#[route(POST "/passkeys/register/finish")]
pub async fn finish_registration(
    cx: &Cx,
    Json(credential): Json<RegisterPublicKeyCredential>,
) -> Result<(StatusCode, Json<Value>)> {
    let Some(user) = current_user(cx) else {
        return failure(StatusCode::UNAUTHORIZED, SESSION_EXPIRED);
    };
    let session = session(cx)?;
    let Some(state) = take_ceremony::<PasskeyRegistration>(&session, REGISTRATION_STATE).await?
    else {
        return failure(StatusCode::BAD_REQUEST, CEREMONY_EXPIRED);
    };

    let Ok(passkey) = webauthn(cx).finish_passkey_registration(&credential, &state) else {
        return failure(StatusCode::BAD_REQUEST, NOT_VERIFIED);
    };
    let details = Credential::from(passkey.clone());
    let aaguid = aaguid_from_attestation(credential.response.attestation_object.as_ref());
    let name = provider_name(aaguid);

    query_insert_passkey(
        &NewPasskey {
            user_id: user.id,
            passkey: &passkey,
            name,
            aaguid,
            backup_eligible: details.backup_eligible,
            backed_up: details.backup_state,
        },
        db(cx),
    )
    .await?;

    let security_url = app_url(cx, "/account/security#passkeys");
    if let Err(e) = mailer(cx)
        .send(&passkey_added_email(&user.email, name, &security_url))
        .await
    {
        tracing::error!("Failed to send passkey-added email: {}", e);
    }
    set_flash(&session, Flash::PasskeyAdded).await?;

    reply(StatusCode::OK, json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct AuthenticationStart {
    #[serde(default)]
    conditional: bool,
}

#[route(POST "/passkeys/login/start")]
pub async fn start_authentication(
    cx: &Cx,
    Json(input): Json<AuthenticationStart>,
) -> Result<(StatusCode, Json<Value>)> {
    if !rate_limits(cx).allow_passkey_challenge(client_ip(cx)) {
        return failure(StatusCode::TOO_MANY_REQUESTS, TOO_MANY_ATTEMPTS);
    }

    let (mut options, state) = webauthn(cx)
        .start_discoverable_authentication()
        .map_err(|e| Error::msg(e.to_string()))?;
    if !input.conditional {
        options.mediation = None;
    }

    store_ceremony::<DiscoverableAuthentication>(&session(cx)?, AUTHENTICATION_STATE, state)
        .await?;

    reply(
        StatusCode::OK,
        json!({ "options": options, "origin": config(cx).origin() }),
    )
}

#[route(POST "/passkeys/login/finish")]
pub async fn finish_authentication(
    cx: &Cx,
    Json(credential): Json<PublicKeyCredential>,
) -> Result<(StatusCode, Json<Value>)> {
    if !rate_limits(cx).allow_password_submission(client_ip(cx)) {
        return failure(StatusCode::TOO_MANY_REQUESTS, TOO_MANY_ATTEMPTS);
    }
    let session = session(cx)?;
    let Some(state) =
        take_ceremony::<DiscoverableAuthentication>(&session, AUTHENTICATION_STATE).await?
    else {
        return failure(StatusCode::BAD_REQUEST, CEREMONY_EXPIRED);
    };

    let Ok((webauthn_id, credential_id)) =
        webauthn(cx).identify_discoverable_authentication(&credential)
    else {
        return failure(StatusCode::BAD_REQUEST, NOT_VERIFIED);
    };
    let unknown = || {
        reply(
            StatusCode::BAD_REQUEST,
            json!({ "error": UNKNOWN_PASSKEY, "unknownCredential": encode_binary(credential_id) }),
        )
    };

    let Some(user) = query_get_user_by_webauthn_id(webauthn_id, db(cx)).await? else {
        return unknown();
    };
    if !matches!(user.status(), AccountStatus::Active { .. }) {
        return unknown();
    }
    let Some(stored) = query_find_passkey(user.id, credential_id, db(cx)).await? else {
        return unknown();
    };

    let mut passkey: Passkey = stored.passkey.0;
    let Ok(result) =
        webauthn(cx).finish_discoverable_authentication_v2(&credential, state, &passkey)
    else {
        return failure(StatusCode::BAD_REQUEST, NOT_VERIFIED);
    };
    passkey.update_credential(&result);
    query_record_passkey_use(stored.id, &passkey, result.backup_state(), db(cx)).await?;

    sign_in(&session, &user).await?;
    let mut body = accepted_credentials(&user, cx).await?;
    body["redirect"] = json!("/account");

    reply(StatusCode::OK, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_providers_get_a_name_and_others_a_default() {
        let icloud = Uuid::parse_str("fbfc3007-154e-4ecc-8c0b-6e020557d7bd").ok();
        assert_eq!(provider_name(icloud), "iCloud Keychain");
        assert_eq!(provider_name(Some(Uuid::new_v4())), "Passkey");
        assert_eq!(provider_name(None), "Passkey");
    }

    #[test]
    fn reads_the_aaguid_from_authenticator_data() {
        let aaguid = Uuid::new_v4();
        let mut auth_data = vec![0u8; AAGUID_START];
        auth_data.extend_from_slice(aaguid.as_bytes());
        auth_data.extend_from_slice(&[0, 0]);
        let object = serde_cbor_2::Value::Map(
            [(
                serde_cbor_2::Value::Text("authData".to_string()),
                serde_cbor_2::Value::Bytes(auth_data),
            )]
            .into_iter()
            .collect(),
        );
        let bytes = serde_cbor_2::to_vec(&object).unwrap();

        assert_eq!(aaguid_from_attestation(&bytes), Some(aaguid));
    }

    #[test]
    fn an_all_zero_aaguid_means_unknown() {
        let mut auth_data = vec![0u8; AAGUID_END + 2];
        auth_data[0] = 1;
        let object = serde_cbor_2::Value::Map(
            [(
                serde_cbor_2::Value::Text("authData".to_string()),
                serde_cbor_2::Value::Bytes(auth_data),
            )]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            aaguid_from_attestation(&serde_cbor_2::to_vec(&object).unwrap()),
            None
        );
    }
}
