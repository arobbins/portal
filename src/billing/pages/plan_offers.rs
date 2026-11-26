use crate::billing::actions::PlanPrice;
use crate::billing::period::{BillingPeriod, yearly_saving_percent};

pub struct PlanOffer {
    first: PlanPrice,
    prices: Vec<PlanPrice>,
}

impl PlanOffer {
    pub fn plan(&self) -> &PlanPrice {
        &self.first
    }

    pub fn shown(&self, period: BillingPeriod) -> &PlanPrice {
        self.exact(period)
            .or_else(|| self.exact(BillingPeriod::Monthly))
            .unwrap_or(&self.first)
    }

    fn exact(&self, period: BillingPeriod) -> Option<&PlanPrice> {
        self.prices
            .iter()
            .find(|price| price.period == Some(period))
    }

    fn both(&self) -> Option<(&PlanPrice, &PlanPrice)> {
        Some((
            self.exact(BillingPeriod::Monthly)?,
            self.exact(BillingPeriod::Yearly)?,
        ))
    }
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

pub fn offers_both_periods(offers: &[PlanOffer]) -> bool {
    offers.iter().any(|offer| offer.both().is_some())
}

pub fn yearly_saving(offers: &[PlanOffer]) -> Option<i64> {
    offers
        .iter()
        .filter_map(PlanOffer::both)
        .map(|(monthly, yearly)| yearly_saving_percent(monthly.amount, yearly.amount).unwrap_or(0))
        .min()
        .filter(|percent| *percent >= 1)
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
    fn each_plan_shows_the_picked_period_or_its_monthly_price() {
        let prices = catalog();
        let offers = plan_offers(&prices);

        assert_eq!(
            offers[0].shown(BillingPeriod::Monthly).price,
            "price_pro_monthly"
        );
        assert_eq!(
            offers[0].shown(BillingPeriod::Yearly).price,
            "price_pro_yearly"
        );
        assert_eq!(
            offers[1].shown(BillingPeriod::Yearly).price,
            "price_business_monthly"
        );
    }

    #[test]
    fn a_plan_with_neither_period_shows_its_first_price() {
        let prices = vec![
            price("team", "price_team_quarterly", 3000, "month", 3),
            price("team", "price_team_weekly", 500, "week", 1),
        ];
        let offers = plan_offers(&prices);

        assert_eq!(
            offers[0].shown(BillingPeriod::Yearly).price,
            "price_team_quarterly"
        );
        assert!(!offers_both_periods(&offers));
    }

    #[test]
    fn the_saving_ignores_plans_without_a_yearly_price() {
        let prices = catalog();
        let offers = plan_offers(&prices);

        assert!(offers_both_periods(&offers));
        assert_eq!(yearly_saving(&offers), Some(16));
    }

    #[test]
    fn the_smallest_saving_is_shown() {
        let mut prices = catalog();
        prices.push(price("business", "price_business_yearly", 39200, "year", 1));
        let offers = plan_offers(&prices);

        assert_eq!(yearly_saving(&offers), Some(16));

        prices.push(price("max", "price_max_monthly", 10000, "month", 1));
        prices.push(price("max", "price_max_yearly", 119900, "year", 1));
        let offers = plan_offers(&prices);

        assert_eq!(yearly_saving(&offers), None);
    }

    #[test]
    fn no_saving_without_a_plan_on_both_periods() {
        let prices = vec![price("pro", "price_pro_monthly", 1200, "month", 1)];
        let offers = plan_offers(&prices);

        assert!(!offers_both_periods(&offers));
        assert_eq!(yearly_saving(&offers), None);
    }
}
