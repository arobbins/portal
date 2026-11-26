use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use sqlx::PgPool;
use topcoat::asset::AssetBundle;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::billing::testing::FakeStripe;
use crate::server::router::router;

async fn pricing_html(db: PgPool, path: &str) -> String {
    let stripe = FakeStripe::start().await;
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

#[sqlx::test]
async fn without_a_plan_on_both_periods_pricing_has_no_period_control(db: PgPool) {
    for path in ["/pricing", "/pricing?billing=yearly"] {
        let html = pricing_html(db.clone(), path).await;

        assert!(html.contains("Choose Pro"), "{path}");
        assert!(html.contains("$12 / month"), "{path}");
        assert!(!html.contains("Billing period"), "{path}");
        assert!(!html.contains("Yearly"), "{path}");
    }
}
