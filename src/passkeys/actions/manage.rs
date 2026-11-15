use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::env::Env;
use crate::passkeys::queries::{
    Passkey, query_delete_passkey, query_list_passkeys, query_rename_passkey,
};
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::name::{MAX_NAME_LENGTH, NameProblem, check_name};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasskeyNotFound {
    NotFound,
}

impl ActionProblem for PasskeyNotFound {
    fn spec(self) -> ProblemSpec {
        ProblemSpec::new("passkey_not_found", "That passkey doesn't exist.")
            .with_status(StatusCode::NOT_FOUND)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Rename a passkey.")]
pub struct RenamePasskey {
    #[schemars(description = "The passkey's id (pk_…).")]
    pub id: String,
    #[schemars(description = "The new name.", length(min = 1, max = MAX_NAME_LENGTH))]
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenamePasskeyProblem {
    Name(NameProblem),
    NotFound,
}

impl ActionProblem for RenamePasskeyProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Name(problem) => problem.spec(),
            Self::NotFound => PasskeyNotFound::NotFound.spec(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Remove a passkey.")]
pub struct RemovePasskey {
    #[schemars(description = "The passkey's id (pk_…).")]
    pub id: String,
}

pub async fn list_passkeys(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<Vec<Passkey>, Infallible> {
    authorize!(caller.require(Permission::ManagePasskeys));

    Ok(Ok(query_list_passkeys(caller.owner_id(), env.db()).await?))
}

pub async fn rename_passkey(
    env: &Env<'_>,
    caller: &Caller,
    input: &RenamePasskey,
) -> ActionResult<(), RenamePasskeyProblem> {
    authorize!(caller.require(Permission::ManagePasskeys));
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return fail(RenamePasskeyProblem::Name(problem)),
    };
    if !PublicIdKind::Passkey.is_valid(&input.id) {
        return fail(RenamePasskeyProblem::NotFound);
    }

    let mut tx = env.db().begin().await?;
    if !query_rename_passkey(&input.id, caller.owner_id(), name, &mut *tx).await? {
        return fail(RenamePasskeyProblem::NotFound);
    }
    let detail = json!({ "passkey": input.id, "name": name });
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::PasskeyRenamed,
        detail,
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(()))
}

pub async fn remove_passkey(
    env: &Env<'_>,
    caller: &Caller,
    input: &RemovePasskey,
) -> ActionResult<String, PasskeyNotFound> {
    authorize!(caller.require(Permission::ManagePasskeys));
    if !PublicIdKind::Passkey.is_valid(&input.id) {
        return fail(PasskeyNotFound::NotFound);
    }

    let mut tx = env.db().begin().await?;
    let Some(name) = query_delete_passkey(&input.id, caller.owner_id(), &mut *tx).await? else {
        return fail(PasskeyNotFound::NotFound);
    };
    let detail = json!({ "passkey": input.id, "name": name });
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::PasskeyRemoved,
        detail,
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, Denied};
    use crate::api::scope::ApiScope;
    use crate::env::App;
    use crate::user::model::User;
    use sqlx::PgPool;
    use std::sync::Arc;

    #[sqlx::test]
    async fn an_api_token_can_never_change_passkeys(db: PgPool) {
        let app = App::for_tests(db);
        let env = app.env(None);
        let caller = Caller::api_token(
            Arc::new(User::default_for_tests()),
            1,
            ApiScope::ALL.to_vec(),
        );
        let id = PublicIdKind::Passkey.generate();
        let rename = RenamePasskey {
            id: id.clone(),
            name: "Sneaky".to_string(),
        };

        let renamed = rename_passkey(&env, &caller, &rename).await.unwrap();
        let removed = remove_passkey(&env, &caller, &RemovePasskey { id })
            .await
            .unwrap();

        assert_eq!(renamed, Err(ActionError::Denied(Denied::BrowserOnly)));
        assert_eq!(removed, Err(ActionError::Denied(Denied::BrowserOnly)));
    }
}
