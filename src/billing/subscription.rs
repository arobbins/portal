use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Cancelling,
    PastDue,
    Ended,
}

impl SubscriptionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Cancelling => "cancelling",
            Self::PastDue => "past_due",
            Self::Ended => "ended",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Cancelling => "Cancelling",
            Self::PastDue => "Past due",
            Self::Ended => "Ended",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub stripe_subscription_id: String,
    pub plan_key: String,
    pub plan_name: String,
    pub price_id: String,
    pub status: SubscriptionStatus,
    pub current_period_end: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub last_event_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionState {
    pub stripe_subscription_id: String,
    pub customer_id: String,
    pub plan_key: String,
    pub plan_name: String,
    pub price_id: String,
    pub status: SubscriptionStatus,
    pub current_period_end: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
}
