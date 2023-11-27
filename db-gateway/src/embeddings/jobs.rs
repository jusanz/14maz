use crate::embeddings::api::get_embedding;
use sqlx::PgPool;
use std::sync::Arc;

pub async fn embed(pool: Arc<PgPool>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
    }
}
