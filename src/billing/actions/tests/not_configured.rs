use sqlx::PgPool;

use super::*;
use crate::action::ActionError;
use crate::billing::actions::{
    BillingPage, BillingStatus, CheckoutProblem, OpenPortal, PortalProblem, StartCheckout,
    get_billing, list_plans, open_billing_portal, receive_stripe_event, start_checkout,
};

#[sqlx::test]
async fn without_stripe_keys_billing_is_off_and_nothing_breaks(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("unbilled@example.test", &db).await;
    let caller = Caller::browser(user.clone());

    assert!(app.config.stripe.is_none());
    assert!(list_plans(&env).await.unwrap().unwrap().data.is_empty());
    let billing = get_billing(&env, &caller).await.unwrap().unwrap();
    assert_eq!(billing.status, BillingStatus::Free);
    assert!(!billing.configured);
    let pro = StartCheckout {
        price: "price_pro".to_string(),
    };
    assert_eq!(
        start_checkout(&env, &caller, &pro).await.unwrap(),
        Err(ActionError::Problem(CheckoutProblem::Unavailable))
    );
    let portal = OpenPortal {
        from: BillingPage::Plan,
    };
    assert_eq!(
        open_billing_portal(&env, &caller, &portal).await.unwrap(),
        Err(ActionError::Problem(PortalProblem::Unavailable))
    );
    let reply = receive_stripe_event(&env, Some("t=1,v1=00"), b"{}")
        .await
        .unwrap();
    assert_eq!(reply, WebhookReply::NotConfigured);
}
