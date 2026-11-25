use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::plan_actions::manage_billing_button;
use super::routes::take_billing_problem;
use crate::account::tabs::AccountSection;
use crate::action::require_browser_caller;
use crate::billing::actions::{BillingPage, get_billing};
use crate::components::card::{card, card_body, card_header};
use crate::components::notice::notice;
use crate::components::tone::Tone;
use crate::web::env;

#[component]
pub async fn invoices_page(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let billing = match get_billing(&env(cx), &caller).await? {
        Ok(billing) => billing,
        Err(error) => match error.for_browser()? {},
    };
    let problem = take_billing_problem(cx).await?;
    let manages = billing.configured && billing.ever_paid;

    Ok(view! {
        <div class="p-account__sub-page-content">
            if let Some(problem) = problem {
                <div role="alert">notice(tone: Tone::Error, (problem.message))</div>
            }
            card(
                id: Some(AccountSection::Invoices.anchor()),
                card_header(
                    title: AccountSection::Invoices.label(),
                    if manages {
                        manage_billing_button(from: BillingPage::Invoices)
                    }
                )
                card_body(
                    if billing.ever_paid {
                        <p>"Invoices and receipts are in Stripe."</p>
                    } else {
                        <p>"No invoices yet."</p>
                    }
                )
            )
        </div>
    })
}
