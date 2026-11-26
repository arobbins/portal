use sqlx::PgPool;

use super::*;
use crate::billing::actions::sync::Applied;
use crate::billing::jobs::CancelStripeSubscription;
use crate::billing::testing::FakeStripe;
use crate::jobs::{Handler, Job, Worker};

const T: i64 = 1_800_000_000;

fn created(sub: &str, event: &str, at: i64) -> Value {
    subscription_for(
        CUSTOMER,
        sub,
        "customer.subscription.created",
        event,
        at,
        "active",
        false,
    )
}

async fn live_rows(db: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM subscriptions WHERE status <> 'ended'"#)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_second_subscription_is_cancelled_refunded_and_logged(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("twice@example.test", CUSTOMER, &db).await;
    deliver(&app, &created("sub_1", "evt_1", T)).await;

    let second = deliver(&app, &created("sub_2", "evt_2", T + 1)).await;
    let worker = Worker::new(app.clone(), vec![Handler::of::<CancelStripeSubscription>()]);
    assert!(worker.run_next().await.unwrap());

    assert_eq!(second, WebhookReply::Accepted(Applied::Duplicate));
    assert_eq!(stripe.seen_at("DELETE", "/v1/subscriptions/sub_2").len(), 1);
    let refunds = stripe.seen_at("POST", "/v1/refunds");
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0].field("payment_intent"), Some("pi_fake"));
    assert_eq!(refunds[0].idempotency_key.as_deref(), Some("refund-sub_2"));
    assert!(
        stripe
            .seen_at("DELETE", "/v1/subscriptions/sub_1")
            .is_empty()
    );
    assert_eq!(
        activity_of(&app, &user).await,
        [
            "Second Pro subscription cancelled and refunded",
            "Subscribed to Pro"
        ]
    );
    assert_eq!(live_rows(&db).await, 1);
}

#[sqlx::test]
async fn two_subscriptions_arriving_at_once_leave_one_live(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    customer("racing@example.test", CUSTOMER, &db).await;
    let first = created("sub_1", "evt_1", T);
    let second = created("sub_2", "evt_2", T);

    let (a, b) = tokio::join!(deliver(&app, &first), deliver(&app, &second));

    let mut outcomes = vec![a, b];
    outcomes.sort_by_key(|reply| format!("{reply:?}"));
    assert_eq!(
        outcomes,
        [
            WebhookReply::Accepted(Applied::Changed),
            WebhookReply::Accepted(Applied::Duplicate)
        ]
    );
    assert_eq!(live_rows(&db).await, 1);
}

#[sqlx::test]
async fn the_database_refuses_a_second_live_subscription(db: PgPool) {
    let user = active_user("guarded@example.test", &db).await;
    let insert = async |sub: &str| {
        sqlx::query!(
            "INSERT INTO subscriptions (user_id, stripe_subscription_id, plan_key, plan_name, price_id, status, current_period_end, last_event_at) VALUES ($1, $2, 'pro', 'Pro', 'price_pro', 'active', now(), now())",
            user.id,
            sub,
        )
        .execute(&db)
        .await
    };

    assert!(insert("sub_1").await.is_ok());
    assert!(insert("sub_2").await.is_err());
}

async fn cancel_jobs(db: &PgPool) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM jobs WHERE kind = 'cancel_stripe_subscription'"#
    )
    .fetch_one(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn later_events_for_the_same_duplicate_refund_it_once(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("again@example.test", CUSTOMER, &db).await;
    deliver(&app, &created("sub_1", "evt_1", T)).await;
    deliver(&app, &created("sub_2", "evt_2", T + 1)).await;
    let updated = subscription_for(
        CUSTOMER,
        "sub_2",
        "customer.subscription.updated",
        "evt_3",
        T + 2,
        "active",
        false,
    );

    let again = deliver(&app, &updated).await;

    assert_eq!(again, WebhookReply::Accepted(Applied::Duplicate));
    assert_eq!(cancel_jobs(&db).await, 1);
    assert_eq!(
        activity_of(&app, &user).await,
        [
            "Second Pro subscription cancelled and refunded",
            "Subscribed to Pro"
        ]
    );
}

fn refund_of(sub: &str) -> CancelStripeSubscription {
    CancelStripeSubscription {
        subscription: sub.to_string(),
        refund: true,
    }
}

#[sqlx::test]
async fn a_refund_stripe_already_made_counts_as_done(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    stripe.reject_refunds_with("charge_already_refunded");

    let outcome = refund_of("sub_2").run(&app).await;

    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!(stripe.seen_at("POST", "/v1/refunds").len(), 1);
}

#[sqlx::test]
async fn another_refund_rejection_is_retried(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    stripe.reject_refunds_with("charge_disputed");

    assert!(refund_of("sub_2").run(&app).await.is_err());
}

#[sqlx::test]
async fn nothing_paid_yet_cancels_without_a_refund(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    for invoices in [
        json!([]),
        json!([{ "id": "in_free", "amount_paid": 0, "payment_intent": null }]),
    ] {
        stripe.set_invoices(invoices);

        let outcome = refund_of("sub_2").run(&app).await;

        assert!(outcome.is_ok(), "{outcome:?}");
    }
    assert_eq!(stripe.seen_at("DELETE", "/v1/subscriptions/sub_2").len(), 2);
    assert!(stripe.seen_at("POST", "/v1/refunds").is_empty());
}
