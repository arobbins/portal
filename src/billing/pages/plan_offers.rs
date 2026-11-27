use crate::billing::actions::PlanPrice;
use crate::billing::catalog::money;
use crate::billing::period::{BillingPeriod, yearly_per_month};

pub struct PlanOffer {
    first: PlanPrice,
    prices: Vec<PlanPrice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferPrices<'a> {
    Both {
        monthly: &'a PlanPrice,
        yearly: &'a PlanPrice,
    },
    YearlyOnly(&'a PlanPrice),
    Single(&'a PlanPrice),
}

impl PlanOffer {
    pub fn plan(&self) -> &PlanPrice {
        &self.first
    }

    pub fn prices(&self) -> OfferPrices<'_> {
        match (
            self.exact(BillingPeriod::Monthly),
            self.exact(BillingPeriod::Yearly),
        ) {
            (Some(monthly), Some(yearly)) => OfferPrices::Both { monthly, yearly },
            (Some(monthly), None) => OfferPrices::Single(monthly),
            (None, Some(yearly)) => OfferPrices::YearlyOnly(yearly),
            (None, None) => OfferPrices::Single(&self.first),
        }
    }

    fn exact(&self, period: BillingPeriod) -> Option<&PlanPrice> {
        self.prices
            .iter()
            .find(|price| price.period == Some(period))
    }
}

pub fn per_month_label(yearly: &PlanPrice) -> String {
    format!(
        "{} / month",
        money(yearly_per_month(yearly.amount), &yearly.currency)
    )
}

pub fn plan_offers(prices: &[PlanPrice]) -> Vec<PlanOffer> {
    let mut offers: Vec<PlanOffer> = Vec::new();
    for price in prices {
        match offers
            .iter_mut()
            .find(|offer| offer.first.plan == price.plan)
        {
            Some(offer) => offer.prices.push(price.clone()),
            None => offers.push(PlanOffer {
                first: price.clone(),
                prices: vec![price.clone()],
            }),
        }
    }
    offers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price(plan: &str, id: &str, amount: i64, interval: &str, count: i64) -> PlanPrice {
        PlanPrice {
            plan: plan.to_string(),
            plan_name: plan.to_string(),
            price: id.to_string(),
            amount,
            currency: "usd".to_string(),
            interval: interval.to_string(),
            label: String::new(),
            decimal_amount: String::new(),
            period: BillingPeriod::of_interval(interval, count),
        }
    }

    fn catalog() -> Vec<PlanPrice> {
        vec![
            price("pro", "price_pro_monthly", 1200, "month", 1),
            price("business", "price_business_monthly", 4900, "month", 1),
            price("pro", "price_pro_yearly", 12000, "year", 1),
        ]
    }

    fn ids(prices: OfferPrices<'_>) -> Vec<&str> {
        match prices {
            OfferPrices::Both { monthly, yearly } => {
                vec![monthly.price.as_str(), yearly.price.as_str()]
            }
            OfferPrices::YearlyOnly(yearly) => vec!["yearly only", yearly.price.as_str()],
            OfferPrices::Single(price) => vec![price.price.as_str()],
        }
    }

    #[test]
    fn plans_keep_the_order_of_their_cheapest_price() {
        let prices = catalog();
        let offers = plan_offers(&prices);

        let plans: Vec<&str> = offers
            .iter()
            .map(|offer| offer.plan().plan.as_str())
            .collect();
        assert_eq!(plans, vec!["pro", "business"]);
    }

    #[test]
    fn a_plan_with_both_periods_offers_both_and_others_their_one_price() {
        let prices = catalog();
        let offers = plan_offers(&prices);

        assert_eq!(
            ids(offers[0].prices()),
            vec!["price_pro_monthly", "price_pro_yearly"]
        );
        assert_eq!(ids(offers[1].prices()), vec!["price_business_monthly"]);
    }

    #[test]
    fn a_plan_with_only_a_yearly_price_is_yearly_only() {
        let prices = vec![price("max", "price_max_yearly", 50000, "year", 1)];
        let offers = plan_offers(&prices);

        assert_eq!(
            ids(offers[0].prices()),
            vec!["yearly only", "price_max_yearly"]
        );
    }

    #[test]
    fn a_plan_with_neither_period_shows_its_first_price() {
        let prices = vec![
            price("team", "price_team_quarterly", 3000, "month", 3),
            price("team", "price_team_weekly", 500, "week", 1),
        ];
        let offers = plan_offers(&prices);

        assert_eq!(ids(offers[0].prices()), vec!["price_team_quarterly"]);
    }

    #[test]
    fn a_yearly_price_is_labelled_per_month_to_the_cent() {
        assert_eq!(
            per_month_label(&price("pro", "a", 12000, "year", 1)),
            "$10 / month"
        );
        assert_eq!(
            per_month_label(&price("pro", "b", 10000, "year", 1)),
            "$8.33 / month"
        );
        assert_eq!(
            per_month_label(&price("pro", "c", 5000, "year", 1)),
            "$4.17 / month"
        );
    }
}
