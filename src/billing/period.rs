use schemars::JsonSchema;
use serde::Serialize;

pub const BILLING_PARAM: &str = "billing";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    Day,
    Week,
    Month,
    Year,
}

impl Interval {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "day" => Some(Self::Day),
            "week" => Some(Self::Week),
            "month" => Some(Self::Month),
            "year" => Some(Self::Year),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }
}

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

    pub fn of_interval(interval: Interval, count: i64) -> Option<Self> {
        match (interval, count) {
            (Interval::Month, 1) => Some(Self::Monthly),
            (Interval::Year, 1) => Some(Self::Yearly),
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
    fn intervals_round_trip_through_their_stripe_names() {
        for interval in [
            Interval::Day,
            Interval::Week,
            Interval::Month,
            Interval::Year,
        ] {
            assert_eq!(Interval::parse(interval.as_str()), Some(interval));
        }
        assert_eq!(Interval::parse("fortnight"), None);
    }

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
            BillingPeriod::of_interval(Interval::Month, 1),
            Some(BillingPeriod::Monthly)
        );
        assert_eq!(
            BillingPeriod::of_interval(Interval::Year, 1),
            Some(BillingPeriod::Yearly)
        );
        assert_eq!(BillingPeriod::of_interval(Interval::Month, 3), None);
        assert_eq!(BillingPeriod::of_interval(Interval::Week, 1), None);
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
