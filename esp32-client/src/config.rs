//! Configuration loading and validation.
//!
//! Loads compile-time configuration from cfg.toml (via build.rs) and optional
//! runtime overrides from NVS. Validates all config values at startup.

/// Number of consecutive errors before logging an elevated error-level warning.
pub const CONSECUTIVE_ERROR_WARN_THRESHOLD: u32 = 10;

#[toml_cfg::toml_config]
pub struct Config {
    /// WiFi network SSID to connect to.
    #[default("")]
    wifi_ssid: &'static str,
    /// WiFi network password. Leave empty for open networks.
    #[default("")]
    wifi_password: &'static str,
    /// Base URL of the backend server (e.g. "http://192.168.1.100:8000").
    #[default("http://192.168.1.100:8000")]
    server_url: &'static str,
    /// API key sent as `X-API-Key` header. Leave empty to disable authentication.
    #[default("")]
    api_key: &'static str,
    /// How long each BLE scan cycle runs, in seconds. Must be between 5 and 300.
    #[default(15)]
    scan_interval_secs: u32,
    /// Maximum capacity of the offline reading buffer (number of readings).
    /// Oldest readings are silently dropped when the buffer is full.
    #[default(50)]
    buffer_capacity: u32,
    /// Task Watchdog Timer timeout in seconds. Must be at least 2× scan_interval_secs.
    #[default(120)]
    watchdog_timeout_secs: u32,
    /// How often to log a health report, in scan cycles. 0 disables health reporting.
    #[default(60)]
    health_report_interval_cycles: u32,
    /// How often to check for OTA firmware updates, in scan cycles. 0 disables OTA checks.
    #[default(60)]
    ota_check_interval_cycles: u32,
    /// Firmware version string reported to the server for OTA version comparison.
    #[default("0.1.0")]
    firmware_version: &'static str,
    /// Maximum duration of a single BLE scan chunk in seconds.
    /// Long scans are split into chunks of this size so the watchdog can be fed between them.
    #[default(30)]
    max_scan_chunk_secs: u32,
    /// Minimum number of BLE advertisement samples per Tilt color before a reading is
    /// reported. Colors with fewer inliers after IQR outlier rejection are skipped.
    /// Must be between 1 and 50.
    #[default(3)]
    min_samples_per_color: u32,
}

const NVS_NAMESPACE: &str = "tilt_cfg";

pub fn nvs_get_string(
    nvs: &esp_idf_svc::nvs::EspNvs<esp_idf_svc::nvs::NvsDefault>,
    key: &str,
) -> Option<String> {
    let mut buf = [0u8; 256];
    match nvs.get_str(key, &mut buf) {
        Ok(Some(s)) => {
            let s = s.trim_end_matches('\0').to_string();
            if s.is_empty() { None } else { Some(s) }
        }
        _ => None,
    }
}

pub fn nvs_set_string(
    nvs: &mut esp_idf_svc::nvs::EspNvs<esp_idf_svc::nvs::NvsDefault>,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    nvs.set_str(key, value)
        .map_err(|e| anyhow::anyhow!("NVS set_str('{}') failed: {:?}", key, e))
}

pub fn nvs_get_u32(
    nvs: &esp_idf_svc::nvs::EspNvs<esp_idf_svc::nvs::NvsDefault>,
    key: &str,
) -> Option<u32> {
    nvs.get_u32(key).ok().flatten()
}

pub fn nvs_set_u32(
    nvs: &mut esp_idf_svc::nvs::EspNvs<esp_idf_svc::nvs::NvsDefault>,
    key: &str,
    value: u32,
) -> anyhow::Result<()> {
    nvs.set_u32(key, value)
        .map_err(|e| anyhow::anyhow!("NVS set_u32('{}') failed: {:?}", key, e))
}

