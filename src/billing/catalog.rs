use serde_json::Value;

use super::period::Interval;
use crate::stripe::{Request, Stripe, StripeError, is_id};

pub const FREE_PLAN_KEY: &str = "free";
pub const FREE_PLAN_NAME: &str = "Free";
const PLAN_KEY_METADATA: &str = "plan_key";
const PRICES_PATH: &str = "/v1/prices?active=true&type=recurring&limit=100&expand[]=data.product";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaidPrice {
    pub plan_key: String,
    pub plan_name: String,
    pub price_id: String,
    pub amount: i64,
    pub currency: String,
    pub interval: Interval,
    pub interval_count: i64,
}

impl PaidPrice {
    pub fn label(&self) -> String {
        let every = match self.interval_count {
            1 => self.interval.as_str().to_string(),
            count => format!("{count} {}s", self.interval.as_str()),
        };
        format!("{} / {every}", money(self.amount, &self.currency))
    }

    pub fn decimal_amount(&self) -> String {
        format!("{}.{:02}", self.amount / 100, self.amount % 100)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    pub prices: Vec<PaidPrice>,
}

impl Catalog {
    pub fn from_list(list: &Value) -> Self {
        let mut prices: Vec<PaidPrice> = list["data"]
            .as_array()
            .map(|data| data.iter().filter_map(offered_price).collect())
            .unwrap_or_default();
        prices.sort_by(|a, b| {
            a.amount
                .cmp(&b.amount)
                .then_with(|| a.plan_name.cmp(&b.plan_name))
        });
        Self { prices }
    }

    pub fn price(&self, price_id: &str) -> Option<&PaidPrice> {
        self.prices.iter().find(|price| price.price_id == price_id)
    }
}

pub async fn load_catalog(stripe: &Stripe) -> Result<Catalog, StripeError> {
    Ok(Catalog::from_list(&stripe.get_cached(PRICES_PATH).await?))
}

pub async fn find_price(stripe: &Stripe, price_id: &str) -> Result<Option<PaidPrice>, StripeError> {
    if let Some(price) = load_catalog(stripe).await?.price(price_id) {
        return Ok(Some(price.clone()));
    }
    if !is_price_id(price_id) {
        return Ok(None);
    }
    let path = format!("/v1/prices/{price_id}?expand[]=product");
    match stripe.send(Request::get(&path)).await {
        Ok(price) => Ok(plan_price(&price)),
        Err(error) if error.is_not_found() => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn is_price_id(value: &str) -> bool {
    is_id(value, "price_")
}

fn offered_price(price: &Value) -> Option<PaidPrice> {
    let offered = price["active"].as_bool() == Some(true)
        && price["type"].as_str() == Some("recurring")
        && price["product"]["active"].as_bool() == Some(true);
    offered.then(|| plan_price(price)).flatten()
}

fn plan_price(price: &Value) -> Option<PaidPrice> {
    let product = &price["product"];
    let plan_key = product["metadata"][PLAN_KEY_METADATA]
        .as_str()
        .map(str::trim)
        .filter(|key| !key.is_empty() && *key != FREE_PLAN_KEY)?;
    Some(PaidPrice {
        plan_key: plan_key.to_string(),
        plan_name: product["name"].as_str()?.to_string(),
        price_id: price["id"].as_str()?.to_string(),
        amount: price["unit_amount"].as_i64()?,
        currency: price["currency"].as_str()?.to_lowercase(),
        interval: Interval::parse(price["recurring"]["interval"].as_str()?)?,
        interval_count: price["recurring"]["interval_count"].as_i64().unwrap_or(1),
    })
}

pub fn money(amount: i64, currency: &str) -> String {
    let whole = amount / 100;
    let cents = amount % 100;
    let number = if cents == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{cents:02}")
    };
    match currency {
        "usd" => format!("${number}"),
        "eur" => format!("€{number}"),
        "gbp" => format!("£{number}"),
        other => format!("{number} {}", other.to_uppercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn price(id: &str, amount: i64, product: Value) -> Value {
        json!({
            "id": id, "active": true, "type": "recurring", "currency": "usd",
            "unit_amount": amount, "recurring": { "interval": "month", "interval_count": 1 },
            "product": product,
        })
    }

    fn product(name: &str, key: Option<&str>) -> Value {
        let metadata = key.map_or_else(|| json!({}), |key| json!({ "plan_key": key }));
        json!({ "id": format!("prod_{name}"), "active": true, "name": name, "metadata": metadata })
    }

    #[test]
    fn only_keyed_active_products_are_offered_cheapest_first() {
        let list = json!({ "data": [
            price("price_business", 4900, product("Business", Some("business"))),
            price("price_addon", 500, product("Add-on", None)),
            price("price_pro", 1200, product("Pro", Some("pro"))),
            price("price_old", 900, json!({ "active": false, "name": "Old", "metadata": { "plan_key": "old" } })),
        ] });

        let catalog = Catalog::from_list(&list);

        let keys: Vec<&str> = catalog.prices.iter().map(|p| p.plan_key.as_str()).collect();
        assert_eq!(keys, vec!["pro", "business"]);
        assert_eq!(catalog.price("price_pro").unwrap().label(), "$12 / month");
        assert_eq!(
            catalog.price("price_pro").unwrap().decimal_amount(),
            "12.00"
        );
        assert!(catalog.price("price_addon").is_none());
    }

    #[test]
    fn amounts_show_cents_only_when_there_are_some() {
        assert_eq!(money(1200, "usd"), "$12");
        assert_eq!(money(1250, "usd"), "$12.50");
        assert_eq!(money(900, "chf"), "9 CHF");
    }

    #[test]
    fn price_ids_are_checked_before_use() {
        assert!(is_price_id("price_pro_monthly"));
        assert!(!is_price_id("prod_pro"));
        assert!(!is_price_id("price_../../customers"));
        assert!(!is_price_id("price_"));
    }

    #[test]
    fn a_price_with_an_unknown_interval_is_left_out() {
        let mut odd = price("price_odd", 1200, product("Pro", Some("pro")));
        odd["recurring"]["interval"] = json!("fortnight");
        let list = json!({ "data": [odd] });

        assert!(Catalog::from_list(&list).prices.is_empty());
    }
}
