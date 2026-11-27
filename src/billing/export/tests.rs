use sqlx::PgPool;
use time::OffsetDateTime;

use super::*;
use crate::billing::queries::{query_insert_customer, query_insert_subscription};
use crate::billing::subscription::{SubscriptionState, SubscriptionStatus};
use crate::env::App;
use crate::user::queries::{query_get_user_by_id, query_update_user_time_zone};
use crate::user::testing::active_user;

const CUSTOMER: &str = "cus_export1";

fn early_on_christmas_utc() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_798_164_000).unwrap()
}

async fn paid_user(
    email: &str,
    status: SubscriptionStatus,
    ends_at: Option<OffsetDateTime>,
    db: &PgPool,
) -> User {
    let user = active_user(email, db).await;
    query_insert_customer(user.id, CUSTOMER, db).await.unwrap();
    let state = SubscriptionState {
        stripe_subscription_id: "sub_export1".to_string(),
        customer_id: CUSTOMER.to_string(),
        plan_key: "pro".to_string(),
        plan_name: "Pro".to_string(),
        price_id: "price_pro".to_string(),
        status,
        current_period_end: early_on_christmas_utc(),
        ends_at,
    };
    query_insert_subscription(user.id, &state, early_on_christmas_utc(), db)
        .await
        .unwrap();
    query_update_user_time_zone(user.id, "America/Chicago", db)
        .await
        .unwrap();
    query_get_user_by_id(user.id, db).await.unwrap().unwrap()
}

async fn billing_row(user: &User, db: &PgPool) -> Vec<String> {
    let app = App::for_tests(db.clone());
    let dataset = billing_export(&app.env(None), user).await.unwrap();
    assert_eq!(dataset.file, "billing.csv");
    assert_eq!(
        dataset.columns,
        [
            "plan",
            "status",
            "renews_on",
            "ends_on",
            "stripe_customer_id"
        ]
    );
    assert_eq!(dataset.rows.len(), 1);
    dataset.rows[0].clone()
}

#[sqlx::test]
async fn a_free_account_exports_the_free_plan_with_empty_cells(db: PgPool) {
    let user = active_user("free@example.test", &db).await;

    assert_eq!(billing_row(&user, &db).await, ["free", "free", "", "", ""]);
}

#[sqlx::test]
async fn a_paid_account_exports_its_plan_renewal_day_in_its_time_zone_and_customer(db: PgPool) {
    let user = paid_user("paid@example.test", SubscriptionStatus::Active, None, &db).await;

    assert_eq!(
        billing_row(&user, &db).await,
        ["pro", "active", "2026-12-24", "", CUSTOMER]
    );
}

#[sqlx::test]
async fn a_cancelling_account_exports_when_it_ends_instead_of_a_renewal(db: PgPool) {
    let ends = Some(early_on_christmas_utc());
    let user = paid_user(
        "leaving@example.test",
        SubscriptionStatus::Cancelling,
        ends,
        &db,
    )
    .await;

    assert_eq!(
        billing_row(&user, &db).await,
        ["pro", "cancelling", "", "2026-12-24", CUSTOMER]
    );
}

#[sqlx::test]
async fn a_past_due_account_exports_its_status(db: PgPool) {
    let user = paid_user("late@example.test", SubscriptionStatus::PastDue, None, &db).await;

    assert_eq!(
        billing_row(&user, &db).await,
        ["pro", "past_due", "2026-12-24", "", CUSTOMER]
    );
}
