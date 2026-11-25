//! Per-account resource caps, so one account cannot exhaust the shared database
//! or disk. Generous for real brewing; they exist to stop abuse and runaway
//! scripts, not to meter normal use.
//!
//! Attachment storage has its own cap next to the upload route
//! (`MAX_USER_BYTES`), and the number of hydrometers is already bounded by the
//! eight Tilt colors being unique per account.

pub const MAX_BREWS: u64 = 500;
pub const MAX_ALERT_TARGETS: u64 = 20;
pub const MAX_ALERT_RULES: u64 = 200;
pub const MAX_API_KEYS: u64 = 20;
pub const MAX_EVENTS_PER_BREW: u64 = 2_000;

/// Largest single `POST /readings` batch.
pub const MAX_READINGS_PER_BATCH: usize = 1_000;
/// Readings one account may ingest in a rolling 24 hours. Eight Tilts reporting
/// every 15 seconds is about 46,000, so this leaves room for legitimate use.
pub const MAX_READINGS_PER_DAY: u64 = 100_000;
/// ESP32 scanners one account may register (by heartbeat).
pub const MAX_SCANNERS: u64 = 32;
/// Live public share links per brew.
pub const MAX_REPORTS_PER_BREW: u64 = 20;
/// Log entries one report may include.
pub const MAX_REPORT_EVENTS: usize = 1_000;
