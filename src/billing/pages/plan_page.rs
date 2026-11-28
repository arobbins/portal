use topcoat::{
    Result,
    context::Cx,
    runtime::{Signal, shard, signal},
    view::{View, ViewExt, component, view},
};

use super::checkout_pending::{checkout_pending, clear_checkout_pending};
use super::plan_actions::manage_billing_button;
use super::routes::take_billing_problem;
use crate::account::tabs::AccountSection;
use crate::action::{ForPage, browser_caller};
use crate::billing::actions::{Billing, BillingPage, BillingStatus, get_billing};
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::notice::notice;
use crate::components::tone::Tone;
use crate::flash::{Flash, take_flash};
use crate::routes::PRICING_PATH;
use crate::time_zone::Zone;
use crate::web::{env, session};

const FLASHES: &[Flash] = &[Flash::CheckoutCancelled];
pub const NOT_SET_UP: &str = "Billing isn't set up yet.";
const PAYMENT_RECEIVED: &str = "Payment received. Your plan will update in a moment.";

#[component]
pub async fn plan_page(cx: &Cx) -> Result<impl View> {
    let cancelled = take_flash(cx, FLASHES).await?;
    let problem = take_billing_problem(cx).await?;

    Ok(view! {
        <div class="p-account__sub-page-content">
            if let Some(problem) = problem {
                <div role="alert">notice(tone: Tone::Error, (problem.message))</div>
            }
            if let Some(flash) = cancelled {
                <div role="status">notice(tone: Tone::Info, (flash.message()))</div>
            }
            your_plan()
        </div>
    })
}

#[shard]
async fn your_plan(cx: &Cx) -> Result<impl View> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(
            view! { <p>"Your session has ended. Sign in again to see your plan."</p> }.boxed(),
        );
    };
    let env = env(cx);
    let billing = get_billing(&env, &caller).await?.for_page()?;
    let waiting = awaiting_confirmation(cx, &billing).await?;
    let checks = signal(cx, || 0u32);
    let _ = checks.get();
    let zone = caller.user.zone();
    let subscribed = billing.configured && billing.status != BillingStatus::Free;

    Ok(view! {
        card(
            id: Some(AccountSection::YourPlan.anchor()),
            card_header(
                title: AccountSection::YourPlan.label(),
                if subscribed {
                    manage_billing_button(from: BillingPage::Plan)
                }
            )
            card_body(
                if waiting {
                    payment_received(checks: &checks)
                }
                plan_summary(billing: &billing, zone: &zone)
            )
        )
    }
    .boxed())
}

#[component]
async fn plan_summary(cx: &Cx, billing: &Billing, zone: &Zone) -> Result<impl View> {
    Ok(view! {
        if !billing.configured {
            <p>(NOT_SET_UP)</p>
        } else if billing.status != BillingStatus::Free {
            subscription_details(billing: billing, zone: zone)
        } else {
            <p>"You're on the Free plan."</p>
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                href: Some(PRICING_PATH),
                "See plans"
            )
        }
    })
}

async fn awaiting_confirmation(cx: &Cx, billing: &Billing) -> Result<bool> {
    let session = session(cx)?;
    if billing.status != BillingStatus::Free {
        clear_checkout_pending(&session).await?;
        return Ok(false);
    }
    checkout_pending(&session, env(cx).now()).await
}

#[component]
async fn payment_received(cx: &Cx, checks: &Signal<u32>) -> Result<impl View> {
    Ok(view! {
        <div role="status">notice(tone: Tone::Info, (PAYMENT_RECEIVED))</div>
        <span
            class="p-billing__poll"
            aria-hidden="true"
            @animationiteration=$(|_e| checks.increment())
        ></span>
    })
}

#[component]
async fn subscription_details(cx: &Cx, billing: &Billing, zone: &Zone) -> Result<impl View> {
    let name = billing.plan_name.as_str();
    let status = format!("Status: {}", billing.status.label());
    let renews = billing
        .renews_at
        .map(|at| format!("Renews {}", zone.date(at)));
    let ends = billing
        .ends_at
        .map(|at| format!("{name} until {}, then Free.", zone.date(at)));
    let failed = (billing.status == BillingStatus::PastDue)
        .then(|| format!("Your last payment failed. Update your card to keep {name}."));

    Ok(view! {
        if let Some(warning) = &failed {
            <div role="alert">notice(tone: Tone::Warning, (warning.as_str()))</div>
        }
        <p class="p-billing__plan-name">
            "You're on the "
            <strong>(name)</strong>
            " plan."
        </p>
        <p>(status.as_str())</p>
        if let Some(ends) = &ends {
            <p>(ends.as_str())</p>
        } else if let Some(renews) = &renews {
            <p>(renews.as_str())</p>
        }
    })
}
