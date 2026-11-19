use std::convert::Infallible;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgConnection;

use crate::action::{ActionResult, Caller, authorize};
use crate::env::Env;
use crate::permission::Permission;
use crate::preferences::newsletters::{Newsletter, subscribed};
use crate::preferences::queries::{
    query_delete_newsletter_opt_out, query_insert_newsletter_opt_out,
};
use crate::user::events::{AccountEventKind, EventSource, record_event};

const GENERAL_DESCRIPTION: &str = "The General newsletter: news about the app.";
const PRODUCT_DESCRIPTION: &str = "Product updates: new features and changes.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "Which newsletters the account is subscribed to.")]
pub struct NewsletterSettings {
    #[schemars(description = GENERAL_DESCRIPTION)]
    pub general_newsletter: bool,
    #[schemars(description = PRODUCT_DESCRIPTION)]
    pub product_updates: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    description = "Subscribe to or unsubscribe from newsletters. Leave a field out to keep that choice as it is."
)]
pub struct UpdateNewsletters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = GENERAL_DESCRIPTION)]
    pub general_newsletter: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = PRODUCT_DESCRIPTION)]
    pub product_updates: Option<bool>,
}

impl UpdateNewsletters {
    pub fn one(newsletter: Newsletter, on: bool) -> Self {
        match newsletter {
            Newsletter::GeneralNewsletter => Self {
                general_newsletter: Some(on),
                product_updates: None,
            },
            Newsletter::ProductUpdates => Self {
                general_newsletter: None,
                product_updates: Some(on),
            },
        }
    }

    fn changes(&self) -> [(Newsletter, Option<bool>); 2] {
        [
            (Newsletter::GeneralNewsletter, self.general_newsletter),
            (Newsletter::ProductUpdates, self.product_updates),
        ]
    }
}

pub async fn get_newsletters(
    env: &Env<'_>,
    caller: &Caller,
) -> ActionResult<NewsletterSettings, Infallible> {
    authorize!(caller.require(Permission::ViewProfile));
    let mut conn = env.db().acquire().await?;

    Ok(Ok(read_settings(caller.user.id, &mut conn).await?))
}

pub async fn update_newsletters(
    env: &Env<'_>,
    caller: &Caller,
    input: &UpdateNewsletters,
) -> ActionResult<NewsletterSettings, Infallible> {
    authorize!(caller.require(Permission::EditProfile));
    let user_id = caller.user.id;

    let mut tx = env.db().begin().await?;
    for (newsletter, choice) in input.changes() {
        let Some(on) = choice else { continue };
        let changed = if on {
            query_delete_newsletter_opt_out(user_id, newsletter, &mut *tx).await?
        } else {
            query_insert_newsletter_opt_out(user_id, newsletter, &mut *tx).await?
        };
        if changed {
            let kind = if on {
                AccountEventKind::NewsletterSubscribed
            } else {
                AccountEventKind::NewsletterUnsubscribed
            };
            let detail = json!({ "newsletter": newsletter.as_str(), "name": newsletter.label() });
            record_event(env, &EventSource::of(caller), kind, detail, &mut *tx).await?;
        }
    }
    let settings = read_settings(user_id, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(settings))
}

async fn read_settings(
    user_id: i32,
    conn: &mut PgConnection,
) -> Result<NewsletterSettings, sqlx::Error> {
    Ok(NewsletterSettings {
        general_newsletter: subscribed(user_id, Newsletter::GeneralNewsletter, &mut *conn).await?,
        product_updates: subscribed(user_id, Newsletter::ProductUpdates, &mut *conn).await?,
    })
}

#[cfg(test)]
mod tests;
