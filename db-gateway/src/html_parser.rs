use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::{Error, PgPool};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

pub async fn create_table(pool: &PgPool) -> Result<(), Error> {
    // CREATE EXTENSION and TABLE commands are separated.
    match sqlx::query(
        r#"
        CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create extension: {}", e);
        }
    }

    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS texts (
          id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
          content JSONB,
          snapshot_id uuid,
          FOREIGN KEY (snapshot_id) REFERENCES snapshots(id),
          created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
          updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create table: {}", e);
        }
    }

    // Each part of function and trigger creation is a separate command.
    match sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION update_updated_at_column()
          RETURNS TRIGGER AS $$
          BEGIN
            NEW.updated_at = NOW();
            RETURN NEW;
          END;
          $$ language 'plpgsql';
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create function: {}", e);
        }
    }

    match sqlx::query(
        r#"
        DROP TRIGGER IF EXISTS update_updated_at ON texts;
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to drop trigger: {}", e);
        }
    }

    match sqlx::query(
        r#"
        CREATE TRIGGER update_updated_at
          BEFORE UPDATE ON texts
          FOR EACH ROW
          EXECUTE FUNCTION update_updated_at_column();
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create trigger: {}", e);
        }
    }

    Ok(())
}

pub async fn html_parser(pool: Arc<PgPool>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        info!("Parsing HTML")
    }
}
