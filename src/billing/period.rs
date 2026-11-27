pub const BILLING_PARAM: &str = "billing";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BillingPeriod {
    Monthly,
    #[default]
    Yearly,
}

impl BillingPeriod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }

    pub fn from_param(value: Option<&str>) -> Self {
        match value {
            Some(value) if value == Self::Monthly.as_str() => Self::Monthly,
            _ => Self::Yearly,
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
            Self::Yearly => None,
            Self::Monthly => Some(format!("{BILLING_PARAM}={}", self.as_str())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_monthly_picks_the_monthly_period() {
        assert_eq!(
            BillingPeriod::from_param(Some("monthly")),
            BillingPeriod::Monthly
        );
        assert_eq!(
            BillingPeriod::from_param(Some("yearly")),
            BillingPeriod::Yearly
        );
        assert_eq!(
            BillingPeriod::from_param(Some("MONTHLY")),
            BillingPeriod::Yearly
        );
        assert_eq!(
            BillingPeriod::from_param(Some("weekly")),
            BillingPeriod::Yearly
        );
        assert_eq!(BillingPeriod::from_param(None), BillingPeriod::Yearly);
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
    fn yearly_is_the_bare_address() {
        assert_eq!(BillingPeriod::Yearly.query(), None);
        assert_eq!(
            BillingPeriod::Monthly.query().as_deref(),
            Some("billing=monthly")
        );
    }
}
