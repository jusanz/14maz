use reqwest;
use serde_json;
use std::env;
use std::error::Error;
use std::fmt;
use tracing;

#[derive(Debug)]
pub struct EmbeddingError {
    message: String,
}

impl EmbeddingError {
    pub fn new(message: &str) -> EmbeddingError {
        EmbeddingError {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for EmbeddingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for EmbeddingError {}

pub async fn get_embedding(text: &str) -> Result<Vec<f64>, Box<dyn Error>> {
    let api_key =
        env::var("OPENAI_API_KEY").map_err(|_| EmbeddingError::new("OPENAI_API_KEY not set"))?;

    let request_body = serde_json::json!({
        "input": text,
        "model": "text-embedding-ada-002"
    });

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to send request: {}", e);
            EmbeddingError::new("Failed to send request")
        })?;

    let response_json: serde_json::Value = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse response as JSON: {}", e);
        EmbeddingError::new("Failed to parse response as JSON")
    })?;

    let embedding: Vec<f64> = response_json["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| {
            tracing::error!("Invalid response format");
            EmbeddingError::new("Invalid response format")
        })?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0))
        .collect();

    Ok(embedding)
}
