use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use topcoat::{Result, context::Cx, router::request};
use tower_sessions::Session;

use crate::billing::catalog::is_price_id;
use crate::billing::checkout_for_price;
use crate::env::{Env, is_within};
use crate::oauth::authorize::{keep_request_alive, request_path};
use crate::routes::PRICE_PARAM;
use crate::secret::PublicIdKind;
use crate::user::model::User;
use crate::web::env;

const AUTHORIZE_PARAM: &str = "authorize";
const AFTER_SIGN_IN: &str = "after_sign_in";
const HANDOFF_LASTS: Duration = Duration::hours(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Handoff {
    Checkout(String),
    Authorize(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredHandoff {
    handoff: Handoff,
    at: i64,
}

impl Handoff {
    pub fn parse(param: &str, value: &str) -> Option<Self> {
        match param {
            PRICE_PARAM if is_price_id(value) => Some(Self::Checkout(value.to_string())),
            AUTHORIZE_PARAM if PublicIdKind::AuthorizationRequest.is_valid(value) => {
                Some(Self::Authorize(value.to_string()))
            }
            _ => None,
        }
    }

    pub fn chosen(price: Option<&str>, request: Option<&str>) -> Option<Self> {
        let price = price.and_then(|price| Self::parse(PRICE_PARAM, price));
        price.or_else(|| request.and_then(|request| Self::parse(AUTHORIZE_PARAM, request)))
    }

    pub fn from_payload(payload: &str) -> Option<Self> {
        Self::parse(PRICE_PARAM, payload).or_else(|| Self::parse(AUTHORIZE_PARAM, payload))
    }

    pub fn param(&self) -> &'static str {
        match self {
            Self::Checkout(_) => PRICE_PARAM,
            Self::Authorize(_) => AUTHORIZE_PARAM,
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Self::Checkout(value) | Self::Authorize(value) => value,
        }
    }
}

pub fn handoff_param(cx: &Cx) -> Option<Handoff> {
    let query = request::uri(cx).query()?;
    url::form_urlencoded::parse(query.as_bytes())
        .find_map(|(key, value)| Handoff::parse(&key, &value))
}

pub fn with_handoff(path: &str, handoff: Option<&Handoff>) -> String {
    match handoff {
        Some(handoff) => format!("{path}?{}={}", handoff.param(), handoff.value()),
        None => path.to_string(),
    }
}

pub async fn keep_handoff_alive(env: &Env<'_>, handoff: Option<&Handoff>) -> Result<()> {
    if let Some(Handoff::Authorize(request)) = handoff {
        keep_request_alive(env, request).await?;
    }
    Ok(())
}

pub async fn remember_handoff(
    env: &Env<'_>,
    session: &Session,
    handoff: Option<Handoff>,
) -> Result<()> {
    let Some(handoff) = handoff else {
        return Ok(());
    };
    keep_handoff_alive(env, Some(&handoff)).await?;
    let at = env.now().unix_timestamp();
    session
        .insert(AFTER_SIGN_IN, StoredHandoff { handoff, at })
        .await?;
    Ok(())
}

async fn take_handoff(session: &Session, now: OffsetDateTime) -> Result<Option<Handoff>> {
    let stored = session.remove::<StoredHandoff>(AFTER_SIGN_IN).await?;
    Ok(stored
        .filter(|stored| is_within(stored.at, now, HANDOFF_LASTS))
        .map(|stored| stored.handoff))
}

pub async fn destination_after_sign_in(
    cx: &Cx,
    session: &Session,
    user: &User,
) -> Result<Option<String>> {
    match take_handoff(session, env(cx).now()).await? {
        None => Ok(None),
        Some(Handoff::Checkout(price)) => checkout_for_price(&env(cx), user.id, price).await,
        Some(Handoff::Authorize(request)) => Ok(Some(request_path(&request))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tower_sessions::MemoryStore;

    use super::*;

    fn session() -> Session {
        Session::new(None, Arc::new(MemoryStore::default()), None)
    }

    async fn store(session: &Session, handoff: Handoff, at: OffsetDateTime) {
        let stored = StoredHandoff {
            handoff,
            at: at.unix_timestamp(),
        };
        session.insert(AFTER_SIGN_IN, stored).await.unwrap();
    }

    #[test]
    fn only_price_ids_and_request_ids_are_added_to_links() {
        let price = Handoff::parse(PRICE_PARAM, "price_pro");
        assert_eq!(
            with_handoff("/login", price.as_ref()),
            "/login?price=price_pro"
        );
        let request = PublicIdKind::AuthorizationRequest.generate();
        let authorize = Handoff::parse(AUTHORIZE_PARAM, &request);
        assert_eq!(
            with_handoff("/login", authorize.as_ref()),
            format!("/login?authorize={request}")
        );
        assert_eq!(Handoff::parse(PRICE_PARAM, "price_x&next=evil"), None);
        assert_eq!(Handoff::parse(AUTHORIZE_PARAM, "https://evil.test"), None);
        assert_eq!(Handoff::parse("next", "price_pro"), None);
        assert_eq!(Handoff::from_payload(&request), authorize);
        assert_eq!(Handoff::from_payload("price_pro"), price);
    }

    #[tokio::test]
    async fn a_handoff_is_used_once_and_only_for_an_hour() {
        let now = OffsetDateTime::now_utc();
        let session = session();
        let handoff = Handoff::Checkout("price_pro".to_string());

        store(&session, handoff.clone(), now).await;
        assert_eq!(
            take_handoff(&session, now).await.unwrap(),
            Some(handoff.clone())
        );
        assert_eq!(take_handoff(&session, now).await.unwrap(), None);

        store(&session, handoff, now).await;
        let later = now + HANDOFF_LASTS + Duration::seconds(1);
        assert_eq!(take_handoff(&session, later).await.unwrap(), None);
    }
}
