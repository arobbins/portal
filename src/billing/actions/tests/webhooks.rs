use serde_json::Value;
use sqlx::PgPool;

use super::*;
use crate::billing::actions::BillingStatus;
use crate::billing::actions::sync::Applied;
use crate::billing::testing::FakeStripe;

const T: i64 = 1_800_000_000;

#[sqlx::test]
async fn a_signed_event_changes_the_plan_once_and_a_replay_is_ignored(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("paid@example.test", CUSTOMER, &db).await;
    let created = subscription("customer.subscription.created", "evt_1", T, "active", false);

    assert_eq!(
        deliver(&app, &created).await,
        WebhookReply::Accepted(Applied::Changed)
    );
    assert_eq!(
        deliver(&app, &created).await,
        WebhookReply::Accepted(Applied::Repeated)
    );

    assert_eq!(
        plan_of(&app, &user).await,
        ("pro".to_string(), BillingStatus::Active)
    );
    assert_eq!(activity_of(&app, &user).await, ["Subscribed to Pro"]);
}

#[sqlx::test]
async fn unsigned_or_wrongly_signed_events_get_400_and_change_nothing(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("forged@example.test", CUSTOMER, &db).await;
    let body =
        subscription("customer.subscription.created", "evt_1", T, "active", false).to_string();
    let now = app.clock.now().unix_timestamp();
    let wrong = sign("whsec_someone_else", now, body.as_bytes());
    let env = app.env(None);

    for header in [None, Some(wrong.as_str()), Some("t=1,v1=00")] {
        let reply = super::super::receive_stripe_event(&env, header, body.as_bytes())
            .await
            .unwrap();
        assert_eq!(reply, WebhookReply::BadSignature);
        assert_eq!(reply.status().as_u16(), 400);
    }
    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::Free);
}

fn sub(kind: &str, n: i64, status: &str, cancelling: bool) -> Value {
    let kind = format!("customer.subscription.{kind}");
    subscription(&kind, &format!("evt_{n}"), T + n, status, cancelling)
}

#[sqlx::test]
async fn an_older_event_never_undoes_a_newer_one(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("ordered@example.test", CUSTOMER, &db).await;

    deliver(&app, &sub("created", 1, "active", false)).await;
    deliver(&app, &sub("updated", 3, "active", true)).await;
    let stale = deliver(&app, &sub("updated", 2, "active", false)).await;

    assert_eq!(stale, WebhookReply::Accepted(Applied::OutOfOrder));
    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::Cancelling);
}

#[sqlx::test]
async fn nothing_brings_an_ended_subscription_back(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("ended@example.test", CUSTOMER, &db).await;

    deliver(&app, &sub("created", 1, "active", false)).await;
    deliver(&app, &sub("deleted", 4, "canceled", false)).await;
    let late = deliver(&app, &sub("updated", 3, "active", false)).await;
    let payment = deliver(&app, &invoice("invoice.paid", "evt_5", T + 5)).await;

    assert_eq!(late, WebhookReply::Accepted(Applied::AlreadyEnded));
    assert_eq!(payment, WebhookReply::Accepted(Applied::AlreadyEnded));
    let free = ("free".to_string(), BillingStatus::Free);
    assert_eq!(plan_of(&app, &user).await, free);
}

#[sqlx::test]
async fn events_for_an_unknown_customer_get_200_and_are_ignored(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("someone@example.test", CUSTOMER, &db).await;
    let stranger = subscription_for(
        "cus_stranger",
        "sub_9",
        "customer.subscription.created",
        "evt_1",
        T,
        "active",
        false,
    );

    let reply = deliver(&app, &stranger).await;

    assert_eq!(reply, WebhookReply::Accepted(Applied::UnknownCustomer));
    assert_eq!(reply.status().as_u16(), 200);
    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::Free);
    let rows = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM subscriptions"#)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    let unknown_invoice = invoice("invoice.payment_failed", "evt_2", T);
    assert_eq!(
        deliver(&app, &unknown_invoice).await,
        WebhookReply::Accepted(Applied::UnknownSubscription)
    );
}

#[sqlx::test]
async fn a_failed_renewal_then_the_end_are_recorded_as_activity(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("renewal@example.test", CUSTOMER, &db).await;

    deliver(&app, &sub("created", 1, "active", false)).await;
    deliver(&app, &invoice("invoice.payment_failed", "evt_2", T + 2)).await;
    deliver(&app, &sub("updated", 3, "past_due", false)).await;
    let past_due = ("pro".to_string(), BillingStatus::PastDue);
    assert_eq!(plan_of(&app, &user).await, past_due);
    deliver(&app, &invoice("invoice.paid", "evt_4", T + 4)).await;
    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::Active);
    deliver(&app, &sub("updated", 5, "active", true)).await;
    deliver(&app, &sub("deleted", 6, "canceled", false)).await;

    let expected = [
        "Pro ended, back on Free",
        "Cancelled Pro (ends Mar 17, 2030)",
        "Payment failed for Pro",
        "Subscribed to Pro",
    ];
    assert_eq!(activity_of(&app, &user).await, expected);
}

#[sqlx::test]
async fn a_repeated_failure_is_logged_once_and_a_cancelled_plan_keeps_its_end(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("cancelled-late@example.test", CUSTOMER, &db).await;

    deliver(&app, &sub("created", 1, "active", false)).await;
    deliver(&app, &sub("updated", 2, "active", true)).await;
    deliver(&app, &invoice("invoice.payment_failed", "evt_3", T + 3)).await;
    deliver(&app, &invoice("invoice.payment_failed", "evt_4", T + 4)).await;
    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::PastDue);
    deliver(&app, &invoice("invoice.paid", "evt_5", T + 5)).await;

    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::Cancelling);
    let expected = [
        "Payment failed for Pro",
        "Cancelled Pro (ends Mar 17, 2030)",
        "Subscribed to Pro",
    ];
    assert_eq!(activity_of(&app, &user).await, expected);
}

#[sqlx::test]
async fn an_event_applies_the_subscription_as_stripe_has_it_now(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("current@example.test", CUSTOMER, &db).await;
    deliver(&app, &sub("created", 1, "active", false)).await;
    let now_cancelling = sub("updated", 9, "active", true);
    stripe.set_subscription("sub_1", now_cancelling["data"]["object"].clone());

    let stale = sub("updated", 2, "active", false);
    assert_eq!(
        deliver(&app, &stale).await,
        WebhookReply::Accepted(Applied::Changed)
    );

    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::Cancelling);
    assert_eq!(stripe.seen_at("GET", "/v1/subscriptions/sub_1").len(), 2);
}

#[sqlx::test]
async fn an_end_still_applies_when_the_price_is_no_longer_offered(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("retired@example.test", CUSTOMER, &db).await;
    deliver(&app, &sub("created", 1, "active", false)).await;
    let mut ended = sub("deleted", 2, "canceled", false);
    ended["data"]["object"]["items"]["data"][0]["price"]["id"] = "price_retired".into();

    assert_eq!(
        deliver(&app, &ended).await,
        WebhookReply::Accepted(Applied::Changed)
    );

    assert_eq!(plan_of(&app, &user).await.1, BillingStatus::Free);
    assert_eq!(activity_of(&app, &user).await[0], "Pro ended, back on Free");
}
