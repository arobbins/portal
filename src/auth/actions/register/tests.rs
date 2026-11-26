use sqlx::PgPool;

use super::*;
use crate::env::App;

async fn stored_zone(email: &str, db: &PgPool) -> Option<String> {
    sqlx::query_scalar!("SELECT time_zone FROM users WHERE email = $1", email)
        .fetch_one(db)
        .await
        .unwrap()
}

fn signing_up(email: &str, time_zone: Option<&str>) -> Register {
    Register {
        price: None,
        authorize: None,
        name: "Zoned".to_string(),
        email: email.to_string(),
        time_zone: time_zone.map(str::to_string),
    }
}

#[sqlx::test]
async fn the_browser_time_zone_is_kept_when_it_is_a_real_zone(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);

    register(
        &env,
        &signing_up("london@example.test", Some("Europe/London")),
    )
    .await
    .unwrap()
    .unwrap();
    register(
        &env,
        &signing_up("nowhere@example.test", Some("Mars/Olympus")),
    )
    .await
    .unwrap()
    .unwrap();
    register(&env, &signing_up("none@example.test", None))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        stored_zone("london@example.test", &db).await.as_deref(),
        Some("Europe/London")
    );
    assert_eq!(stored_zone("nowhere@example.test", &db).await, None);
    assert_eq!(stored_zone("none@example.test", &db).await, None);
}
