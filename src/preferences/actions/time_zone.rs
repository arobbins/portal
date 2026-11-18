use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::action::{ActionProblem, ActionResult, Caller, ProblemSpec, authorize, fail};
use crate::env::Env;
use crate::permission::Permission;
use crate::time_zone::Zone;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::queries::query_update_user_time_zone;

pub const TIME_ZONE_FIELD: &str = "time_zone";
const MAX_TIME_ZONE_LENGTH: usize = 64;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Choose the time zone dates and times are shown in.")]
pub struct UpdateTimeZone {
    #[schemars(
        description = "An IANA time zone name, such as Europe/Paris or America/New_York.",
        length(min = 1, max = MAX_TIME_ZONE_LENGTH)
    )]
    pub time_zone: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeZoneProblem {
    Unknown,
}

impl ActionProblem for TimeZoneProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Unknown => {
                ProblemSpec::new("unknown_time_zone", "Choose a time zone from the list.")
                    .on_field(TIME_ZONE_FIELD)
            }
        }
    }
}

pub async fn update_time_zone(
    env: &Env<'_>,
    caller: &Caller,
    input: &UpdateTimeZone,
) -> ActionResult<String, TimeZoneProblem> {
    authorize!(caller.require(Permission::EditProfile));
    let Some(zone) = Zone::parse(&input.time_zone) else {
        return fail(TimeZoneProblem::Unknown);
    };

    let mut tx = env.db().begin().await?;
    query_update_user_time_zone(caller.user.id, zone.name(), &mut *tx).await?;
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::TimeZoneChanged,
        json!({ "time_zone": zone.name() }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(zone.name().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionError;
    use crate::env::App;
    use crate::user::testing::active_user;
    use sqlx::PgPool;

    fn choose(time_zone: &str) -> UpdateTimeZone {
        UpdateTimeZone {
            time_zone: time_zone.to_string(),
        }
    }

    #[sqlx::test]
    async fn a_known_zone_is_saved_and_logged(db: PgPool) {
        let app = App::for_tests(db.clone());
        let env = app.env(None);
        let user = active_user("zone@example.test", &db).await;
        let caller = Caller::browser(user.clone());

        let saved = update_time_zone(&env, &caller, &choose(" Europe/Paris "))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved, "Europe/Paris");
        let stored = sqlx::query_scalar!("SELECT time_zone FROM users WHERE id = $1", user.id)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(stored.as_deref(), Some("Europe/Paris"));
        let kinds = sqlx::query_scalar!(
            "SELECT kind FROM account_events WHERE user_id = $1 AND kind = 'time_zone_changed'",
            user.id
        )
        .fetch_all(&db)
        .await
        .unwrap();
        assert_eq!(kinds.len(), 1);
    }

    #[sqlx::test]
    async fn an_unknown_zone_changes_nothing(db: PgPool) {
        let app = App::for_tests(db.clone());
        let env = app.env(None);
        let user = active_user("nozone@example.test", &db).await;
        let caller = Caller::browser(user.clone());

        let result = update_time_zone(&env, &caller, &choose("Mars/Olympus_Mons"))
            .await
            .unwrap();
        assert!(matches!(
            result,
            Err(ActionError::Problem(TimeZoneProblem::Unknown))
        ));
        let stored = sqlx::query_scalar!("SELECT time_zone FROM users WHERE id = $1", user.id)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(stored, None);
    }
}
