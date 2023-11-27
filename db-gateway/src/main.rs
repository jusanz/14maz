use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use std::env;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Postgres Connection Pool

    let postgres_user = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());
    let postgres_password =
        env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "postgres".to_string());
    let postgres_host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let postgres_db = env::var("POSTGRES_DB").unwrap_or_else(|_| "postgres".to_string());
    let postgres_url = format!(
        "postgres://{}:{}@{}:5432/{}",
        postgres_user, postgres_password, postgres_host, postgres_db
    );

    let pool = Arc::new(
        PgPool::connect(&postgres_url)
            .await
            .expect("Failed to create PgPool"),
    );

    // Create tables if they don't exist

    db_gateway::snapshots::create_snapshots_table(&pool.clone())
        .await
        .unwrap();
    db_gateway::urls::create_urls_table(&pool.clone())
        .await
        .unwrap();
    db_gateway::html_parser::create_table(&pool.clone())
        .await
        .unwrap();

    // Start Jobs

    tokio::spawn(db_gateway::html_parser::html_parser(pool.clone()));

    // Run Server

    let app = Router::new()
        .route("/api/urls", post(db_gateway::urls::insert_url))
        .route(
            "/api/url",
            get(db_gateway::snapshots::fetch_url_to_snapshot),
        )
        .route("/api/url/delete", post(db_gateway::urls::delete_url))
        .route(
            "/api/snapshots",
            post(db_gateway::snapshots::insert_snapshot),
        )
        .layer(Extension(pool.clone()));

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
