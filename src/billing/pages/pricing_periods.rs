use topcoat::{
    Result,
    context::Cx,
    router::{href, request},
    runtime::Signal,
    view::{View, component, view},
};

use super::plan_actions::choose_button;
use super::plan_offers::{PlanOffer, yearly_saving};
use super::pricing_page::pricing_page;
use crate::billing::actions::{Billing, BillingStatus, PlanPrice};
use crate::billing::period::{BILLING_PARAM, BillingPeriod};
use crate::components::segmented::{Segment, segmented};

pub fn picked_period(cx: &Cx) -> BillingPeriod {
    let value = request::uri(cx).query().and_then(|query| {
        url::form_urlencoded::parse(query.as_bytes())
            .find(|(key, _)| key == BILLING_PARAM)
            .map(|(_, value)| value.into_owned())
    });
    BillingPeriod::from_param(value.as_deref())
}

#[component]
pub async fn period_switch(
    cx: &Cx,
    offers: &[PlanOffer],
    period: &Signal<String>,
) -> Result<impl View> {
    let base = href!(pricing_page).resolve(cx);
    let saving = yearly_saving(offers).map(|percent| format!("Save {percent}%"));
    let segments: Vec<Segment> = BillingPeriod::ALL
        .into_iter()
        .map(|choice| Segment {
            value: choice.as_str().to_string(),
            href: match choice.query() {
                Some(query) => format!("{base}?{query}"),
                None => base.to_string(),
            },
            label: choice.label().to_string(),
            note: match choice {
                BillingPeriod::Yearly => saving.clone(),
                BillingPeriod::Monthly => None,
            },
        })
        .collect();

    Ok(view! {
        <div class="p-pricing__periods">
            segmented(label: "Billing period", segments: &segments, selected: period)
        </div>
    })
}

#[component]
pub async fn offer_prices(
    cx: &Cx,
    offer: &PlanOffer,
    period: &Signal<String>,
    billing: Option<&Billing>,
) -> Result<impl View> {
    let monthly = offer.shown(BillingPeriod::Monthly);
    let yearly = offer.shown(BillingPeriod::Yearly);
    let shown: Vec<(&PlanPrice, &str)> = if monthly.price == yearly.price {
        vec![(monthly, "")]
    } else {
        vec![
            (monthly, BillingPeriod::Monthly.as_str()),
            (yearly, BillingPeriod::Yearly.as_str()),
        ]
    };
    let can_choose = billing.is_none_or(|billing| billing.status == BillingStatus::Free);
    let signed_in = billing.is_some();

    Ok(view! {
        #[key(price.price.as_str())]
        for (price, only) in shown {
            offer_price(
                price: price,
                only: only,
                period: period,
                can_choose: can_choose,
                signed_in: signed_in
            )
        }
    })
}

#[component]
async fn offer_price(
    cx: &Cx,
    price: &PlanPrice,
    only: &str,
    period: &Signal<String>,
    can_choose: bool,
    signed_in: bool,
) -> Result<impl View> {
    let only = only.to_string();

    Ok(view! {
        <div
            class="p-pricing__offer"
            :hidden=$(if only.is_empty() { false } else { period.get() != only })
        >
            <p class="p-pricing__price">(price.label.as_str())</p>
            if can_choose {
                choose_button(price: price, signed_in: signed_in)
            }
        </div>
    })
}
