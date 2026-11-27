use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffState {
    SignedOut,
    ChoseCheckout,
    ChoseAuthorize,
    SignedIn,
    AwaitingPayment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffEvent {
    OpensSignInWithPrice,
    OpensAppRequest,
    SignsIn,
    RegistersAndConfirms,
    WaitsAnHour,
    ReturnsFromCheckout,
    SubscriptionArrives,
    SignsOut,
}

use HandoffEvent as E;
use HandoffState as S;

pub const SIGN_IN_HANDOFF: Machine<HandoffState, HandoffEvent> = Machine {
    name: "Sign-in hand-off",
    summary: "Where a browser goes after signing in when it came to pay for a plan or to connect an app, and the Plan card's wait for the payment to arrive.",
    states: &[
        S::SignedOut,
        S::ChoseCheckout,
        S::ChoseAuthorize,
        S::SignedIn,
        S::AwaitingPayment,
    ],
    terminal: &[],
    events: &[
        E::OpensSignInWithPrice,
        E::OpensAppRequest,
        E::SignsIn,
        E::RegistersAndConfirms,
        E::WaitsAnHour,
        E::ReturnsFromCheckout,
        E::SubscriptionArrives,
        E::SignsOut,
    ],
    stored_in: &["session:AFTER_SIGN_IN", "session:CHECKOUT_PENDING"],
    rules: &[GOES_WHERE_CHOSEN, NOTICE_ONLY_WHILE_AWAITING],
    writers: &[
        Writer {
            caller: "src/auth/pages/login.rs:login_page",
            query: "remember_handoff",
            events: &[E::OpensSignInWithPrice, E::OpensAppRequest],
        },
        Writer {
            caller: "src/auth/pages/verify_email.rs:set_password_submit",
            query: "remember_handoff",
            events: &[E::RegistersAndConfirms],
        },
        Writer {
            caller: "src/auth/handoff.rs:destination_after_sign_in",
            query: "take_handoff",
            events: &[E::SignsIn, E::RegistersAndConfirms],
        },
        Writer {
            caller: "src/billing/pages/routes.rs:checkout_paid",
            query: "mark_checkout_pending",
            events: &[E::ReturnsFromCheckout],
        },
        Writer {
            caller: "src/billing/pages/plan_page.rs:awaiting_confirmation",
            query: "clear_checkout_pending",
            events: &[E::SubscriptionArrives],
        },
    ],
};

pub const GOES_WHERE_CHOSEN: &str = "Signing in or confirming a registration goes to Checkout for a chosen price, to the approval screen for a chosen app, and otherwise to the passkey prompt or the account; the choice is taken before the prompt.";
pub const NOTICE_ONLY_WHILE_AWAITING: &str =
    "The Plan card says the payment was received only while it waits for the subscription.";

const SIGNED_IN_ALREADY: &str = "Register is offered only while signed out.";

pub fn expect(from: HandoffState, event: HandoffEvent) -> Expect<HandoffState> {
    use Expect::{Moves, Refused, Unreachable};
    let signed_in = matches!(from, S::SignedIn | S::AwaitingPayment);
    match (from, event) {
        (_, E::RegistersAndConfirms) if signed_in => Unreachable(SIGNED_IN_ALREADY),
        (_, E::OpensSignInWithPrice | E::OpensAppRequest | E::SignsIn) if signed_in => Refused,
        (_, E::OpensSignInWithPrice) => Moves(S::ChoseCheckout),
        (_, E::OpensAppRequest) => Moves(S::ChoseAuthorize),
        (_, E::SignsIn | E::RegistersAndConfirms) => Moves(S::SignedIn),
        (S::ChoseCheckout | S::ChoseAuthorize, E::WaitsAnHour | E::SignsOut) => Moves(S::SignedOut),
        (S::SignedIn, E::ReturnsFromCheckout) => Moves(S::AwaitingPayment),
        (S::AwaitingPayment, E::WaitsAnHour | E::SubscriptionArrives) => Moves(S::SignedIn),
        (S::SignedIn | S::AwaitingPayment, E::SignsOut) => Moves(S::SignedOut),
        _ => Refused,
    }
}

impl Labeled for HandoffState {
    fn id(self) -> &'static str {
        match self {
            S::SignedOut => "signed_out",
            S::ChoseCheckout => "chose_checkout",
            S::ChoseAuthorize => "chose_authorize",
            S::SignedIn => "signed_in",
            S::AwaitingPayment => "awaiting_payment",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::SignedOut => "Signed out, nothing chosen",
            S::ChoseCheckout => "Signed out, chose a plan",
            S::ChoseAuthorize => "Signed out, connecting an app",
            S::SignedIn => "Signed in, nothing pending",
            S::AwaitingPayment => "Back from Checkout, waiting",
        }
    }
}

impl Labeled for HandoffEvent {
    fn id(self) -> &'static str {
        match self {
            E::OpensSignInWithPrice => "opens_sign_in_with_price",
            E::OpensAppRequest => "opens_app_request",
            E::SignsIn => "signs_in",
            E::RegistersAndConfirms => "registers_and_confirms",
            E::WaitsAnHour => "waits_an_hour",
            E::ReturnsFromCheckout => "returns_from_checkout",
            E::SubscriptionArrives => "subscription_arrives",
            E::SignsOut => "signs_out",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::OpensSignInWithPrice => "Opens /login?price=",
            E::OpensAppRequest => "An app sends the browser to /oauth/authorize",
            E::SignsIn => "Signs in with a password",
            E::RegistersAndConfirms => "Registers and sets a password from the link",
            E::WaitsAnHour => "An hour passes",
            E::ReturnsFromCheckout => "Stripe sends the browser back paid",
            E::SubscriptionArrives => "The subscription webhook lands, then the Plan page loads",
            E::SignsOut => "Signs out",
        }
    }
}

#[cfg(test)]
mod tests;
