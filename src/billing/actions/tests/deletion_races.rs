use sqlx::{PgPool, Postgres, Transaction};

use crate::jobs::Job;

use super::deletion::{confirmed, queued_cancellations};
use super::*;
use crate::admin::actions::{TargetUser, delete_user};
use crate::auth::actions::delete_account;
use crate::billing::jobs::CancelStripeSubscription;
use crate::billing::queries::query_lock_customer;
use crate::billing::testing::unreachable_stripe;
use crate::user::queries::query_get_user_by_id;
use crate::user::testing::active_user;

async fn webhook_holding_customer(db: &PgPool) -> Transaction<'static, Postgres> {
    let mut webhook = db.begin().await.unwrap();
    query_lock_customer(CUSTOMER, &mut *webhook)
        .await
        .unwrap()
        .unwrap();
    webhook
}

async fn record_subscription(mut webhook: Transaction<'_, Postgres>, user_id: i32, id: &str) {
    sqlx::query!(
        "INSERT INTO subscriptions (user_id, stripe_subscription_id, plan_key, plan_name, price_id, status, current_period_end, last_event_at)
        VALUES ($1, $2, 'pro', 'Pro', 'price_pro', 'active', now(), now())",
        user_id,
        id,
    )
    .execute(&mut *webhook)
    .await
    .unwrap();
    webhook.commit().await.unwrap();
}

async fn admin_caller(db: &PgPool) -> Caller {
    let boss = active_user("boss@example.test", db).await;
    sqlx::query!(
        "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
        boss.id
    )
    .execute(db)
    .await
    .unwrap();
    let boss = query_get_user_by_id(boss.id, db).await.unwrap().unwrap();
    Caller::browser(Arc::new(boss))
}

async fn expect_one_cancellation(id: &str, db: &PgPool) {
    assert_eq!(
        queued_cancellations(db).await,
        vec![(Some(id.to_string()), CancelStripeSubscription::MAX_ATTEMPTS)]
    );
}

#[sqlx::test]
async fn deletion_waits_for_a_subscription_being_recorded(db: PgPool) {
    let app = unreachable_stripe(&db);
    let user = customer("racing@example.test", CUSTOMER, &db).await;
    let webhook = webhook_holding_customer(&db).await;

    let deleting = {
        let (app, user) = (app.clone(), user.clone());
        tokio::spawn(async move {
            delete_account(&app.env(None), &Caller::browser(user), &confirmed(), None)
                .await
                .unwrap()
                .unwrap();
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!deleting.is_finished());
    record_subscription(webhook, user.id, "sub_racing").await;
    deleting.await.unwrap();

    expect_one_cancellation("sub_racing", &db).await;
}

#[sqlx::test]
async fn an_admin_deletion_waits_for_a_subscription_being_recorded(db: PgPool) {
    let app = unreachable_stripe(&db);
    let admin = admin_caller(&db).await;
    let user = customer("admin-racing@example.test", CUSTOMER, &db).await;
    let webhook = webhook_holding_customer(&db).await;

    let deleting = {
        let app = app.clone();
        let input = TargetUser {
            id: user.public_id.clone(),
        };
        tokio::spawn(async move {
            delete_user(&app.env(None), &admin, &input)
                .await
                .unwrap()
                .unwrap();
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    record_subscription(webhook, user.id, "sub_admin_racing").await;
    deleting.await.unwrap();

    expect_one_cancellation("sub_admin_racing", &db).await;
}
