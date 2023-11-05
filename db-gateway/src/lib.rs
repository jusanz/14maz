pub mod html_parser;
pub mod snapshots;
pub mod urls;

use sqlx::{Error, PgPool, Row};
use tracing::debug;
use url::{ParseError, Url};

pub async fn print_table_schema(pool: &PgPool, table_name: &str) -> Result<(), Error> {
    let rows = sqlx::query("SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = $1")
        .bind(table_name)
        .fetch_all(pool)
        .await?;

    for row in rows {
        let column_name: String = row.try_get("column_name")?;
        let data_type: String = row.try_get("data_type")?;
        let is_nullable: String = row.try_get("is_nullable")?;
        debug!("{} {} {}", column_name, data_type, is_nullable);
    }

    Ok(())
}

pub fn validate_url(url: &str) -> Result<bool, ParseError> {
    let parsed_url = Url::parse(url);
    match parsed_url {
        Ok(url) => Ok(url.has_host()),
        Err(ParseError::RelativeUrlWithoutBase) => Ok(false),
        Err(e) => Err(e),
    }
}
