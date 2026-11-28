use sqlx::PgConnection;

use super::RateLimits;
use super::limit::Limit;
use super::queries::{
    query_count_rate_limit_hits, query_insert_rate_limit_hit, query_lock_rate_limit_key,
    query_take_rate_limit_slot,
};

impl RateLimits {
    pub(super) async fn allow(&self, limit: Limit, key: &str) -> Result<bool, sqlx::Error> {
        let mut tx = self.db.begin().await?;
        let taken = self.take_slot(limit, key, &mut tx).await?;
        tx.commit().await?;
        Ok(taken)
    }

    pub(super) async fn take_slot(
        &self,
        limit: Limit,
        key: &str,
        tx: &mut PgConnection,
    ) -> Result<bool, sqlx::Error> {
        query_lock_rate_limit_key(limit.bucket(), key, &mut *tx).await?;
        query_take_rate_limit_slot(
            limit.bucket(),
            key,
            limit.window_seconds(),
            limit.max(self.mode),
            &mut *tx,
        )
        .await
    }

    pub(super) async fn blocked(&self, limit: Limit, key: &str) -> Result<bool, sqlx::Error> {
        let hits =
            query_count_rate_limit_hits(limit.bucket(), key, limit.window_seconds(), &self.db)
                .await?;
        Ok(hits >= limit.max(self.mode))
    }

    pub(super) async fn record(&self, limit: Limit, key: &str) -> Result<(), sqlx::Error> {
        query_insert_rate_limit_hit(limit.bucket(), key, &self.db).await
    }
}
