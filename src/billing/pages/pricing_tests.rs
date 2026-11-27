use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use topcoat::asset::AssetBundle;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::billing::testing::{FakeStripe, price, price_every};
use crate::server::router::router;

async fn pricing_html(db: PgPool, stripe: &FakeStripe, path: &str) -> String {
    let assets = AssetBundle::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/assets"))
        .expect("missing asset bundle: run `topcoat asset bundle` before `cargo test`");
    let app =
        router(stripe.app(&db), assets).layer(SessionManagerLayer::new(MemoryStore::default()));
    let request = Request::get(path)
        .header("host", "localhost")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn with_yearly_pro() -> FakeStripe {
    let stripe = FakeStripe::start().await;
    stripe.set_prices(json!([
        price("price_pro_monthly", 1200, "Pro", "pro"),
        price("price_business_monthly", 4900, "Business", "business"),
        price_every("price_pro_yearly", 10000, "Pro", "pro", "year"),
        price_every("price_max_yearly", 50000, "Max", "max", "year"),
    ]));
    stripe
}

fn shows(html: &str, price: &str) -> bool {
    html.contains(&format!("-->{price}<!--"))
}

fn switch_is_on(html: &str) -> bool {
    let Some(input) = html
        .split("<input")
        .find(|input| input.contains("role=\"switch\""))
    else {
        return false;
    };
    let mut quoted = false;
    let mut outside_quotes = String::new();
    for character in input.chars() {
        match character {
            '"' => quoted = !quoted,
            '>' if !quoted => break,
            _ if !quoted => outside_quotes.push(character),
            _ => {}
        }
    }
    outside_quotes
        .split_whitespace()
        .any(|attribute| attribute == "checked" || attribute == "checked=")
}

#[sqlx::test]
async fn without_a_plan_on_both_periods_pricing_has_no_switch(db: PgPool) {
    let stripe = FakeStripe::start().await;
    for path in ["/pricing", "/pricing?billing=monthly"] {
        let html = pricing_html(db.clone(), &stripe, path).await;

        assert!(html.contains("Choose Pro"), "{path}");
        assert!(html.contains("$12 / month"), "{path}");
        assert!(!html.contains("role=\"switch\""), "{path}");
        assert!(!html.contains("Billed yearly"), "{path}");
        assert!(!html.contains("Show monthly prices"), "{path}");
    }
}

#[sqlx::test]
async fn pricing_opens_on_yearly_per_month_for_any_value_but_monthly(db: PgPool) {
    let stripe = with_yearly_pro().await;
    for path in [
        "/pricing",
        "/pricing?billing=yearly",
        "/pricing?billing=weekly",
    ] {
        let html = pricing_html(db.clone(), &stripe, path).await;

        assert_eq!(html.matches("role=\"switch\"").count(), 1, "{path}");
        assert!(switch_is_on(&html), "{path}");
        assert!(shows(&html, "$8.33 / month"), "{path}");
        assert!(!shows(&html, "$12 / month"), "{path}");
        assert!(html.contains("value=\"price_pro_yearly\""), "{path}");
        assert!(!html.contains("value=\"price_pro_monthly\""), "{path}");
        assert!(
            html.contains("href=\"/pricing?billing=monthly\">Show monthly prices"),
            "{path}"
        );
        assert!(!html.contains("Save "), "{path}");
    }
}

#[sqlx::test]
async fn monthly_pricing_shows_monthly_prices_and_links_back_to_yearly(db: PgPool) {
    let stripe = with_yearly_pro().await;
    let html = pricing_html(db, &stripe, "/pricing?billing=monthly").await;

    assert!(!switch_is_on(&html));
    assert!(shows(&html, "$12 / month"));
    assert!(!shows(&html, "$8.33 / month"));
    assert!(html.contains("value=\"price_pro_monthly\""));
    assert!(!html.contains("value=\"price_pro_yearly\""));
    assert!(html.contains("href=\"/pricing\">Show yearly prices"));
}

#[sqlx::test]
async fn single_period_plans_show_their_one_price_without_a_switch(db: PgPool) {
    let stripe = with_yearly_pro().await;
    for path in ["/pricing", "/pricing?billing=monthly"] {
        let html = pricing_html(db.clone(), &stripe, path).await;

        assert!(html.contains("$49 / month"), "{path}");
        assert!(html.contains("$41.67 / month"), "{path}");
        assert!(html.contains("value=\"price_max_yearly\""), "{path}");
        assert!(html.contains(">Billed yearly</p>"), "{path}");
    }
}

#[sqlx::test]
async fn the_json_ld_keeps_every_price_at_its_real_amount(db: PgPool) {
    let stripe = with_yearly_pro().await;
    let html = pricing_html(db, &stripe, "/pricing").await;

    for amount in ["\"12.00\"", "\"100.00\"", "\"49.00\"", "\"500.00\""] {
        assert!(html.contains(amount), "{amount}");
    }
}
