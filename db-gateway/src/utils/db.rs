use sqlx::{Error, PgPool};
use tracing::error;

pub async fn create_extensions(pool: &PgPool) -> Result<(), Error> {
    match sqlx::query(r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp";"#)
        .execute(pool)
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create uuid-ossp extension: {}", e);
        }
    }

    match sqlx::query(r#"CREATE EXTENSION IF NOT EXISTS vector;"#)
        .execute(pool)
        .await
    {
        Ok(_) => (),
        Err(e) => {
            error!("Failed to create vector extension: {}", e);
        }
    }

    Ok(())
}

pub async fn create_functions(pool: &PgPool) -> Result<(), Error> {
    match sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION updated_at()
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
    Ok(())
}
