use schemars::JsonSchema;
use serde::Deserialize;

use crate::action::{ActionProblem, ActionResult, Caller, Outcome, ProblemSpec, authorize, fail};
use crate::auth::emails::account_deleted_email;
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::permission::Permission;
use crate::user::queries::query_delete_user;

pub const CONFIRMATION_FIELD: &str = "confirmation";
const CONFIRMATION_WORD: &str = "DELETE";

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Delete the account and everything in it, right away and for good.")]
pub struct DeleteAccount {
    #[schemars(description = "The word DELETE, typed to confirm.")]
    pub confirmation: String,
    #[serde(default)]
    #[schemars(
        description = "The current password; leave it empty right after confirming with a passkey."
    )]
    pub current_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteAccountProblem {
    Confirmation,
    Reauth(ReauthProblem),
}

impl ActionProblem for DeleteAccountProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Confirmation => ProblemSpec::new(
                "confirmation_mismatch",
                "Type DELETE in capital letters to confirm.",
            )
            .on_field(CONFIRMATION_FIELD),
            Self::Reauth(problem) => problem.spec(),
        }
    }
}

pub async fn delete_account(
    env: &Env<'_>,
    caller: &Caller,
    input: &DeleteAccount,
    passkey: Option<PasskeyProof>,
) -> ActionResult<(), DeleteAccountProblem> {
    authorize!(caller.require(Permission::DeleteAccount));
    if let Err(problem) = check_request(env, caller, input, passkey).await? {
        return fail(problem);
    }

    let user = &caller.user;
    let mut tx = env.db().begin().await?;
    enqueue(&SendEmail(account_deleted_email(&user.email)), &mut *tx).await?;
    if !query_delete_user(user.id, &mut *tx).await? {
        return Err(topcoat::Error::msg("signed-in user no longer exists"));
    }
    tx.commit().await?;
    tracing::info!(user_id = user.id, "account deleted");

    Ok(Ok(()))
}

async fn check_request(
    env: &Env<'_>,
    caller: &Caller,
    input: &DeleteAccount,
    passkey: Option<PasskeyProof>,
) -> Outcome<(), DeleteAccountProblem> {
    if input.confirmation.trim() != CONFIRMATION_WORD {
        return fail(DeleteAccountProblem::Confirmation);
    }
    let password = &input.current_password;
    Ok(check_reauth(env, &caller.user, password, passkey)
        .await?
        .map_err(DeleteAccountProblem::Reauth))
}

#[cfg(test)]
mod tests;
