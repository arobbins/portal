use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{HeaderName, content::Form, header, route},
    runtime::procedure,
};

use crate::account::tabs::AccountTab;
use crate::action::{ActionResult, Caller, FieldError, browser_caller, require_browser_caller};
use crate::auth::reauth::{return_passkey_proof, take_passkey_proof};
use crate::data_export::actions::{DataExport, ExportData, ExportProblem, export_data};
use crate::web::{env, redisplay, session};

type Download = ([(HeaderName, String); 3], Vec<u8>);

struct ExportFailed {
    problem: ExportProblem,
}

#[procedure]
pub(super) async fn export_data_js(
    cx: &Cx,
    current_password: String,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let input = ExportData { current_password };

    match run_export(cx, &caller, &input).await? {
        Ok(export) => Ok(Ok(format!(
            "{}\n{}",
            export.file_name,
            STANDARD.encode(&export.zip)
        ))),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/data/export")]
pub async fn export_data_submit(cx: &Cx, Form(input): Form<ExportData>) -> Result<Download> {
    let caller = require_browser_caller(cx)?;

    match run_export(cx, &caller, &input).await? {
        Ok(export) => Ok(download(export)),
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(
                AccountTab::Profile.path(),
                ExportFailed { problem },
            ))
        }
    }
}

fn download(export: DataExport) -> Download {
    let disposition = format!("attachment; filename=\"{}\"", export.file_name);
    (
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        export.zip,
    )
}

async fn run_export(
    cx: &Cx,
    caller: &Caller,
    input: &ExportData,
) -> ActionResult<DataExport, ExportProblem> {
    let env = env(cx);
    let session = session(cx)?;
    let passkey = take_passkey_proof(&session, &caller.user, env.now()).await?;
    let outcome = export_data(&env, caller, input, passkey).await?;
    if outcome.is_err() {
        return_passkey_proof(&session, passkey).await?;
    }
    Ok(outcome)
}

pub fn export_problem(cx: &Cx) -> Option<ExportProblem> {
    try_context::<ExportFailed>(cx).map(|failed| failed.problem)
}
