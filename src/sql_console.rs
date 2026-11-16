use serde_json::Value;
use sqlx::PgPool;

const HIDDEN: [&str; 4] = ["password", "hash", "secret", "public_key"];

pub async fn run(database_url: &str, query: Option<String>) {
    let Some(query) = query else {
        eprintln!("usage: webserver sql \"SELECT …\"");
        std::process::exit(2);
    };
    let db = PgPool::connect(database_url)
        .await
        .expect("could not connect to DATABASE_URL");

    match read_rows(&db, &query).await {
        Ok(rows) if rows.is_empty() => println!("(no rows)"),
        Ok(rows) => rows.iter().for_each(|row| println!("{row}")),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

async fn read_rows(db: &PgPool, query: &str) -> Result<Vec<Value>, sqlx::Error> {
    let mut tx = db.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    let wrapped = format!(
        "SELECT to_jsonb(row) FROM ({}) AS row",
        query.trim().trim_end_matches(';')
    );
    let rows: Vec<Value> = sqlx::query_scalar(&wrapped).fetch_all(&mut *tx).await?;
    tx.rollback().await?;

    Ok(rows.into_iter().map(hide_secrets).collect())
}

fn hide_secrets(mut row: Value) -> Value {
    if let Some(columns) = row.as_object_mut() {
        for (name, value) in columns.iter_mut() {
            if HIDDEN.iter().any(|hidden| name.contains(hidden)) && !value.is_null() {
                *value = Value::String("[hidden]".into());
            }
        }
    }
    row
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn secret_columns_are_hidden_but_nulls_stay_visible() {
        let row = json!({ "email": "a@example.test", "password": "$argon2", "token_hash": null });

        assert_eq!(
            hide_secrets(row),
            json!({ "email": "a@example.test", "password": "[hidden]", "token_hash": null })
        );
    }

    #[sqlx::test]
    async fn the_console_reads_rows_and_refuses_writes(db: PgPool) {
        let rows = read_rows(&db, "SELECT 1 AS one;").await.unwrap();
        assert_eq!(rows, vec![json!({ "one": 1 })]);

        let write = read_rows(
            &db,
            "WITH gone AS (DELETE FROM users RETURNING id) SELECT * FROM gone",
        )
        .await;
        assert!(write.is_err());
    }
}
