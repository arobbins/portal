use sqlx::PgPool;

use super::*;
use crate::auth::actions::{DeleteAccount, delete_account};
use crate::billing::jobs::CancelStripeSubscription;
use crate::billing::testing::{FakeStripe, unreachable_stripe};
use crate::jobs::{Handler, Job, Worker};
use crate::user::testing::TEST_PASSWORD;

fn confirmed() -> DeleteAccount {
    DeleteAccount {
        confirmation: "DELETE".to_string(),
        current_password: TEST_PASSWORD.to_string(),
    }
}

async fn queued_cancellations(db: &PgPool) -> Vec<(Option<String>, i32)> {
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
