//! HTTP upload client.
//!
//! POSTs batches of TiltReading as JSON to the server's /api/v1/readings
//! endpoint using esp-idf-svc's HTTP client. Handles timeouts, status codes,
//! and optional API key authentication.
