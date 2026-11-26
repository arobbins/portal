use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    view::{NodeViewParts, PartsWriter, View, component, view},
};

use super::plan_actions::{choose_button, manage_billing_button};
use super::routes::take_billing_problem;
use crate::action::browser_caller;
use crate::app::page_title;
use crate::billing::actions::{
    Billing, BillingPage, BillingStatus, PlanPrice, UNAVAILABLE, get_billing, list_plans,
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
    let free = PlanPrice {
        plan: FREE_PLAN_KEY.to_string(),
        plan_name: FREE_PLAN_NAME.to_string(),
        price: String::new(),
        amount: 0,
        currency: "usd".to_string(),
        interval: String::new(),
        label: money(0, "usd"),
        decimal_amount: String::new(),
    };

    Ok(view! {
        <section class="l-contain p-pricing">
            page_heading(text: "Pricing")
            <p class="a-lead p-pricing__tagline">
                "Start free. Move to a paid plan when you need more, and cancel any time."
            </p>
            if let Some(message) = error {
                <div role="alert">notice(tone: Tone::Error, (message))</div>
            }
            <ul class="p-pricing__plans" role="list">
                plan_row(price: &free, billing: billing.as_ref())
                #[key(price.price.as_str())]
                for price in &plans {
                    plan_row(price: price, billing: billing.as_ref())
                }
            </ul>
            for product in products {
                (JsonLd(product))
            }
        </section>
    })
}

#[component]
async fn plan_row(cx: &Cx, price: &PlanPrice, billing: Option<&Billing>) -> Result<impl View> {
    let current_key = billing.map_or("", |billing| billing.plan.as_str());
    let is_current = current_key == price.plan;
    let status = is_current.then_some((CURRENT_PLAN, Tone::Success));
    let is_free = price.plan == FREE_PLAN_KEY;
    let can_choose =
        !is_free && billing.is_none_or(|billing| billing.status == BillingStatus::Free);
    let manages = is_current && !is_free;
    let signed_in = billing.is_some();

    Ok(view! {
        <li class="p-pricing__plan">
            " "
            card(
                card_header(title: &price.plan_name, status: status)
                card_body(
                    <p class="p-pricing__price">(price.label.as_str())</p>
                    if can_choose {
                        choose_button(price: price, signed_in: signed_in)
                    }
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
