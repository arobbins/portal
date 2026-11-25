use crate::billing::pages::handoff::checkout_after_sign_in;
use crate::routes::ACCOUNT_PATH;
use serde_json::json;
use topcoat::{
    Result,
    context::Cx,
    router::{Body, BodyLimit, StatusCode, route},
};
use webauthn_rs::prelude::{PublicKeyCredential, RegisterPublicKeyCredential};

use crate::action::{ActionError, ActionProblem, SESSION_EXPIRED, browser_caller};
use crate::api::body::{MAX_JSON_BODY, read_json};
use crate::api::problem::{ApiProblem, ApiResponse, ok};
use crate::auth::session::sign_in;
use crate::devices::lifecycle::SignInMethod;
use crate::flash::{Flash, set_flash};
use crate::passkeys::actions::{
    PasskeySignInProblem, StartRegistration, StartSignIn, finish_passkey_registration,
    finish_passkey_sign_in, finish_passkey_verification, start_passkey_registration,
    start_passkey_sign_in, start_passkey_verification,
};
use crate::passkeys::webauthn::{accepted_credentials, encode_binary};
use crate::web::{app, env, session};

fn session_expired() -> ApiResponse {
    ApiProblem::new(StatusCode::UNAUTHORIZED, "session_expired", SESSION_EXPIRED).response()
}

fn browser_problem<P: ActionProblem>(error: ActionError<P>) -> Result<ApiResponse> {
    Ok(ApiProblem::of(error.for_browser()?).response())
}

#[route(POST "/passkeys/register/start", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn start_registration(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(session_expired());
    };
    let input: StartRegistration = match read_json(cx, body).await? {
        Ok(input) => input,
        Err(response) => return Ok(response),
    };

    match start_passkey_registration(&env(cx), &session(cx)?, &caller, &input).await? {
        Ok(options) => Ok(ok(&options)),
        Err(error) => browser_problem(error),
    }
}

#[route(POST "/passkeys/register/finish", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn finish_registration(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(session_expired());
    };
    let credential: RegisterPublicKeyCredential = match read_json(cx, body).await? {
        Ok(credential) => credential,
        Err(response) => return Ok(response),
    };

    match finish_passkey_registration(&env(cx), &session(cx)?, &caller, &credential).await? {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::PasskeyAdded).await?;
            Ok(ok(&json!({ "ok": true })))
        }
        Err(error) => browser_problem(error),
    }
}

#[route(POST "/passkeys/login/start", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn start_authentication(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let input: StartSignIn = match read_json(cx, body).await? {
        Ok(input) => input,
        Err(response) => return Ok(response),
    };

    Ok(
        match start_passkey_sign_in(&env(cx), &session(cx)?, &input).await? {
            Ok(options) => ok(&options),
            Err(problem) => ApiProblem::of(problem).response(),
        },
    )
}

#[route(POST "/passkeys/login/finish", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn finish_authentication(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let credential: PublicKeyCredential = match read_json(cx, body).await? {
        Ok(credential) => credential,
        Err(response) => return Ok(response),
    };
    let user = match finish_passkey_sign_in(&env(cx), &session(cx)?, &credential).await? {
        Ok(user) => user,
        Err(problem @ PasskeySignInProblem::UnknownPasskey) => {
            return Ok(ApiProblem::of(problem)
                .extra(
                    "unknown_credential",
                    encode_binary(credential.raw_id.as_ref()),
                )
                .response());
        }
        Err(problem) => return Ok(ApiProblem::of(problem).response()),
    };
    sign_in(cx, &session(cx)?, &user, SignInMethod::Passkey).await?;

    let mut body = accepted_credentials(app(cx), &user).await?;
    let checkout = checkout_after_sign_in(cx, &session(cx)?, &user).await?;
    body["redirect"] = json!(checkout.as_deref().unwrap_or(ACCOUNT_PATH));
    Ok(ok(&body))
}

#[route(POST "/passkeys/verify/start", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn start_verification(cx: &Cx) -> Result<ApiResponse> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(session_expired());
    };

    match start_passkey_verification(&env(cx), &session(cx)?, &caller).await? {
        Ok(options) => Ok(ok(&options)),
        Err(error) => browser_problem(error),
    }
}

#[route(POST "/passkeys/verify/finish", with(BodyLimit::max(MAX_JSON_BODY)))]
pub async fn finish_verification(cx: &Cx, body: Body) -> Result<ApiResponse> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(session_expired());
    };
    let credential: PublicKeyCredential = match read_json(cx, body).await? {
        Ok(credential) => credential,
        Err(response) => return Ok(response),
    };

    match finish_passkey_verification(&env(cx), &session(cx)?, &caller, &credential).await? {
        Ok(()) => Ok(ok(&json!({ "ok": true }))),
        Err(error) => browser_problem(error),
    }
}