pub fn apply_nvs_overrides(
    cfg: &mut Config,
    nvs_partition: &esp_idf_svc::nvs::EspDefaultNvsPartition,
) {
    let nvs = match esp_idf_svc::nvs::EspNvs::new(nvs_partition.clone(), NVS_NAMESPACE, true) {
        Ok(nvs) => nvs,
        Err(e) => {
            log::warn!("Failed to open NVS namespace '{}': {:?}, using compile-time defaults", NVS_NAMESPACE, e);
            return;
        }
    };

    if let Some(val) = nvs_get_string(&nvs, "server_url") {
        log::info!("NVS override: server_url = '{}'", val);
        cfg.server_url = Box::leak(val.into_boxed_str());
    }
    if let Some(val) = nvs_get_string(&nvs, "api_key") {
        log::info!("NVS override: api_key = ***");
        cfg.api_key = Box::leak(val.into_boxed_str());
    }
    if let Some(val) = nvs_get_u32(&nvs, "scan_interval") {
        log::info!("NVS override: scan_interval_secs = {}", val);
        cfg.scan_interval_secs = val;
    }
    if let Some(val) = nvs_get_u32(&nvs, "ota_check_interval") {
        log::info!("NVS override: ota_check_interval_cycles = {}", val);
        cfg.ota_check_interval_cycles = val;
    }
    if let Some(val) = nvs_get_u32(&nvs, "min_samples") {
        log::info!("NVS override: min_samples_per_color = {}", val);
        cfg.min_samples_per_color = val;
    }
}

pub fn validate_config(cfg: &Config) -> anyhow::Result<()> {
    if cfg.wifi_ssid.is_empty() {
        anyhow::bail!("wifi_ssid must not be empty");
    }
    if !cfg.server_url.starts_with("http://") && !cfg.server_url.starts_with("https://") {
        anyhow::bail!(
            "server_url must start with http:// or https://, got '{}'",
            cfg.server_url
        );
    }
    if cfg.scan_interval_secs < 5 || cfg.scan_interval_secs > 300 {
        anyhow::bail!(
            "scan_interval_secs must be between 5 and 300, got {}",
            cfg.scan_interval_secs
        );
    }
    if cfg.min_samples_per_color < 1 || cfg.min_samples_per_color > 50 {
        anyhow::bail!(
            "min_samples_per_color must be between 1 and 50, got {}",
            cfg.min_samples_per_color
        );
    }
    if cfg.buffer_capacity < 10 || cfg.buffer_capacity > 500 {
        anyhow::bail!(
            "buffer_capacity must be between 10 and 500, got {}",
            cfg.buffer_capacity
        );
    }
    // The longest the main task can block without feeding the watchdog is one
    // scan chunk (max_scan_chunk_secs). Require the timeout to be at least 3×
    // that so incidental delays (upload, flash write) don't cause false fires.
    let min_wdt = cfg.max_scan_chunk_secs * 3;
    if cfg.watchdog_timeout_secs < min_wdt {
        anyhow::bail!(
            "watchdog_timeout_secs ({}) must be >= 3 * max_scan_chunk_secs ({})",
            cfg.watchdog_timeout_secs,
            min_wdt
        );
    }
    Ok(())
}

fn mask(s: &str) -> &str {
    if s.is_empty() { "<empty>" } else { "***" }
}

pub fn log_config(cfg: &Config) {
    log::info!("Configuration:");
    log::info!("  wifi_ssid              = '{}'", cfg.wifi_ssid);
    log::info!("  wifi_password          = {}", mask(cfg.wifi_password));
    log::info!("  server_url             = '{}'", cfg.server_url);
    log::info!("  api_key                = {}", mask(cfg.api_key));
    log::info!("  scan_interval_secs     = {}", cfg.scan_interval_secs);
    log::info!("  max_scan_chunk_secs    = {}", cfg.max_scan_chunk_secs);
    log::info!("  buffer_capacity        = {}", cfg.buffer_capacity);
    log::info!("  watchdog_timeout       = {}s", cfg.watchdog_timeout_secs);
    log::info!("  health_interval        = {} cycles", cfg.health_report_interval_cycles);
    log::info!("  ota_check_interval     = {} cycles", cfg.ota_check_interval_cycles);
    log::info!("  min_samples_per_color  = {}", cfg.min_samples_per_color);
    log::info!("  firmware_version       = '{}'", cfg.firmware_version);
}
