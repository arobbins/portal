use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanState {
    Free,
    Active,
    Cancelling,
    PastDue,
    PastDueCancelling,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripeChange {
    Subscribes,
    SchedulesCancel,
    Resumes,
    ChangesPlan,
    PaymentFails,
    Pays,
    Ends,
    Replayed,
    Late,
    Forged,
}

use PlanState as S;
use StripeChange as E;

pub const SUBSCRIPTION: Machine<PlanState, StripeChange> = Machine {
    name: "Subscription",
    summary: "An account's paid plan as signed Stripe webhooks report it: subscribed, cancelling, past due or ended; no live subscription is Free.",
    states: &[
        S::Free,
        S::Active,
        S::Cancelling,
        S::PastDue,
        S::PastDueCancelling,
        S::Ended,
    ],
    terminal: &[],
    events: &[
        E::Subscribes,
        E::SchedulesCancel,
        E::Resumes,
        E::ChangesPlan,
        E::PaymentFails,
        E::Pays,
        E::Ends,
        E::Replayed,
        E::Late,
        E::Forged,
    ],
    stored_in: &[
        "subscriptions (row)",
        "subscriptions.status",
        "subscriptions.ends_at",
        "subscriptions.plan_key",
    ],
    rules: &[
        EACH_CHANGE_IS_RECORDED,
        ONE_LIVE_SUBSCRIPTION,
        ONLY_SIGNED_EVENTS,
    ],
    writers: &[
        Writer {
            caller: "src/billing/actions/sync.rs:start_subscription",
            query: "query_insert_subscription",
            events: &[
                E::Subscribes,
                E::SchedulesCancel,
                E::Resumes,
                E::ChangesPlan,
            ],
        },
        Writer {
            caller: "src/billing/actions/sync.rs:apply_subscription",
            query: "query_update_subscription",
            events: &[E::SchedulesCancel, E::Resumes, E::ChangesPlan, E::Ends],
        },
        Writer {
            caller: "src/billing/actions/sync.rs:apply_invoice",
            query: "query_update_subscription_status",
            events: &[E::PaymentFails, E::Pays],
        },
    ],
};

pub const EACH_CHANGE_IS_RECORDED: &str = "Each change records exactly its one account event on the stripe channel (subscribed, subscription_cancelled, payment_failed once per failure, plan_changed, subscription_ended, duplicate_subscription_refunded); nothing else records one.";
pub const ONE_LIVE_SUBSCRIPTION: &str = "An account has at most one live subscription: a second one is cancelled and fully refunded at Stripe instead of stored.";
pub const ONLY_SIGNED_EVENTS: &str =
    "Only a correctly signed delivery gets in: a forged one gets 400 and changes nothing.";

const NOTHING_DELIVERED: &str = "No event has been delivered for this account yet.";

pub fn expect(from: PlanState, event: StripeChange) -> Expect<PlanState> {
    use Expect::{Moves, Refused, Unreachable};
    let live = !matches!(from, S::Free | S::Ended);
    match (from, event) {
        (S::Free, E::Replayed | E::Late) => Unreachable(NOTHING_DELIVERED),
        (_, E::Replayed | E::Late | E::Forged) => Refused,
        (S::Free | S::Ended, E::Subscribes) => Moves(S::Active),
        (S::Ended, _) => Refused,
        (_, E::Subscribes) => Refused,
        (S::Free, E::SchedulesCancel) => Moves(S::Cancelling),
        (S::Free, E::Resumes | E::ChangesPlan) => Moves(S::Active),
        (S::Free, _) => Refused,
        (_, E::ChangesPlan) => Moves(from),
        (_, E::Ends) if live => Moves(S::Ended),
        (S::Active, E::SchedulesCancel) => Moves(S::Cancelling),
        (S::Active, E::PaymentFails) => Moves(S::PastDue),
        (S::Cancelling, E::Resumes) => Moves(S::Active),
        (S::Cancelling, E::PaymentFails) => Moves(S::PastDueCancelling),
        (S::PastDue, E::SchedulesCancel) => Moves(S::PastDueCancelling),
        (S::PastDue, E::Pays) => Moves(S::Active),
        (S::PastDueCancelling, E::Resumes) => Moves(S::PastDue),
        (S::PastDueCancelling, E::Pays) => Moves(S::Cancelling),
        _ => Refused,
    }
}

impl Labeled for PlanState {
    fn id(self) -> &'static str {
        match self {
            S::Free => "free",
            S::Active => "active",
            S::Cancelling => "cancelling",
            S::PastDue => "past_due",
            S::PastDueCancelling => "past_due_cancelling",
            S::Ended => "ended",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::Free => "Free, never subscribed",
            S::Active => "Active",
            S::Cancelling => "Cancelling",
            S::PastDue => "Past due",
            S::PastDueCancelling => "Past due, cancel scheduled",
            S::Ended => "Ended (back on Free)",
        }
    }
}

impl Labeled for StripeChange {
    fn id(self) -> &'static str {
        match self {
            E::Subscribes => "subscribes",
            E::SchedulesCancel => "schedules_cancel",
            E::Resumes => "resumes",
            E::ChangesPlan => "changes_plan",
            E::PaymentFails => "payment_fails",
            E::Pays => "pays",
            E::Ends => "ends",
            E::Replayed => "replayed",
            E::Late => "late",
            E::Forged => "forged",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Subscribes => "A new subscription is created",
            E::SchedulesCancel => "Cancel at period end",
            E::Resumes => "Cancellation withdrawn",
            E::ChangesPlan => "Switches to another plan",
            E::PaymentFails => "invoice.payment_failed",
            E::Pays => "invoice.paid",
            E::Ends => "Subscription deleted",
            E::Replayed => "Stripe resends the last event",
            E::Late => "An older event arrives late",
            E::Forged => "A delivery with a wrong signature",
        }
    }
}

#[cfg(test)]
mod tests;
