use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use topcoat::router::StatusCode;

use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::auth::reauth::{PasskeyProof, ReauthProblem, check_reauth};
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
    #[serde(default)]
    #[schemars(
        description = "The current password; leave it empty right after confirming with a passkey."
    )]
    pub current_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovePasskeyProblem {
    NotFound,
    Reauth(ReauthProblem),
}

impl ActionProblem for RemovePasskeyProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::NotFound => PasskeyNotFound::NotFound.spec(),
            Self::Reauth(problem) => problem.spec(),
        }
    }
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
    passkey: Option<PasskeyProof>,
) -> ActionResult<String, RemovePasskeyProblem> {
    authorize!(caller.require(Permission::ManagePasskeys));
    if !PublicIdKind::Passkey.is_valid(&input.id) {
        return fail(RemovePasskeyProblem::NotFound);
    }
    let password = &input.current_password;
    if let Err(problem) = check_reauth(env, &caller.user, password, passkey).await? {
        return fail(RemovePasskeyProblem::Reauth(problem));
    }

    let mut tx = env.db().begin().await?;
    let Some(name) = query_delete_passkey(&input.id, caller.owner_id(), &mut *tx).await? else {
        return fail(RemovePasskeyProblem::NotFound);
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
    use crate::auth::password::CurrentPasswordProblem;
    use crate::auth::reauth::{PasskeyProof, ReauthProblem};
    use crate::env::App;
    use crate::user::model::User;
    use crate::user::testing::{TEST_PASSWORD, active_user};
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
        let input = RemovePasskey {
            id,
            current_password: TEST_PASSWORD.to_string(),
        };
        let removed = remove_passkey(&env, &caller, &input, None).await.unwrap();

        assert_eq!(renamed, Err(ActionError::Denied(Denied::BrowserOnly)));
        assert_eq!(removed, Err(ActionError::Denied(Denied::BrowserOnly)));
    }

    async fn spare_passkey(user_id: i32, db: &PgPool) -> String {
        sqlx::query_scalar!(
            r#"INSERT INTO user_passkeys (user_id, credential_id, passkey, name, backup_eligible, backed_up, public_id)
            VALUES ($1, convert_to(random_base62(16), 'UTF8'), '{}', 'Spare key', false, false, 'pk_' || random_base62(20))
            RETURNING public_id"#,
            user_id,
        )
        .fetch_one(db)
        .await
        .unwrap()
    }

    async fn remaining(user_id: i32, db: &PgPool) -> i64 {
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM user_passkeys WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(db)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn removing_a_passkey_needs_the_password_or_a_passkey_confirmation(db: PgPool) {
        let app = App::for_tests(db.clone());
        let env = app.env(None);
        let user = active_user("remover@example.test", &db).await;
        let caller = Caller::browser(user.clone());
        let remove = |password: &str, id: &str| RemovePasskey {
            id: id.to_string(),
            current_password: password.to_string(),
        };
        let id = spare_passkey(user.id, &db).await;

        let missing = remove_passkey(&env, &caller, &remove("", &id), None)
            .await
            .unwrap();
        let wrong = remove_passkey(&env, &caller, &remove("not-it", &id), None)
            .await
            .unwrap();

        assert_eq!(
            missing,
            Err(ActionError::Problem(RemovePasskeyProblem::Reauth(
                ReauthProblem::Missing
            )))
        );
        assert_eq!(
            wrong,
            Err(ActionError::Problem(RemovePasskeyProblem::Reauth(
                ReauthProblem::Password(CurrentPasswordProblem::Wrong)
            )))
        );
        assert_eq!(remaining(user.id, &db).await, 1);

        let with_password = remove_passkey(&env, &caller, &remove(TEST_PASSWORD, &id), None).await;
        assert!(with_password.unwrap().is_ok());
        let id = spare_passkey(user.id, &db).await;
        let proof = Some(PasskeyProof::for_tests(user.id));
        let with_passkey = remove_passkey(&env, &caller, &remove("", &id), proof).await;
        assert!(with_passkey.unwrap().is_ok());
        assert_eq!(remaining(user.id, &db).await, 0);
    }
}
