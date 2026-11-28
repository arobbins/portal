use serde_json::json;

use super::{
    FeedbackCategory, FeedbackProblem, FeedbackReceipt, MAX_FEEDBACK_LENGTH, SendApiFeedback,
};
use crate::action::{ActionResult, Caller, authorize, fail};
use crate::api::catalog::common_errors;
use crate::api::registry::endpoints;
use crate::env::Env;
use crate::feedback::queries::{NewFeedback, query_insert_api_feedback};
use crate::feedback::redact::redact_secrets;
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};

struct CheckedFeedback {
    endpoint: Option<String>,
    problem_code: Option<String>,
    request_id: Option<String>,
    message: String,
}

pub async fn send_api_feedback(
    env: &Env<'_>,
    caller: &Caller,
    input: &SendApiFeedback,
) -> ActionResult<FeedbackReceipt, FeedbackProblem> {
    authorize!(caller.require(Permission::SendApiFeedback));
    let checked = match check_feedback(input) {
        Ok(checked) => checked,
        Err(problem) => return fail(problem),
    };
    if !env
        .limits()
        .allow_feedback(caller.api_token_id(), caller.user.id)
        .await?
    {
        return fail(FeedbackProblem::TooManyAttempts);
    }
    let id = store_feedback(env, caller, input.category, &checked).await?;

    Ok(Ok(FeedbackReceipt { id }))
}

async fn store_feedback(
    env: &Env<'_>,
    caller: &Caller,
    category: FeedbackCategory,
    checked: &CheckedFeedback,
) -> topcoat::Result<String> {
    let public_id = PublicIdKind::Feedback.generate();
    let mut tx = env.db().begin().await?;
    query_insert_api_feedback(
        &NewFeedback {
            public_id: &public_id,
            user_id: caller.user.id,
            api_token_id: caller.api_token_id(),
            category,
            endpoint: checked.endpoint.as_deref(),
            problem_code: checked.problem_code.as_deref(),
            request_id: checked.request_id.as_deref(),
            message: &checked.message,
        },
        &mut *tx,
    )
    .await?;
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::ApiFeedbackSent,
        json!({ "feedback": public_id, "category": category.as_str() }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(public_id)
}

fn check_feedback(input: &SendApiFeedback) -> Result<CheckedFeedback, FeedbackProblem> {
    let message = redact_secrets(input.message.trim());
    if message.is_empty() {
        return Err(FeedbackProblem::MessageMissing);
    }
    if message.chars().count() > MAX_FEEDBACK_LENGTH {
        return Err(FeedbackProblem::MessageTooLong);
    }

    Ok(CheckedFeedback {
        endpoint: checked_endpoint(input.endpoint.as_deref())?,
        problem_code: checked_problem_code(input.problem_code.as_deref())?,
        request_id: checked_request_id(input.request_id.as_deref())?,
        message,
    })
}

fn present(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn checked_endpoint(value: Option<&str>) -> Result<Option<String>, FeedbackProblem> {
    let Some(value) = present(value) else {
        return Ok(None);
    };
    let id = value.replace('_', "-");
    match endpoints().iter().any(|endpoint| endpoint.id == id) {
        true => Ok(Some(id)),
        false => Err(FeedbackProblem::UnknownEndpoint),
    }
}

fn checked_problem_code(value: Option<&str>) -> Result<Option<String>, FeedbackProblem> {
    let Some(code) = present(value) else {
        return Ok(None);
    };
    let known = common_errors()
        .into_iter()
        .chain(endpoints().into_iter().flat_map(|endpoint| endpoint.errors))
        .any(|error| error.code == code);
    match known {
        true => Ok(Some(code.to_string())),
        false => Err(FeedbackProblem::UnknownProblemCode),
    }
}

fn checked_request_id(value: Option<&str>) -> Result<Option<String>, FeedbackProblem> {
    let Some(value) = present(value) else {
        return Ok(None);
    };
    uuid::Uuid::parse_str(value)
        .map(|id| Some(id.to_string()))
        .map_err(|_| FeedbackProblem::InvalidRequestId)
}
