use std::convert::Infallible;

use crate::env::Env;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::action::{ActionError, ActionResult, Caller, authorize, fail};
use crate::permission::Permission;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;
use crate::user::name::{MAX_NAME_LENGTH, NameProblem, check_name};
use crate::user::queries::query_update_user_name;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "The signed-in user's profile.")]
pub struct ProfileView {
    #[schemars(description = "Display name.")]
    pub name: String,
    #[schemars(description = "Email address the account signs in with.")]
    pub email: String,
}

impl ProfileView {
    fn of(user: &User) -> Self {
        Self {
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Change the user's display name.")]
pub struct UpdateProfile {
    #[schemars(
        description = "New display name; must not be blank. Leading and trailing spaces are removed.",
        length(min = 1, max = MAX_NAME_LENGTH)
    )]
    pub name: String,
}

pub fn get_profile(caller: &Caller) -> Result<ProfileView, ActionError<Infallible>> {
    caller
        .require(Permission::ViewProfile)
        .map_err(ActionError::Denied)?;

    Ok(ProfileView::of(&caller.user))
}

pub async fn update_profile(
    env: &Env<'_>,
    caller: &Caller,
    input: &UpdateProfile,
) -> ActionResult<ProfileView, NameProblem> {
    authorize!(caller.require(Permission::EditProfile));
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => return fail(problem),
    };

    let mut tx = env.db().begin().await?;
    query_update_user_name(caller.user.id, name, &mut *tx).await?;
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::ProfileUpdated,
        json!({ "fields": ["name"] }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(ProfileView {
        name: name.to_string(),
        ..ProfileView::of(&caller.user)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::scope::ApiScope;
    use std::sync::Arc;

    #[test]
    fn reading_the_profile_needs_the_read_scope() {
        let user = Arc::new(User::default_for_tests());
        let reader = Caller::api_token(user.clone(), 1, vec![ApiScope::ProfileRead]);
        let nothing = Caller::api_token(user.clone(), 2, vec![]);

        assert_eq!(get_profile(&reader).unwrap().email, user.email);
        assert!(matches!(get_profile(&nothing), Err(ActionError::Denied(_))));
    }

    async fn saved_user(db: &sqlx::PgPool) -> Arc<User> {
        let user_id = crate::user::queries::query_insert_user(
            &crate::user::queries::NewUser {
                name: "Before",
                email: "actions@example.test",
            },
            db,
        )
        .await
        .unwrap()
        .unwrap();
        Arc::new(
            crate::user::queries::query_get_user_by_id(user_id, db)
                .await
                .unwrap()
                .unwrap(),
        )
    }

    async fn state(user_id: i32, db: &sqlx::PgPool) -> (String, Vec<(String, Option<i32>)>) {
        let name = sqlx::query_scalar!("SELECT name FROM users WHERE id = $1", user_id)
            .fetch_one(db)
            .await
            .unwrap();
        let events = sqlx::query!(
            "SELECT channel, api_token_id FROM account_events WHERE user_id = $1 ORDER BY id",
            user_id,
        )
        .fetch_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.channel, row.api_token_id))
        .collect();
        (name, events)
    }

    fn rename(name: &str) -> UpdateProfile {
        UpdateProfile {
            name: name.to_string(),
        }
    }

    #[sqlx::test]
    async fn renaming_works_from_the_browser_and_a_write_token_and_is_logged(db: sqlx::PgPool) {
        let user = saved_user(&db).await;
        let app = crate::env::App::for_tests(db.clone());
        let env = app.env(None);

        let browser = Caller::browser(user.clone());
        assert_eq!(
            update_profile(&env, &browser, &rename(" From Browser "))
                .await
                .unwrap()
                .unwrap()
                .name,
            "From Browser"
        );
        let writer = Caller::api_token(user.clone(), 9, vec![ApiScope::ProfileWrite]);
        update_profile(&env, &writer, &rename("From Token"))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            state(user.id, &db).await,
            (
                "From Token".to_string(),
                vec![
                    ("browser".to_string(), None),
                    ("api_token".to_string(), Some(9))
                ]
            )
        );
    }

    #[sqlx::test]
    async fn a_read_only_token_changes_nothing(db: sqlx::PgPool) {
        let user = saved_user(&db).await;
        let app = crate::env::App::for_tests(db.clone());
        let env = app.env(None);
        let reader = Caller::api_token(user.clone(), 3, vec![ApiScope::ProfileRead]);

        let outcome = update_profile(&env, &reader, &rename("Sneaky"))
            .await
            .unwrap();

        assert_eq!(
            outcome,
            Err(ActionError::Denied(crate::action::Denied::MissingScope(
                ApiScope::ProfileWrite
            )))
        );
        assert_eq!(state(user.id, &db).await, ("Before".to_string(), vec![]));
    }

    #[test]
    fn the_update_schema_describes_its_limits() {
        let schema = serde_json::to_value(schemars::schema_for!(UpdateProfile)).unwrap();

        assert_eq!(schema["required"], serde_json::json!(["name"]));
        assert_eq!(schema["additionalProperties"], serde_json::json!(false));
        assert_eq!(schema["properties"]["name"]["maxLength"], 100);
        assert_eq!(schema["properties"]["name"]["minLength"], 1);
    }
}
