use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use super::archive::{archive, file_name};
use super::datasets::collect;
use super::emails::data_exported_email;
use crate::action::{
    ActionProblem, ActionResult, Caller, Outcome, ProblemSpec, TooManyAttempts, authorize, fail,
};
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::permission::Permission;
use crate::user::events::{AccountEventKind, EventSource, record_event};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Download a copy of everything the app keeps about the account.")]
pub struct ExportData {
    #[serde(default)]
    #[schemars(
        description = "The current password; leave it empty right after confirming with a passkey."
    )]
    pub current_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportProblem {
    Reauth(ReauthProblem),
    TooManyAttempts,
}

impl ActionProblem for ExportProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Reauth(problem) => problem.spec(),
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataExport {
    pub file_name: String,
    pub zip: Vec<u8>,
}

pub async fn export_data(
    env: &Env<'_>,
    caller: &Caller,
    input: &ExportData,
    passkey: Option<PasskeyProof>,
) -> ActionResult<DataExport, ExportProblem> {
    authorize!(caller.require(Permission::ExportData));
    if let Err(problem) = check_request(env, caller, input, passkey).await? {
        return fail(problem);
    }

    let datasets = collect(env, &caller.user).await?;
    let export = DataExport {
        file_name: file_name(env.now()),
        zip: archive(&datasets)?,
    };
    let mut tx = env.db().begin().await?;
    let detail = json!({ "files": datasets.len() + 1 });
    let source = EventSource::of(caller);
    record_event(
        env,
        &source,
        AccountEventKind::DataExported,
        detail,
        &mut *tx,
    )
    .await?;
    enqueue(
        &SendEmail(data_exported_email(&caller.user.email)),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(export))
}

async fn check_request(
    env: &Env<'_>,
    caller: &Caller,
    input: &ExportData,
    passkey: Option<PasskeyProof>,
) -> Outcome<(), ExportProblem> {
    let password = &input.current_password;
    if let Err(problem) = check_reauth(env, &caller.user, password, passkey).await? {
        return fail(ExportProblem::Reauth(problem));
    }
    if !env.limits().allow_export(caller.user.id).await? {
        return fail(ExportProblem::TooManyAttempts);
    }
    Ok(Ok(()))
}

#[cfg(test)]
mod tests;
