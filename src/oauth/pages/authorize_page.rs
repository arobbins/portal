use std::collections::HashMap;

use axum::http::{HeaderValue, header::CACHE_CONTROL};
use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        Body,
        error::{SeeOther, see_other},
        page,
        request::{self, Bytes, FromRequest},
        response::response_headers,
        route,
    },
    view::{View, ViewExt, view},
};

use super::approval_form::{Approval, approval_form};
use super::cant_connect::cant_connect;
use crate::action::{ActionResult, Caller, ID_FIELD, page_status, require_browser_caller};
use crate::api::scope::ApiScope;
use crate::app::page_title;
use crate::auth::handoff::{Handoff, with_handoff};
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::auth::reauth::{return_passkey_proof, take_passkey_proof};
use crate::auth::session::current_user;
use crate::oauth::actions::{ConnectApp, ConnectProblem, connect_app, deny_app};
use crate::oauth::authorize::{
    AuthorizeParams, CantConnect, REQUEST_PARAM, Started, keep_request_alive, live_request,
    request_path, start_authorization,
};
use crate::oauth::queries::RequestRow;
use crate::oauth::redirect::return_host;
use crate::routes::LOGIN_PATH;
use crate::web::{env, redisplay, session};

pub const SCOPE_FIELD: &str = "scope";
pub const DECISION_FIELD: &str = "decision";
pub const DENY: &str = "deny";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Approve,
    Deny,
}

impl Decision {
    fn of(value: &str) -> Self {
        if value == DENY {
            Self::Deny
        } else {
            Self::Approve
        }
    }
}

struct ConnectFailed {
    problem: ConnectProblem,
}

fn query_params(cx: &Cx) -> HashMap<String, String> {
    let query = request::uri(cx).query().unwrap_or_default();
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}

#[page("/oauth/authorize", with(Metadata::new().title(page_title("Connect an app"))))]
pub async fn authorize_page(cx: &Cx) -> Result<impl View> {
    response_headers(cx).append(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    let env = env(cx);
    let mut query = query_params(cx);
    let Some(request_id) = query.remove(REQUEST_PARAM) else {
        let params: AuthorizeParams =
            serde_json::from_value(serde_json::to_value(query)?).unwrap_or_default();
        return match start_authorization(&env, &params).await? {
            Started::Request(id) => Err(see_other(request_path(&id)).into()),
            Started::ReturnError(url) => Err(see_other(url).into()),
            Started::Refused(reason) => Ok(view! { cant_connect(reason: reason) }.boxed()),
        };
    };
    let Some(request) = live_request(&env, &request_id).await? else {
        return Ok(view! { cant_connect(reason: CantConnect::Expired) }.boxed());
    };
    if current_user(cx).is_none() {
        let handoff = Handoff::Authorize(request.public_id.clone());
        return Err(see_other(with_handoff(LOGIN_PATH, Some(&handoff))).into());
    }
    keep_request_alive(&env, &request.public_id).await?;
    let problem = try_context::<ConnectFailed>(cx).map(|failed| failed.problem);
    let approval = approval_of(&request);

    Ok(view! {
        if let Some(problem) = problem {
            (page_status(problem))
        }
        approval_form(approval: &approval, problem: problem)
    }
    .boxed())
}

fn approval_of(request: &RequestRow) -> Approval {
    Approval {
        request: request.public_id.clone(),
        name: request.client_name.clone(),
        return_host: return_host(&request.redirect_uri),
        scopes: request.scopes.clone(),
    }
}

#[route(POST "/oauth/authorize")]
pub async fn authorize_submit(cx: &Cx, body: Body) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let bytes = Bytes::from_request(cx, body).await.unwrap_or_default();
    let fields: Vec<(String, String)> = url::form_urlencoded::parse(&bytes).into_owned().collect();
    let value = |name: &str| {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    let request = value(ID_FIELD);
    let outcome = if Decision::of(&value(DECISION_FIELD)) == Decision::Deny {
        deny_app(&env(cx), &caller, &request).await?
    } else {
        let scopes = fields
            .iter()
            .filter(|(key, _)| key == SCOPE_FIELD)
            .filter_map(|(_, scope)| ApiScope::from_str(scope))
            .collect();
        let current_password = value(CURRENT_PASSWORD_FIELD);
        let input = ConnectApp {
            request: request.clone(),
            scopes,
            current_password,
        };
        connect(cx, &caller, &input).await?
    };
    match outcome {
        Ok(redirect) => Ok(see_other(redirect)),
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(request_path(&request), ConnectFailed { problem }))
        }
    }
}

async fn connect(
    cx: &Cx,
    caller: &Caller,
    input: &ConnectApp,
) -> ActionResult<String, ConnectProblem> {
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;
    let outcome = connect_app(&env, caller, input, passkey).await?;
    if outcome.is_err() {
        return_passkey_proof(&session, passkey).await?;
    }
    Ok(outcome)
}
