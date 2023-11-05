use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::{Error, PgPool};
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Payload {
    url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    url: Option<String>,
    html: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct Record {
    id: Uuid,
    url: String,
    content: Option<serde_json::Value>,
}

impl Record {
    pub fn url(&self) -> &str {
        &self.url
    }
}

pub async fn create_urls_table(pool: &PgPool) -> Result<(), Error> {
    // CREATE EXTENSION and TABLE commands are separated.
    sqlx::query(
        r#"
        CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS urls (
          id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
          url TEXT NOT NULL UNIQUE,
          content JSONB,
          snapshot_id uuid,
          FOREIGN KEY (snapshot_id) REFERENCES snapshots(id),
          created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
          updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Each part of function and trigger creation is a separate command.
    sqlx::query(
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
    .await?;

    sqlx::query(
        r#"
        DROP TRIGGER IF EXISTS update_urls_updated_at ON urls;
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER update_urls_updated_at
          BEFORE UPDATE ON urls
          FOR EACH ROW
          EXECUTE FUNCTION update_updated_at_column();
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_url(
    Extension(pool): Extension<Arc<PgPool>>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    let url = match payload.url {
        Some(url) => url,
        None => return (StatusCode::BAD_REQUEST, "Missing url"),
    };

    if !crate::validate_url(&url).unwrap() {
        return (StatusCode::BAD_REQUEST, "only absolute urls are allowed");
    }

    let pool = pool.as_ref();

    match sqlx::query(
        r#"
        INSERT INTO urls (url)
        VALUES ($1) ON CONFLICT (url) DO NOTHING
        "#,
    )
    .bind(url)
    .execute(pool)
    .await
    {
        Ok(result) => {
            if result.rows_affected() == 0 {
                (StatusCode::OK, "URL already exists")
            } else {
                (StatusCode::OK, "Url inserted")
            }
        }
        Err(e) => {
            error!("Failed to insert url: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to insert url")
        }
    }
}

pub async fn delete_url(
    Extension(pool): Extension<Arc<PgPool>>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    let url = match payload.url {
        Some(url) => url,
        None => return (StatusCode::BAD_REQUEST, "Missing url"),
    };
    let pool = pool.as_ref();

    match sqlx::query(
        r#"
        DELETE FROM urls
        WHERE url = $1
        "#,
    )
    .bind(url)
    .execute(pool)
    .await
    {
        Ok(_) => (StatusCode::OK, "Url deleted"),
        Err(e) => {
            error!("Failed to delete url: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete url")
        }
    }
}
