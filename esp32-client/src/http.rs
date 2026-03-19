//! HTTP upload client.
//!
//! POSTs batches of TiltReading as JSON to the server's /api/v1/readings
//! endpoint using esp-idf-svc's HTTP client. Handles timeouts, status codes,
//! and optional API key authentication.

use anyhow::{Context, Result};
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};

use crate::tilt::TiltReading;

/// Maximum response body bytes read from the server in a single request.
/// Embedded systems have limited RAM; 1 KiB is sufficient for API error messages
/// and OTA JSON responses while preventing unbounded allocation.
const HTTP_RESPONSE_MAX_BYTES: usize = 1024;

/// HTTP client for uploading Tilt readings and querying the server API.
pub struct HttpUploader {
    server_url: &'static str,
    api_key: &'static str,
}

impl HttpUploader {
    /// Create a new uploader targeting `server_url`.
    ///
    /// If `api_key` is non-empty it is sent as the `X-API-Key` request header.
    pub fn new(server_url: &'static str, api_key: &'static str) -> Self {
        Self {
            server_url,
            api_key,
        }
    }

    /// POST a batch of readings as JSON to `/api/v1/readings`.
    ///
    /// Returns `Ok(())` on any 2xx response. On non-2xx responses the server's
    /// error body (up to `HTTP_RESPONSE_MAX_BYTES`) is included in the error.
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

        if (200..300).contains(&(status as u32)) {
            log::debug!("Upload successful: {} readings, status={}", readings.len(), status);
            Ok(())
        } else {
            let body = read_response_body(&mut conn);
            let body_str = core::str::from_utf8(&body).unwrap_or("<non-utf8>");
            Err(anyhow::anyhow!(
                "HTTP upload failed: status={}, body={}",
                status,
                body_str
            ))
        }
    }

    /// GET `url` and parse the response body as JSON.
    ///
    /// Returns an error on non-2xx status codes or JSON parse failures.
    /// Response bodies are capped at `HTTP_RESPONSE_MAX_BYTES`.
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
        let body = read_response_body(&mut conn);

        if status == 404 {
            return Err(anyhow::anyhow!("GET {} returned 404", url));
        }

        if !(200..300).contains(&(status as u32)) {
            let body_str = core::str::from_utf8(&body).unwrap_or("<non-utf8>");
            return Err(anyhow::anyhow!(
                "GET {} failed: status={}, body={}",
                url,
                status,
                body_str
            ));
        }

        serde_json::from_slice(&body).context("Failed to parse JSON response")
    }
}

/// Read the HTTP response body into a `Vec<u8>`, capped at `HTTP_RESPONSE_MAX_BYTES`.
///
/// Reads in 256-byte chunks to keep stack usage low. Stops early if the cap is
/// reached rather than allocating unbounded memory.
fn read_response_body(conn: &mut EspHttpConnection) -> Vec<u8> {
    let mut body = Vec::with_capacity(HTTP_RESPONSE_MAX_BYTES);
    let mut chunk = [0u8; 256];
    loop {
        match conn.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let space = HTTP_RESPONSE_MAX_BYTES.saturating_sub(body.len());
                if space == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..n.min(space)]);
            }
        }
    }
    body
}
