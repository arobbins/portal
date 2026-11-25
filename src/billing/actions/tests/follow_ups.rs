use sqlx::PgPool;

use super::*;
use crate::admin::actions::{TargetUser, delete_user};
use crate::auth::actions::{Register, register};
use crate::auth::links::TokenPurpose;
use crate::billing::jobs::{CancelStripeSubscription, UpdateStripeCustomerEmail};
use crate::billing::testing::FakeStripe;
use crate::jobs::{Handler, Job, Worker};
use crate::routes::LOGIN_PATH;
use crate::state_machines::testing::{Browser, admin_caller, assets};

async fn queued(kind: &str, db: &PgPool) -> Vec<Value> {
    sqlx::query_scalar!("SELECT payload FROM jobs WHERE kind = $1", kind)
        .fetch_all(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn an_email_change_updates_the_stripe_customer(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("before@example.test", CUSTOMER, &db).await;
    let mut tx = db.begin().await.unwrap();
    crate::billing::follow_email_change(user.id, "after@example.test", &mut tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let worker = Worker::new(
        app.clone(),
        vec![Handler::of::<UpdateStripeCustomerEmail>()],
    );

    assert!(worker.run_next().await.unwrap());

    let updates = stripe.seen_at("POST", &format!("/v1/customers/{CUSTOMER}"));
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].field("email"), Some("after@example.test"));
    let free = active_user("never-paid@example.test", &db).await;
    let mut tx = db.begin().await.unwrap();
    crate::billing::follow_email_change(free.id, "x@example.test", &mut tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(queued("update_stripe_customer_email", &db).await.is_empty());
}

#[sqlx::test]
async fn an_admin_deletion_cancels_the_subscription_too(db: PgPool) {
    let stripe = FakeStripe::start().await;
    let app = stripe.app(&db);
    let user = customer("removed@example.test", CUSTOMER, &db).await;
    let event = subscription(
        "customer.subscription.created",
        "evt_1",
        1_800_000_000,
        "active",
        false,
    );
    deliver(&app, &event).await;
    let admin = admin_caller("deleting-admin@example.test", &db).await;
    let target = TargetUser {
        id: user.public_id.clone(),
    };

    delete_user(&app.env(None), &admin, &target)
        .await
        .unwrap()
        .unwrap();

    let jobs = queued(CancelStripeSubscription::KIND, &db).await;
    assert_eq!(
        jobs,
        [serde_json::json!({ "subscription": "sub_1", "refund": false })]
    );
}

fn signing_up(email: &str, price: Option<&str>) -> Register {
    Register {
        price: price.map(str::to_string),
        name: "Chooser".to_string(),
        email: email.to_string(),
        time_zone: None,
    }
}

async fn link_payload_of(email: &str, db: &PgPool) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT payload FROM user_tokens WHERE user_id = (SELECT id FROM users WHERE email = $1)",
        email,
    )
    .fetch_one(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn only_a_real_price_id_travels_with_the_registration_link(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);

    register(&env, &signing_up("chose@example.test", Some("price_pro")))
        .await
        .unwrap()
        .unwrap();
    register(&env, &signing_up("junk@example.test", Some("../customers")))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        link_payload_of("chose@example.test", &db).await.as_deref(),
        Some("price_pro")
    );
    assert_eq!(link_payload_of("junk@example.test", &db).await, None);
    let forced =
        sqlx::query!("UPDATE user_tokens SET payload = 'not a price' WHERE payload = 'price_pro'")
            .execute(&db)
            .await;
    assert!(forced.is_err(), "the payload CHECK accepts only price ids");
}

#[sqlx::test]
async fn opening_sign_in_with_a_price_remembers_only_a_real_one(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let mut chooser = Browser::new(&app, &assets, [198, 51, 130, 1]);
    let mut prober = Browser::new(&app, &assets, [198, 51, 130, 2]);

    chooser.get(&format!("{LOGIN_PATH}?price=price_pro")).await;
    prober
        .get(&format!("{LOGIN_PATH}?price=price_%2F..%2Fx"))
        .await;

    let chosen = chooser.session_value("chosen_price").await.unwrap();
    assert_eq!(chosen["price"], "price_pro");
    assert!(prober.session_value("chosen_price").await.is_none());
}

#[sqlx::test]
async fn opening_the_registration_link_reads_no_price_and_consumes_nothing(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let chooser = signing_up("opened@example.test", Some("price_pro"));
    register(&env, &chooser).await.unwrap().unwrap();
    let emails: String = queued("send_email", &db)
        .await
        .into_iter()
        .map(|job| job.to_string())
        .collect();
    let link_start = TokenPurpose::EmailVerification.path("");
    let start = emails.find(&link_start).unwrap();
    let path: String = emails[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '-')
        .collect();
    let mut browser = Browser::new(&app, &assets(), [198, 51, 130, 3]);

    let page = browser.get(&path).await;

    assert_eq!(page.status(), axum::http::StatusCode::OK);
    assert!(browser.session_value("chosen_price").await.is_none());
    let payload = link_payload_of("opened@example.test", &db).await;
    assert_eq!(payload.as_deref(), Some("price_pro"));
}
