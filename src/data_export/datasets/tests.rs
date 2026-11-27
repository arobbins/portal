use std::collections::BTreeSet;

use sqlx::PgPool;

use super::*;
use crate::env::App;
use crate::user::testing::active_user;

#[sqlx::test]
async fn every_table_about_a_user_is_exported_or_explained(db: PgPool) {
    let user = active_user("everything@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let datasets = collect(&app.env(None), &user).await.unwrap();
    let covered: BTreeSet<&str> = datasets
        .iter()
        .flat_map(|dataset| dataset.tables.iter().copied())
        .chain(LEFT_OUT.iter().map(|left_out| left_out.table))
        .collect();

    let mut about_users: BTreeSet<String> = sqlx::query_scalar!(
        r#"SELECT DISTINCT table_name AS "table_name!" FROM information_schema.columns
           WHERE table_schema = 'public' AND column_name = 'user_id'"#
    )
    .fetch_all(&db)
    .await
    .unwrap()
    .into_iter()
    .collect();
    about_users.insert("users".to_string());

    let missing: Vec<&String> = about_users
        .iter()
        .filter(|table| !covered.contains(table.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "add these to the export, or to LEFT_OUT with a reason: {missing:?}"
    );
}

#[sqlx::test]
async fn every_dataset_has_a_header_and_rows_as_wide_as_it(db: PgPool) {
    let user = active_user("shape@example.test", &db).await;
    let app = App::for_tests(db.clone());

    for dataset in collect(&app.env(None), &user).await.unwrap() {
        assert!(!dataset.columns.is_empty(), "{}", dataset.file);
        for row in &dataset.rows {
            assert_eq!(row.len(), dataset.columns.len(), "{}", dataset.file);
        }
    }
}

#[sqlx::test]
async fn no_table_is_both_exported_and_left_out(db: PgPool) {
    let user = active_user("overlap@example.test", &db).await;
    let app = App::for_tests(db.clone());
    let exported: BTreeSet<&str> = collect(&app.env(None), &user)
        .await
        .unwrap()
        .iter()
        .flat_map(|dataset| dataset.tables.iter().copied())
        .collect();

    for left_out in LEFT_OUT {
        assert!(!exported.contains(left_out.table), "{}", left_out.table);
    }
}
