use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::signal,
    view::{View, attributes, component, view},
};

use super::routes::{open_portal_submit, start_checkout_submit};
use crate::auth::handoff::PRICE_PARAM;
use crate::billing::actions::{BillingPage, PRICE_FIELD, PlanPrice};
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::form::form;
use crate::routes::LOGIN_PATH;

#[component]
pub async fn choose_button(cx: &Cx, price: &PlanPrice, signed_in: bool) -> Result<impl View> {
    let label = format!("Choose {}", price.plan_name);
    let is_submitting = signal(cx, || false);

    Ok(view! {
        if signed_in {
            form(
                submitting: Some(&is_submitting),
                attrs: attributes! {
                    method="post"
                    action=(href!(start_checkout_submit))
                    class="p-pricing__choose"
                },
                <input type="hidden" name=(PRICE_FIELD) value=(price.price.as_str())>
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    (label.as_str())
                )
            )
        } else {
            <form method="get" action=(LOGIN_PATH) class="p-pricing__choose">
                <input type="hidden" name=(PRICE_PARAM) value=(price.price.as_str())>
                button(button_type: ButtonType::Submit, (label.as_str()))
            </form>
        }
    })
}

#[component]
pub async fn manage_billing_button(cx: &Cx, from: BillingPage) -> Result<impl View> {
    let is_submitting = signal(cx, || false);

    Ok(view! {
        form(
            submitting: Some(&is_submitting),
            attrs: attributes! {
                method="post"
                action=(href!(open_portal_submit))
                class="p-billing__manage"
            },
            <input type="hidden" name="from" value=(from.as_str())>
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                button_type: ButtonType::Submit,
                busy: $(is_submitting.get()),
                "Manage billing"
            )
        )
    })
}
