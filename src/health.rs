use axum::{Router, extract::State, http::StatusCode, routing::get};
use sqlx::{PgExecutor, PgPool};

pub fn router(db: PgPool) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(db)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(db): State<PgPool>) -> (StatusCode, &'static str) {
    match query_ping(&db).await {
        Ok(_) => (StatusCode::OK, "ready"),
        Err(e) => {
            tracing::error!("readiness check failed: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable")
        }
    }
}

async fn query_ping(db: impl PgExecutor<'_>) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT 1 AS "one!""#)
        .fetch_one(db)
        .await
}
