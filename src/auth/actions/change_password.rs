use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::auth::emails::password_changed_email;
use crate::auth::links::TokenPurpose;
use crate::auth::password::{
    CurrentPasswordProblem, MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH, PasswordProblem,
    check_current_password, check_new_password, hash_password,
};
use crate::auth::queries::query_delete_user_tokens;
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::permission::Permission;
use crate::routes::RESET_PASSWORD_PATH;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;
use crate::user::queries::query_update_password;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Change the password while signed in.")]
pub struct ChangePassword {
    #[schemars(description = "The current password.")]
    pub current_password: String,
    #[schemars(
        description = "The new password.",
        length(min = MIN_PASSWORD_LENGTH, max = MAX_PASSWORD_LENGTH)
    )]
    pub password: String,
    #[schemars(description = "The new password again.")]
    pub confirm_password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangePasswordProblem {
    CurrentPassword(CurrentPasswordProblem),
    NewPassword(PasswordProblem),
}

impl ActionProblem for ChangePasswordProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::CurrentPassword(problem) => problem.spec(),
            Self::NewPassword(problem) => problem.spec(),
        }
    }
}

pub async fn change_password(
    env: &Env<'_>,
    caller: &Caller,
    input: &ChangePassword,
) -> ActionResult<User, ChangePasswordProblem> {
    authorize!(caller.require(Permission::ManagePassword));
    if let Err(problem) = check_current_password(env, &caller.user, &input.current_password).await?
    {
        return fail(ChangePasswordProblem::CurrentPassword(problem));
    }
    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return fail(ChangePasswordProblem::NewPassword(problem));
    }

    let password_hash = hash_password(&input.password)?;
    let mut tx = env.db().begin().await?;
    let updated = query_update_password(caller.user.id, &password_hash, &mut *tx)
        .await?
        .ok_or_else(|| topcoat::Error::msg("signed-in user is not active"))?;
    query_delete_user_tokens(caller.user.id, TokenPurpose::EmailChange, &mut *tx).await?;
    query_delete_user_tokens(caller.user.id, TokenPurpose::PasswordReset, &mut *tx).await?;
    let source = EventSource::of(caller);
    record_event(
        env,
        &source,
        AccountEventKind::PasswordChanged,
        json!({}),
        &mut *tx,
    )
    .await?;
    announce_password_change(env, &updated.email, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(updated))
}

pub(super) async fn announce_password_change(
    env: &Env<'_>,
    recipient: &str,
    tx: &mut sqlx::PgConnection,
) -> Result<(), sqlx::Error> {
    let reset_url = env.app.url(RESET_PASSWORD_PATH);
    enqueue(
        &SendEmail(password_changed_email(recipient, &reset_url)),
        tx,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, Denied};
    use crate::api::scope::ApiScope;
    use crate::env::App;
    use sqlx::PgPool;
    use std::sync::Arc;

    #[sqlx::test]
    async fn an_api_token_can_never_change_the_password(db: PgPool) {
        let app = App::for_tests(db);
        let caller = Caller::api_token(
            Arc::new(User::default_for_tests()),
            1,
            ApiScope::ALL.to_vec(),
        );
        let input = ChangePassword {
            current_password: "hash".to_string(),
            password: "a-new-password".to_string(),
            confirm_password: "a-new-password".to_string(),
        };

        let outcome = change_password(&app.env(None), &caller, &input)
            .await
            .unwrap();

        assert_eq!(
            outcome.unwrap_err(),
            ActionError::Denied(Denied::BrowserOnly)
        );
    }

    #[sqlx::test]
    async fn a_changed_password_is_emailed_to_the_owner(db: PgPool) {
        let app = App::for_tests(db.clone());
        let user = crate::user::testing::active_user("changed@example.test", &db).await;
        let input = ChangePassword {
            current_password: crate::user::testing::TEST_PASSWORD.to_string(),
            password: "a-brand-new-password".to_string(),
            confirm_password: "a-brand-new-password".to_string(),
        };

        change_password(&app.env(None), &Caller::browser(user), &input)
            .await
            .unwrap()
            .unwrap();

        let subjects = sqlx::query_scalar!(
            r#"SELECT payload->>'subject' AS "subject!" FROM jobs WHERE kind = 'send_email' AND payload->>'to' = 'changed@example.test'"#
        )
        .fetch_all(&db)
        .await
        .unwrap();
        assert_eq!(subjects, ["Your password was changed"]);
    }

    #[test]
    fn every_problem_names_a_real_password_field() {
        let fields = ["current_password", "password", "confirm_password"];
        for problem in [
            ChangePasswordProblem::CurrentPassword(CurrentPasswordProblem::Wrong),
            ChangePasswordProblem::NewPassword(PasswordProblem::TooShort),
            ChangePasswordProblem::NewPassword(PasswordProblem::Mismatch),
        ] {
            assert!(fields.contains(&problem.field().unwrap()));
        }
    }
}
