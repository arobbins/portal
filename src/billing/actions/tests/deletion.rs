use sqlx::PgPool;

use super::*;
use crate::auth::actions::{DeleteAccount, delete_account};
use crate::billing::jobs::CancelStripeSubscription;
use crate::billing::testing::{FakeStripe, unreachable_stripe};
use crate::jobs::{Handler, Job, Worker};
use crate::user::testing::TEST_PASSWORD;

pub(super) fn confirmed() -> DeleteAccount {
    DeleteAccount {
        confirmation: "DELETE".to_string(),
        current_password: TEST_PASSWORD.to_string(),
    }
}

pub(super) async fn queued_cancellations(db: &PgPool) -> Vec<(Option<String>, i32)> {
    sqlx::query!(
        "SELECT payload->>'subscription' AS subscription, max_attempts FROM jobs WHERE kind = 'cancel_stripe_subscription'"
    )
    .fetch_all(db)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.subscription, row.max_attempts))
    .collect()
}

async fn subscribe(email: &str, db: &PgPool) -> Arc<User> {
    let stripe = FakeStripe::start().await;
    let app = &stripe.app(db);
    let user = customer(email, CUSTOMER, db).await;
    let created = subscription(
        "customer.subscription.created",
        "evt_1",
        1_800_000_000,
        "active",
        false,
    );
    deliver(app, &created).await;
    user
}

#[sqlx::test]
async fn deleting_a_subscribed_account_queues_the_cancellation_and_still_deletes(db: PgPool) {
    let app = unreachable_stripe(&db);
    let user = subscribe("leaving@example.test", &db).await;

    let outcome = delete_account(
        &app.env(None),
        &Caller::browser(user.clone()),
        &confirmed(),
        None,
    )
    .await
    .unwrap();

    assert_eq!(outcome, Ok(()));
    let left = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM users WHERE id = $1"#,
        user.id
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(left, 0);
    assert_eq!(
        queued_cancellations(&db).await,
        vec![(
            Some("sub_1".to_string()),
            CancelStripeSubscription::MAX_ATTEMPTS
        )]
    );
}

#[sqlx::test]
async fn the_cancellation_is_retried_until_stripe_answers(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = subscribe("retried@example.test", &db).await;
    delete_account(&app.env(None), &Caller::browser(user), &confirmed(), None)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("DELETE FROM jobs WHERE kind = 'send_email'")
        .execute(&db)
        .await
        .unwrap();
    stripe.fail_next_deletes(2);
    let worker = Worker::new(app.clone(), vec![Handler::of::<CancelStripeSubscription>()]);

    for _ in 0..3 {
        assert!(worker.run_next().await.unwrap());
        sqlx::query!("UPDATE jobs SET run_at = now()")
            .execute(&db)
            .await
            .unwrap();
    }

    assert_eq!(stripe.seen_at("DELETE", "/v1/subscriptions/sub_1").len(), 3);
    assert!(queued_cancellations(&db).await.is_empty());
    const { assert!(CancelStripeSubscription::MAX_ATTEMPTS >= 50) };
}

#[sqlx::test]
async fn deleting_a_free_account_cancels_nothing(db: PgPool) {
    let app = unreachable_stripe(&db);
    let user = active_user("free-leaver@example.test", &db).await;

    delete_account(&app.env(None), &Caller::browser(user), &confirmed(), None)
        .await
        .unwrap()
        .unwrap();

    assert!(queued_cancellations(&db).await.is_empty());
}

const GONE: &str = "cus_gone";

fn live_for(customer: Value) -> (Value, Value) {
    let event = subscription_for(
        GONE,
        "sub_late",
        "customer.subscription.created",
        "evt_late",
        1_800_000_000,
        "active",
        false,
    );
    let mut current = event["data"]["object"].clone();
    current["customer"] = customer;
    (event, current)
}

fn tagged(user: &str, app: &str) -> Value {
    json!({ "id": GONE, "object": "customer", "metadata": { "user": user, "app": app } })
}

async fn queued_refunds(db: &PgPool) -> Vec<Value> {
    sqlx::query_scalar!("SELECT payload FROM jobs WHERE kind = 'cancel_stripe_subscription'")
        .fetch_all(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_subscription_paid_after_the_account_is_gone_is_cancelled_and_refunded(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let (event, current) = live_for(tagged("usr_deletedlongago000000", app.config.origin()));
    stripe.set_subscription("sub_late", current);

    let reply = deliver(&app, &event).await;

    assert_eq!(reply.status().as_u16(), 200);
    assert_eq!(
        queued_refunds(&db).await,
        [json!({ "subscription": "sub_late", "refund": true })]
    );
    assert_eq!(deliver(&app, &event).await.status().as_u16(), 200);
    assert_eq!(queued_refunds(&db).await.len(), 1);
}

#[sqlx::test]
async fn subscriptions_of_customers_this_app_did_not_create_are_left_alone(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let alive = active_user("still-here@example.test", &db).await;
    let origin = app.config.origin().to_string();
    let strangers = [
        json!(GONE),
        json!({ "id": GONE, "object": "customer", "metadata": {} }),
        tagged("usr_deletedlongago000000", "https://another-app.example"),
        tagged("someone-else", &origin),
        tagged(&alive.public_id, &origin),
    ];

    for (n, customer) in strangers.into_iter().enumerate() {
        let (mut event, current) = live_for(customer);
        event["id"] = json!(format!("evt_stranger_{n}"));
        stripe.set_subscription("sub_late", current);
        let reply = deliver(&app, &event).await;
        assert_eq!(reply.status().as_u16(), 200);
    }
    let (mut as_sent, _) = live_for(json!(GONE));
    as_sent["id"] = json!("evt_not_found");
    as_sent["data"]["object"]["id"] = json!("sub_unread");
    deliver(&app, &as_sent).await;

    assert!(queued_refunds(&db).await.is_empty());
}
