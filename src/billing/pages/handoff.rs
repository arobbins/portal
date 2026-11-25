use std::sync::Arc;

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use topcoat::{Result, context::Cx, router::request};
use tower_sessions::Session;

use crate::action::Caller;
use crate::billing::actions::{StartCheckout, start_checkout};
use crate::billing::catalog::is_price_id;
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::web::env;

pub const PRICE_PARAM: &str = "price";
const CHOSEN_PRICE: &str = "chosen_price";
const CHECKOUT_PENDING: &str = "checkout_pending";
const CHOICE_LASTS: Duration = Duration::hours(1);
const PENDING_LASTS: Duration = Duration::hours(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChosenPrice {
    price: String,
    chosen_at: i64,
}

pub fn price_param(cx: &Cx) -> Option<String> {
    let query = request::uri(cx).query()?;
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == PRICE_PARAM)
        .map(|(_, value)| value.into_owned())
        .filter(|price| is_price_id(price))
}

pub fn with_price(path: &str, price: Option<&str>) -> String {
    match price.filter(|price| is_price_id(price)) {
        Some(price) => format!("{path}?{PRICE_PARAM}={price}"),
        None => path.to_string(),
    }
}

pub async fn remember_chosen_price(
    session: &Session,
    price: Option<&str>,
    now: OffsetDateTime,
) -> Result<()> {
    let Some(price) = price.filter(|price| is_price_id(price)) else {
        return Ok(());
    };
    let chosen = ChosenPrice {
        price: price.to_string(),
        chosen_at: now.unix_timestamp(),
    };
    session.insert(CHOSEN_PRICE, chosen).await?;
    Ok(())
}

async fn take_chosen_price(session: &Session, now: OffsetDateTime) -> Result<Option<String>> {
    let chosen = session.remove::<ChosenPrice>(CHOSEN_PRICE).await?;
    Ok(chosen
        .filter(|chosen| now.unix_timestamp() - chosen.chosen_at < CHOICE_LASTS.whole_seconds())
        .map(|chosen| chosen.price))
}

pub async fn checkout_after_sign_in(
    cx: &Cx,
    session: &Session,
    user: &User,
) -> Result<Option<String>> {
    let env = env(cx);
    let Some(price) = take_chosen_price(session, env.now()).await? else {
        return Ok(None);
    };
    let Some(user) = query_get_user_by_id(user.id, env.db()).await? else {
        return Ok(None);
    };
    let caller = Caller::browser(Arc::new(user));
    match start_checkout(&env, &caller, &StartCheckout { price }).await? {
        Ok(link) => Ok(Some(link.checkout_url)),
        Err(error) => {
            tracing::info!("no checkout after sign-in: {:?}", error.for_browser()?);
            Ok(None)
        }
    }
}

pub async fn mark_checkout_pending(session: &Session, now: OffsetDateTime) -> Result<()> {
    session
        .insert(CHECKOUT_PENDING, now.unix_timestamp())
        .await?;
    Ok(())
}

pub async fn checkout_pending(session: &Session, now: OffsetDateTime) -> Result<bool> {
    let marked = session.get::<i64>(CHECKOUT_PENDING).await?;
    Ok(marked
        .and_then(|at| OffsetDateTime::from_unix_timestamp(at).ok())
        .is_some_and(|at| now - at < PENDING_LASTS))
}

pub async fn clear_checkout_pending(session: &Session) -> Result<()> {
    session.remove::<i64>(CHECKOUT_PENDING).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tower_sessions::MemoryStore;

    use super::*;

    fn session() -> Session {
        Session::new(None, Arc::new(MemoryStore::default()), None)
    }

    #[test]
    fn only_price_ids_are_added_to_links() {
        assert_eq!(
            with_price("/login", Some("price_pro")),
            "/login?price=price_pro"
        );
        assert_eq!(with_price("/login", Some("price_x&next=evil")), "/login");
        assert_eq!(with_price("/login", None), "/login");
    }

    #[tokio::test]
    async fn a_chosen_price_is_used_once_and_only_for_an_hour() {
        let now = OffsetDateTime::now_utc();
        let session = session();
        let taken = async |at| take_chosen_price(&session, at).await.unwrap();
        remember_chosen_price(&session, Some("price_pro"), now)
            .await
            .unwrap();

        assert_eq!(taken(now).await.as_deref(), Some("price_pro"));
        assert_eq!(taken(now).await, None);

        remember_chosen_price(&session, Some("price_pro"), now)
            .await
            .unwrap();
        assert_eq!(taken(now + CHOICE_LASTS + Duration::seconds(1)).await, None);

        remember_chosen_price(&session, Some("javascript:alert(1)"), now)
            .await
            .unwrap();
        assert_eq!(taken(now).await, None);
    }
}
