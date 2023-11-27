use serde_json;
use sqlx::PgPool;
use tracing;

pub async fn create_embeddings_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS embeddings (
          id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
          embedding vector,
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
            tracing::error!("Failed to create table: {}", e);
        }
    }

    match sqlx::query(
        r#"
        DROP TRIGGER IF EXISTS updated_at ON embeddings;
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Failed to drop trigger: {}", e);
        }
    }

    match sqlx::query(
        r#"
        CREATE TRIGGER updated_at
          BEFORE UPDATE ON embeddings
          FOR EACH ROW
          EXECUTE FUNCTION updated_at();
        "#,
    )
    .execute(pool)
    .await
    {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Failed to create trigger: {}", e);
        }
    }

    Ok(())
}

pub async fn insert_embedding(
    pool: &PgPool,
    embedding: &Vec<f64>,
    content: serde_json::Value,
) -> Result<(), sqlx::Error> {
    match sqlx::query(r#"INSERT INTO embeddings (embedding, content) VALUES ($1, $2)"#)
        .bind(embedding)
        .bind(content)
        .execute(pool)
        .await
    {
        Ok(_result) => {
            tracing::info!("Embedding inserted");
        }
        Err(e) => {
            tracing::error!("Failed to insert embedding: {}", e);
        }
    }

    Ok(())
}
