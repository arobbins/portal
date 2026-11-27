use topcoat::{
    Result,
    context::Cx,
    router::{href, request},
    runtime::{Event, Signal},
    view::{View, attributes, component, view},
};

use super::plan_actions::choose_button;
use super::plan_offers::{OfferPrices, PlanOffer, per_month_label};
use super::pricing_page::pricing_page;
use crate::billing::actions::{Billing, BillingStatus, PlanPrice};
use crate::billing::period::{BILLING_PARAM, BillingPeriod};
use crate::components::toggle::toggle;

const BILLED_YEARLY: &str = "Billed yearly";
const PRICE_CLASS: &str = "p-pricing__price";
const TO_YEARLY_CLASS: &str = "p-pricing__price p-pricing__price--to-yearly";
const TO_MONTHLY_CLASS: &str = "p-pricing__price p-pricing__price--to-monthly";

pub fn picked_period(cx: &Cx) -> BillingPeriod {
    let value = request::uri(cx).query().and_then(|query| {
        url::form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == BILLING_PARAM)
            .map(|(_, value)| value.into_owned())
    });
    BillingPeriod::from_param(value.as_deref())
}

fn period_href(cx: &Cx, period: BillingPeriod) -> String {
    let base = href!(pricing_page).resolve(cx);
    match period.query() {
        Some(query) => format!("{base}?{query}"),
        None => base.to_string(),
    }
}

#[component]
pub async fn offer_prices(
    cx: &Cx,
    offer: &PlanOffer,
    picked: BillingPeriod,
    yearly: &Signal<bool>,
    flipped: &Signal<bool>,
    billing: Option<&Billing>,
) -> Result<impl View> {
    let can_choose = billing.is_none_or(|billing| billing.status == BillingStatus::Free);
    let signed_in = billing.is_some();
    let plan_name = offer.plan().plan_name.as_str();
    let (switched, single) = match offer.prices() {
        OfferPrices::Both {
            monthly,
            yearly: price,
        } => (Some((monthly, price)), None),
        OfferPrices::YearlyOnly(price) => (None, Some((per_month_label(price), true, price))),
        OfferPrices::Single(price) => (None, Some((price.label.clone(), false, price))),
    };

    Ok(view! {
        <div class="p-pricing__offer">
            if let Some((monthly_price, yearly_price)) = switched {
                switched_prices(
                    monthly: monthly_price,
                    yearly: yearly_price,
                    picked: picked,
                    is_yearly: yearly,
                    flipped: flipped,
                    choose: can_choose.then_some(signed_in)
                )
            }
            if let Some((label, billed_yearly, price)) = single {
                <p class=(PRICE_CLASS)>(label.as_str())</p>
                if billed_yearly {
                    <p class="p-pricing__billed">(BILLED_YEARLY)</p>
                }
                if can_choose {
                    choose_button(
                        plan_name: plan_name,
                        price: price.price.clone(),
                        signed_in: signed_in
                    )
                }
            }
        </div>
    })
}

#[component]
async fn switched_prices(
    cx: &Cx,
    monthly: &PlanPrice,
    yearly: &PlanPrice,
    picked: BillingPeriod,
    is_yearly: &Signal<bool>,
    flipped: &Signal<bool>,
    choose: Option<bool>,
) -> Result<impl View> {
    let per_month = per_month_label(yearly);
    let monthly_label = monthly.label.clone();
    let yearly_price = yearly.price.clone();
    let monthly_price = monthly.price.clone();
    Ok(view! {
        <p
            :class=$(if !flipped.get() {
                PRICE_CLASS
            } else if is_yearly.get() {
                TO_YEARLY_CLASS
            } else {
                TO_MONTHLY_CLASS
            })
        >
            $(if is_yearly.get() { per_month.clone() } else { monthly_label.clone() })
        </p>
        billed_yearly_switch(
            plan: &yearly.plan,
            picked: picked,
            is_yearly: is_yearly,
            flipped: flipped
        )
        if let Some(signed_in) = choose {
            choose_button(
                plan_name: &yearly.plan_name,
                price: $(if is_yearly.get() {
                    yearly_price.clone()
                } else {
                    monthly_price.clone()
                }),
                signed_in: signed_in
            )
        }
    })
}

#[component]
async fn billed_yearly_switch(
    cx: &Cx,
    plan: &str,
    picked: BillingPeriod,
    is_yearly: &Signal<bool>,
    flipped: &Signal<bool>,
) -> Result<impl View> {
    let yearly_href = period_href(cx, BillingPeriod::Yearly);
    let monthly_href = period_href(cx, BillingPeriod::Monthly);
    let (other_href, other_label) = if picked == BillingPeriod::Yearly {
        (monthly_href.clone(), "Show monthly prices")
    } else {
        (yearly_href.clone(), "Show yearly prices")
    };
    let id = format!("billed-yearly-{plan}");

    Ok(view! {
        <div class="p-pricing__billing">
            toggle(
                label: BILLED_YEARLY,
                id: &id,
                name: BILLING_PARAM,
                checked: $(is_yearly.get()),
                attrs: attributes! {
                    @change=$(|e: Event| {
                        let on = e.current_target.checked;
                        flipped.set(true);
                        is_yearly.set(on);
                        let _address = if on {
                            yearly_href.clone()
                        } else {
                            monthly_href.clone()
                        };
                        raw!(
                            "history.replaceState(history.state, '', String(${_address}))",
                            (),
                        )
                    })
                }
            )
            <noscript>
                <a class="p-pricing__period-link" href=(other_href.as_str())>
                    (other_label)
                </a>
            </noscript>
        </div>
    })
}
