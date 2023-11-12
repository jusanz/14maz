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

#[derive(Deserialize)]
pub struct Payload {
    url: Option<String>,
    html: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    url: Option<String>,
    html: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Snapshot {
    id: Uuid,
    url: String,
    content: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ResponseBody {
    data: Option<String>,
}

pub async fn create_snapshots_table(pool: &PgPool) -> Result<(), Error> {
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
        CREATE TABLE IF NOT EXISTS snapshots (
          id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
          url TEXT NOT NULL,
          content JSONB,
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
        DROP TRIGGER IF EXISTS update_snapshots_updated_at ON snapshots;
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
        CREATE TRIGGER update_snapshots_updated_at
          BEFORE UPDATE ON snapshots
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

pub async fn insert_snapshot(
    Extension(pool): Extension<Arc<PgPool>>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    let url = match payload.url {
        Some(url) => url,
        None => return (StatusCode::BAD_REQUEST, "Missing url"),
    };
    let html = match payload.html {
        Some(html) => html,
        None => return (StatusCode::BAD_REQUEST, "Missing html"),
    };

    if !crate::validate_url(&url).unwrap() {
        return (StatusCode::BAD_REQUEST, "only absolute urls are allowed");
    }

    let pool = pool.as_ref();

    match crate::urls::crawl_url(pool, &url).await {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to crawl url: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to crawl url");
        }
    };

    let last_snapshot = match fetch_last_snapshot(&url, pool).await {
        Ok(last_snapshot) => last_snapshot,
        Err(e) => {
            error!("Failed to fetch last snapshot: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch last snapshot",
            );
        }
    };

    match &last_snapshot {
        Some(last_snapshot) => {
            debug!("Last snapshot: {:?}", last_snapshot.html);
        }
        None => {
            debug!("No last snapshot");
        }
    }

    if let Some(last_snapshot) = last_snapshot {
        if last_snapshot.html == Some(html.to_string()) {
            info!("No need to insert the same content twice.");
            return (StatusCode::OK, "No need to insert the same content twice.");
        }
    }

    let content = Content {
        url: Some(url.to_string()),
        html: Some(html.to_string()),
    };

    let content_value = serde_json::to_value(&content).unwrap();

    let record = match sqlx::query_as::<_, Snapshot>(
        r#"
        INSERT INTO snapshots (url, content)
        VALUES ($1, $2)
        RETURNING *
        "#,
    )
    .bind(&url)
    .bind(content_value)
    .fetch_optional(pool)
    .await
    {
        Ok(record) => record,
        Err(e) => {
            error!("Failed to insert snapshot: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to insert snapshot",
            );
        }
    };

    match record {
        Some(record) => {
            add_relation_to_url(&url, &record.id, pool).await.unwrap();
            (StatusCode::OK, "Snapshot inserted")
        }
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to insert snapshot",
        ),
    }
}

async fn fetch_last_snapshot(url: &str, pool: &PgPool) -> Result<Option<Content>, Error> {
    debug!("Fetching last snapshot for {}", url);

    let sql = r#"
        SELECT * FROM snapshots
        WHERE url = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#;

    let result = match sqlx::query_as::<_, Snapshot>(sql)
        .bind(url)
        .fetch_optional(pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            return Err(e);
        }
    };

    let content = match result {
        Some(result) => result.content,
        None => return Ok(None),
    };

    if let Some(content) = content {
        let content: Content = serde_json::from_value(content).unwrap();
        Ok(Some(content))
    } else {
        Ok(None)
    }
}

async fn add_relation_to_url(url: &str, snapshot_id: &Uuid, pool: &PgPool) -> Result<(), Error> {
    //let snapshot_uuid = Uuid::parse_str(snapshot_id).map_err(|e| sqlx::Error::TypeNotFound {
    //    type_name: snapshot_id.to_string(),
    //})?;

    sqlx::query(
        r#"
        UPDATE urls
        SET snapshot_id = $1
        WHERE url = $2
        "#,
    )
    .bind(snapshot_id)
    .bind(url)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn fetch_url_to_snapshot(Extension(pool): Extension<Arc<PgPool>>) -> impl IntoResponse {
    let pool = pool.as_ref();

    let sql = r#"
        SELECT * FROM urls
        WHERE snapshot_id IS NULL
        ORDER BY updated_at ASC
        LIMIT 1
        "#;

    let result = match sqlx::query_as::<_, crate::urls::Record>(sql)
        .fetch_optional(pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to fetch url: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResponseBody { data: None }),
            );
        }
    };

    if let Some(record) = result {
        let url = record.url();
        return (
            StatusCode::OK,
            Json(ResponseBody {
                data: Some(url.to_string()),
            }),
        );
    }

    let sql = r#"
        SELECT * FROM urls
        ORDER BY (content->>'crawled_at') NULLS FIRST
        LIMIT 1
        "#;

    let result = match sqlx::query_as::<_, crate::urls::Record>(sql)
        .fetch_optional(pool)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to fetch url: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ResponseBody { data: None }),
            );
        }
    };

    match result {
        Some(record) => {
            let url = record.url();
            (
                StatusCode::OK,
                Json(ResponseBody {
                    data: Some(url.to_string()),
                }),
            )
        }
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ResponseBody { data: None }),
        ),
    }
}
