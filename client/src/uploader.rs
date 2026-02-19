use std::fmt;

use reqwest::StatusCode;
use shared::{CreateReadingsBatch, TiltReading};

#[derive(Debug)]
pub enum UploadError {
    Network(reqwest::Error),
    ServerError(StatusCode, String),
}

impl fmt::Display for UploadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UploadError::Network(e) => write!(f, "network error: {e}"),
            UploadError::ServerError(status, body) => {
                write!(f, "server error {status}: {body}")
            }
        }
    }
}

impl std::error::Error for UploadError {}

impl From<reqwest::Error> for UploadError {
    fn from(e: reqwest::Error) -> Self {
        UploadError::Network(e)
    }
}

pub struct Uploader {
    client: reqwest::Client,
    readings_url: String,
    api_key: Option<String>,
}

impl Uploader {
    pub fn new(server_url: &str, api_key: Option<String>) -> Self {
        let base = server_url.trim_end_matches('/');
        Self {
            client: reqwest::Client::new(),
            readings_url: format!("{base}/api/v1/readings"),
            api_key,
        }
    }

    pub async fn upload_batch(
        &self,
        readings: &[TiltReading],
    ) -> Result<serde_json::Value, UploadError> {
        let batch = CreateReadingsBatch(readings.to_vec());

        tracing::debug!(
            url = %self.readings_url,
            count = readings.len(),
            "Uploading readings batch"
        );

        let mut request = self.client.post(&self.readings_url).json(&batch);

        if let Some(ref key) = self.api_key {
            request = request.header("X-API-Key", key);
        }

        let response = request.send().await?;

        let status = response.status();
        let body = response.text().await?;

        tracing::debug!(%status, %body, "Upload response");

        if status.is_success() {
            let value: serde_json::Value =
                serde_json::from_str(&body).unwrap_or(serde_json::json!({ "raw": body }));
            Ok(value)
        } else {
            Err(UploadError::ServerError(status, body))
        }
    }
}
