use sqlx::PgPool;

use super::*;
use crate::action::ActionError;
use crate::billing::actions::{
    BillingPage, CheckoutProblem, OpenPortal, PlansProblem, PortalProblem, StartCheckout,
    list_plans, open_billing_portal, start_checkout,
};
use crate::billing::testing::FakeStripe;
use crate::rate_limit::{BILLING_SESSIONS_PER_ACCOUNT_PER_HOUR, PLAN_LISTS_PER_IP_PER_MINUTE};

#[sqlx::test]
async fn checkouts_and_portal_visits_share_ten_an_hour_per_account(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = active_user("eager@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let env = app.env(None);
    let pro = StartCheckout {
        price: "price_pro".to_string(),
    };

    assert_eq!(BILLING_SESSIONS_PER_ACCOUNT_PER_HOUR, 10);
    for _ in 0..BILLING_SESSIONS_PER_ACCOUNT_PER_HOUR {
        assert!(start_checkout(&env, &caller, &pro).await.unwrap().is_ok());
    }

    assert_eq!(
        start_checkout(&env, &caller, &pro).await.unwrap(),
        Err(ActionError::Problem(CheckoutProblem::TooManyAttempts))
    );
    let portal = OpenPortal {
        from: BillingPage::Plan,
    };
    assert_eq!(
        open_billing_portal(&env, &caller, &portal).await.unwrap(),
        Err(ActionError::Problem(PortalProblem::TooManyAttempts))
    );
    let someone_else = Caller::browser(active_user("patient@example.test", &db).await);
    assert!(
        start_checkout(&env, &someone_else, &pro)
            .await
            .unwrap()
            .is_ok()
    );
}

#[sqlx::test]
async fn the_public_plans_list_allows_sixty_a_minute_per_ip(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let visitor = app.env("203.0.113.7".parse().ok());

    assert_eq!(PLAN_LISTS_PER_IP_PER_MINUTE, 60);
    for _ in 0..PLAN_LISTS_PER_IP_PER_MINUTE {
        let plans = list_plans(&visitor).await.unwrap().unwrap();
        let keys: Vec<&str> = plans.data.iter().map(|price| price.plan.as_str()).collect();
        assert_eq!(keys, ["pro", "business"]);
    }

    assert_eq!(
        list_plans(&visitor).await.unwrap(),
        Err(PlansProblem::TooManyRequests)
    );
    let neighbour = app.env("203.0.113.8".parse().ok());
    assert!(list_plans(&neighbour).await.unwrap().is_ok());
    assert_eq!(stripe.seen_at("GET", "/v1/prices").len(), 1);
}
