mod checkout;
mod customer;
mod overview;
mod plans;
mod portal;
mod sync;
mod webhook;

pub use checkout::{CheckoutLink, CheckoutProblem, PRICE_FIELD, StartCheckout, start_checkout};
pub use overview::{AdminBilling, Billing, BillingStatus, admin_billing, get_billing};
pub use plans::{PlanList, PlanPrice, PlansProblem, list_plans};
pub use portal::{BillingPage, OpenPortal, PortalProblem, open_billing_portal};
pub use webhook::receive_stripe_event;

use crate::action::{ActionProblem, ProblemSpec, TooManyAttempts};
use topcoat::router::StatusCode;

pub const UNAVAILABLE: &str = "Billing is unavailable right now. Try again in a few minutes.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailable;

impl ActionProblem for Unavailable {
    fn spec(self) -> ProblemSpec {
        ProblemSpec::new("billing_unavailable", UNAVAILABLE)
            .with_status(StatusCode::SERVICE_UNAVAILABLE)
    }
}

fn too_many_attempts() -> ProblemSpec {
    TooManyAttempts.spec()
}

#[cfg(test)]
mod tests;
