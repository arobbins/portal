use sqlx::PgPool;

use super::*;
use crate::api::scope::ApiScope;
use crate::data_export::utc;
use crate::env::App;
use crate::oauth::testing::connect_test_app;
use crate::user::testing::active_user;

#[sqlx::test]
async fn an_account_without_connected_apps_exports_only_the_header(db: PgPool) {
    let user = active_user("alone@example.test", &db).await;
    let app = App::for_tests(db.clone());

    let dataset = connected_apps_export(&app.env(None), &user).await.unwrap();

    assert_eq!(dataset.file, "connected_apps.csv");
    assert_eq!(
        dataset.columns,
        [
            "name",
            "returns_to",
            "scopes",
            "connected_at",
            "last_used_at"
        ]
    );
    assert!(dataset.rows.is_empty());
}

#[sqlx::test]
async fn a_connected_app_exports_its_name_host_raw_scopes_and_dates(db: PgPool) {
    let user = active_user("connected@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let scopes = [ApiScope::ProfileRead, ApiScope::BillingRead];
    connect_test_app(&app, &user, &scopes).await;

    let dataset = connected_apps_export(&app.env(None), &user).await.unwrap();

    assert_eq!(dataset.rows.len(), 1);
    let row = &dataset.rows[0];
    assert_eq!(row[0], "Claude");
    assert_eq!(row[1], "claude.ai");
    assert_eq!(row[2], "profile:read billing:read");
    assert_eq!(row[3], utc(app.clock.now()));
    assert_eq!(row.len(), dataset.columns.len());
}
