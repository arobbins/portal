pub const BILLING_PARAM: &str = "billing";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BillingPeriod {
    #[default]
    Monthly,
    Yearly,
}

impl BillingPeriod {
    pub const ALL: [Self; 2] = [Self::Monthly, Self::Yearly];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Monthly => "Monthly",
            Self::Yearly => "Yearly",
        }
    }

    pub fn from_param(value: Option<&str>) -> Self {
        match value {
            Some(value) if value == Self::Yearly.as_str() => Self::Yearly,
            _ => Self::Monthly,
        }
    }

    pub fn of_interval(interval: &str, count: i64) -> Option<Self> {
        match (interval, count) {
            ("month", 1) => Some(Self::Monthly),
            ("year", 1) => Some(Self::Yearly),
            _ => None,
        }
    }

    pub fn query(self) -> Option<String> {
        match self {
            Self::Monthly => None,
            Self::Yearly => Some(format!("{BILLING_PARAM}={}", self.as_str())),
        }
    }
}

pub fn yearly_saving_percent(monthly: i64, yearly: i64) -> Option<i64> {
    let twelve_months = monthly.checked_mul(12).filter(|total| *total > 0)?;
    let saved = twelve_months - yearly;
    (saved > 0).then(|| saved * 100 / twelve_months)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_yearly_picks_the_yearly_period() {
        assert_eq!(
            BillingPeriod::from_param(Some("yearly")),
            BillingPeriod::Yearly
        );
        assert_eq!(
            BillingPeriod::from_param(Some("monthly")),
            BillingPeriod::Monthly
        );
        assert_eq!(
            BillingPeriod::from_param(Some("YEARLY")),
            BillingPeriod::Monthly
        );
        assert_eq!(
            BillingPeriod::from_param(Some("weekly")),
            BillingPeriod::Monthly
        );
        assert_eq!(BillingPeriod::from_param(None), BillingPeriod::Monthly);
    }

    #[test]
    fn only_single_months_and_years_are_periods() {
        assert_eq!(
            BillingPeriod::of_interval("month", 1),
            Some(BillingPeriod::Monthly)
        );
        assert_eq!(
            BillingPeriod::of_interval("year", 1),
            Some(BillingPeriod::Yearly)
        );
        assert_eq!(BillingPeriod::of_interval("month", 3), None);
        assert_eq!(BillingPeriod::of_interval("week", 1), None);
    }

    #[test]
    fn monthly_is_the_bare_address() {
        assert_eq!(BillingPeriod::Monthly.query(), None);
        assert_eq!(
            BillingPeriod::Yearly.query().as_deref(),
            Some("billing=yearly")
        );
    }

    #[test]
    fn savings_round_down() {
        assert_eq!(yearly_saving_percent(1200, 12000), Some(16));
        assert_eq!(yearly_saving_percent(1000, 10000), Some(16));
        assert_eq!(yearly_saving_percent(1000, 9600), Some(20));
        assert_eq!(yearly_saving_percent(1000, 11999), Some(0));
    }

    #[test]
    fn no_saving_when_yearly_costs_the_same_or_more() {
        assert_eq!(yearly_saving_percent(1000, 12000), None);
        assert_eq!(yearly_saving_percent(1000, 13000), None);
        assert_eq!(yearly_saving_percent(0, 0), None);
    }
}
