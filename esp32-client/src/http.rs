//! HTTP upload client.
//!
//! POSTs batches of TiltReading as JSON to the server's /api/v1/readings
//! endpoint using esp-idf-svc's HTTP client. Handles timeouts, status codes,
//! and optional API key authentication.

use anyhow::{Context, Result};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::io::Read;

use crate::tilt::TiltReading;

pub struct HttpUploader {
    server_url: &'static str,
    api_key: &'static str,
}

impl HttpUploader {
    pub fn new(server_url: &'static str, api_key: &'static str) -> Self {
        Self {
            server_url,
            api_key,
        }
    }

    pub fn upload_batch(&self, readings: &[TiltReading]) -> Result<()> {
        let url = format!("{}/api/v1/readings", self.server_url.trim_end_matches('/'));
        let payload =
            serde_json::to_vec(readings).context("Failed to serialize readings to JSON")?;

        let config = HttpConfig {
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            timeout: Some(std::time::Duration::from_secs(15)),
            ..Default::default()
        };

        let mut conn =
            EspHttpConnection::new(&config).context("Failed to create HTTP connection")?;

        let content_len_str = payload.len().to_string();
        let mut headers: Vec<(&str, &str)> = vec![
            ("Content-Type", "application/json"),
            ("Content-Length", content_len_str.as_str()),
        ];
        if !self.api_key.is_empty() {
            headers.push(("X-API-Key", self.api_key));
        }

        conn.initiate_request(
            esp_idf_svc::http::Method::Post,
            &url,
            &headers,
        )
        .context("Failed to initiate HTTP request")?;

        conn.write_all(&payload)
            .context("Failed to write request body")?;

        conn.initiate_response()
            .context("Failed to initiate HTTP response")?;

        let status = conn.status();

        // Read response body for error messages
        let mut body = [0u8; 256];
        let bytes_read = conn.read(&mut body).unwrap_or(0);

        if (200..300).contains(&(status as u32)) {
            log::debug!("Upload successful: {} readings, status={}", readings.len(), status);
            Ok(())
        } else {
            let body_str = core::str::from_utf8(&body[..bytes_read]).unwrap_or("<non-utf8>");
            Err(anyhow::anyhow!(
                "HTTP upload failed: status={}, body={}",
                status,
                body_str
            ))
        }
    }

    pub fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let config = HttpConfig {
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            timeout: Some(std::time::Duration::from_secs(10)),
            ..Default::default()
        };

        let mut conn =
            EspHttpConnection::new(&config).context("Failed to create HTTP connection")?;

        let mut headers: Vec<(&str, &str)> = vec![("Accept", "application/json")];
        if !self.api_key.is_empty() {
            headers.push(("X-API-Key", self.api_key));
        }

        conn.initiate_request(esp_idf_svc::http::Method::Get, url, &headers)
            .context("Failed to initiate GET request")?;

        conn.initiate_response()
            .context("Failed to initiate GET response")?;

        let status = conn.status();

        let mut body = [0u8; 512];
        let bytes_read = conn.read(&mut body).unwrap_or(0);

        if status == 404 {
            return Err(anyhow::anyhow!("GET {} returned 404", url));
        }

        if !(200..300).contains(&(status as u32)) {
            let body_str = core::str::from_utf8(&body[..bytes_read]).unwrap_or("<non-utf8>");
            return Err(anyhow::anyhow!(
                "GET {} failed: status={}, body={}",
                url,
                status,
                body_str
            ));
        }

        serde_json::from_slice(&body[..bytes_read]).context("Failed to parse JSON response")
    }
}
