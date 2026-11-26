use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    runtime::signal,
    view::{Child, NodeViewParts, PartsWriter, View, component, view},
};

use super::plan_actions::manage_billing_button;
use super::plan_offers::{offers_both_periods, plan_offers};
use super::pricing_periods::{offer_prices, period_switch, picked_period};
use super::routes::take_billing_problem;
use crate::action::browser_caller;
use crate::app::page_title;
use crate::billing::actions::{
    Billing, BillingPage, PlanPrice, UNAVAILABLE, get_billing, list_plans,
};
use crate::billing::catalog::{FREE_PLAN_KEY, FREE_PLAN_NAME, money};
use crate::components::card::{card, card_body, card_header};
use crate::components::notice::notice;
use crate::components::tone::Tone;
use crate::layout::page_heading::page_heading;
use crate::web::env;

const CURRENT_PLAN: &str = "Current plan";

pub struct JsonLd(String);

impl NodeViewParts for JsonLd {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        let safe = self.0.replace('<', "\\u003c");
        parts.push_string_unescaped(format!(
            "<script type=\"application/ld+json\">{safe}</script>"
        ));
    }
}

#[page("/pricing", with(Metadata::new().title(page_title("Pricing"))))]
pub async fn pricing_page(cx: &Cx) -> Result<impl View> {
    let env = env(cx);
    let (plans, unavailable) = match list_plans(&env).await? {
        Ok(list) => (list.data, false),
        Err(_) => (Vec::new(), true),
    };
    let billing = match browser_caller(cx) {
        Some(caller) => get_billing(&env, &caller).await?.ok(),
        None => None,
    };
    let problem = take_billing_problem(cx).await?;
    let error = problem
        .map(|spec| spec.message)
        .or(unavailable.then_some(UNAVAILABLE));
    let products = json_ld(&plans);
    let offers = plan_offers(&plans);
    let picked = picked_period(cx);
    let period = signal(cx, || picked.as_str().to_string());
    let free_label = money(0, "usd");

    Ok(view! {
        <section class="l-contain p-pricing">
            page_heading(text: "Pricing")
            <p class="a-lead p-pricing__tagline">
                "Start free. Move to a paid plan when you need more, and cancel any time."
            </p>
            if let Some(message) = error {
                <div role="alert">notice(tone: Tone::Error, (message))</div>
            }
            if offers_both_periods(&offers) {
                period_switch(offers: &offers, period: &period)
            }
            <ul class="p-pricing__plans" role="list">
                plan_row(
                    name: FREE_PLAN_NAME,
                    key: FREE_PLAN_KEY,
                    billing: billing.as_ref(),
                    <p class="p-pricing__price">(free_label.as_str())</p>
                )
                #[key(offer.plan().plan.as_str())]
                for offer in &offers {
                    plan_row(
                        name: &offer.plan().plan_name,
                        key: &offer.plan().plan,
                        billing: billing.as_ref(),
                        offer_prices(
                            offer: offer,
                            period: &period,
                            billing: billing.as_ref()
                        )
                    )
                }
            </ul>
            for product in products {
                (JsonLd(product))
            }
        </section>
    })
}

#[component]
async fn plan_row(
    cx: &Cx,
    name: &str,
    key: &str,
    billing: Option<&Billing>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let is_current = billing.is_some_and(|billing| billing.plan == key);
    let status = is_current.then_some((CURRENT_PLAN, Tone::Success));
    let manages = is_current && key != FREE_PLAN_KEY;

    Ok(view! {
        <li class="p-pricing__plan">
            " "
            card(
                card_header(title: name, status: status)
                card_body(
                    (child)
                    if manages {
                        manage_billing_button(from: BillingPage::Pricing)
                    }
                )
            )
            " "
        </li>
    })
}

fn json_ld(plans: &[PlanPrice]) -> Vec<String> {
    let mut products: Vec<(String, Value)> = Vec::new();
    for price in plans {
        let offer = json!({
            "@type": "Offer",
            "price": price.decimal_amount,
            "priceCurrency": price.currency.to_uppercase(),
            "category": "subscription",
        });
        match products.iter_mut().find(|(key, _)| *key == price.plan) {
            Some((_, product)) => product["offers"]
                .as_array_mut()
                .into_iter()
                .for_each(|offers| offers.push(offer.clone())),
            None => products.push((
                price.plan.clone(),
                json!({
                    "@context": "https://schema.org",
                    "@type": "Product",
                    "name": price.plan_name,
                    "offers": [offer],
                }),
            )),
        }
    }
    products
        .into_iter()
        .map(|(_, product)| product.to_string())
        .collect()
}
