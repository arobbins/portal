use std::convert::Infallible;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::action::{ActionResult, Caller, authorize};
use crate::env::Env;
use crate::permission::Permission;
use crate::preferences::notifications::{NotificationTopic, wants};
use crate::preferences::queries::{query_delete_opt_out, query_insert_opt_out};
use crate::user::events::{AccountEventKind, EventSource, record_event};

pub const SIGN_IN_ALERTS_FIELD: &str = "sign_in_alerts";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(
    description = "Which optional emails the user gets. Security changes are always emailed."
)]
pub struct NotificationSettings {
    #[schemars(
        description = "Email when someone signs in from a browser or device not seen before."
    )]
    pub sign_in_alerts: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Choose which optional emails to get.")]
pub struct UpdateNotifications {
    #[serde(default, deserialize_with = "crate::utils::checkbox")]
    #[schemars(
        description = "Email when someone signs in from a browser or device not seen before."
    )]
    pub sign_in_alerts: bool,
}

pub async fn get_notifications(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<NotificationSettings, Infallible> {
    authorize!(caller.require(Permission::ViewProfile));
    let sign_in_alerts = wants(caller.user.id, NotificationTopic::SignInAlerts, env.db()).await?;

    Ok(Ok(NotificationSettings { sign_in_alerts }))
}

pub async fn update_notifications(
    env: &Env<'_>,
    caller: &Caller,
    input: &UpdateNotifications,
) -> ActionResult<NotificationSettings, Infallible> {
    authorize!(caller.require(Permission::EditProfile));
    let topic = NotificationTopic::SignInAlerts;

    let mut tx = env.db().begin().await?;
    if input.sign_in_alerts {
        query_delete_opt_out(caller.user.id, topic, &mut *tx).await?;
    } else {
        query_insert_opt_out(caller.user.id, topic, &mut *tx).await?;
    }
    record_event(
        env,
        &EventSource::of(caller),
        AccountEventKind::NotificationsUpdated,
        json!({ "sign_in_alerts": input.sign_in_alerts }),
        &mut *tx,
    )
    .await?;
    tx.commit().await?;

    Ok(Ok(NotificationSettings {
        sign_in_alerts: input.sign_in_alerts,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::App;
    use crate::user::testing::active_user;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn sign_in_alerts_are_on_until_turned_off_and_each_change_is_logged(db: PgPool) {
        let app = App::for_tests(db.clone());
        let env = app.env(None);
        let user = active_user("alerts@example.test", &db).await;
        let caller = Caller::browser(user.clone());
        let read = async || get_notifications(&env, &caller).await.unwrap().unwrap();

        assert!(read().await.sign_in_alerts);
        let off = UpdateNotifications {
            sign_in_alerts: false,
        };
        update_notifications(&env, &caller, &off)
            .await
            .unwrap()
            .unwrap();
        assert!(!read().await.sign_in_alerts);
        update_notifications(&env, &caller, &off)
            .await
            .unwrap()
            .unwrap();
        let on = UpdateNotifications {
            sign_in_alerts: true,
        };
        update_notifications(&env, &caller, &on)
            .await
            .unwrap()
            .unwrap();
        assert!(read().await.sign_in_alerts);
        let logged = sqlx::query_scalar!(
            "SELECT count(*) AS \"count!\" FROM account_events WHERE user_id = $1 AND kind = 'notifications_updated'",
            user.id
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(logged, 3);
    }

    #[test]
    fn a_form_checkbox_or_json_flag_is_understood() {
        let form: UpdateNotifications = serde_urlencoded::from_str("sign_in_alerts=on").unwrap();
        assert!(form.sign_in_alerts);
        let unchecked: UpdateNotifications = serde_urlencoded::from_str("").unwrap();
        assert!(!unchecked.sign_in_alerts);
        let json: UpdateNotifications =
            serde_json::from_str(r#"{"sign_in_alerts": true}"#).unwrap();
        assert!(json.sign_in_alerts);
    }
}
