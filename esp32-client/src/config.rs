//! Configuration loading and validation.
//!
//! Loads compile-time configuration from cfg.toml (via build.rs) and optional
//! runtime overrides from NVS. Validates all config values at startup.

/// Number of consecutive errors before logging an elevated error-level warning.
pub const CONSECUTIVE_ERROR_WARN_THRESHOLD: u32 = 10;

/// Firmware version, taken from Cargo.toml so it always matches the built image.
/// Compared against the server's OTA manifest.
pub const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Hex Ed25519 public key that OTA manifests must be signed with. Injected at build
/// time by `scripts/release.sh` (`OTA_PUBLIC_KEY`). When absent, OTA is refused.
pub const OTA_PUBLIC_KEY_HEX: Option<&str> = option_env!("OTA_PUBLIC_KEY");

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
    /// Human-friendly device name (set via web setup; informational).
    #[default("")]
    device_name: &'static str,
    /// Maximum duration of a single BLE scan chunk in seconds.
    /// Long scans are split into chunks of this size so the watchdog can be fed between them.
    #[default(30)]
    max_scan_chunk_secs: u32,
    /// Minimum number of BLE advertisement samples per Tilt color before a reading is
    /// reported. Colors with fewer inliers after IQR outlier rejection are skipped.
    /// Must be between 1 and 50.
    #[default(3)]
    min_samples_per_color: u32,
    /// Exponential weighted average smoothing factor as an integer percentage.
    /// e.g. 30 means α = 0.30. Range 1–100. Set to 100 to disable smoothing (pass-through).
    #[default(30)]
    ewa_alpha_percent: u32,
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

    if let Some(val) = nvs_get_string(&nvs, "wifi_ssid") {
        log::info!("NVS override: wifi_ssid = '{}'", val);
        cfg.wifi_ssid = Box::leak(val.into_boxed_str());
    }
    if let Some(val) = nvs_get_string(&nvs, "wifi_pass") {
        log::info!("NVS override: wifi_password = ***");
        cfg.wifi_password = Box::leak(val.into_boxed_str());
    }
    if let Some(val) = nvs_get_string(&nvs, "device_name") {
        log::info!("NVS override: device_name = '{}'", val);
        cfg.device_name = Box::leak(val.into_boxed_str());
    }
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
    if let Some(val) = nvs_get_u32(&nvs, "ewa_alpha") {
        log::info!("NVS override: ewa_alpha_percent = {}", val);
        cfg.ewa_alpha_percent = val;
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
    if cfg.ewa_alpha_percent < 1 || cfg.ewa_alpha_percent > 100 {
        anyhow::bail!(
            "ewa_alpha_percent must be between 1 and 100, got {}",
            cfg.ewa_alpha_percent
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
    log::info!("  ewa_alpha              = {} (α={:.2})", cfg.ewa_alpha_percent, cfg.ewa_alpha_percent as f64 / 100.0);
    log::info!("  device_name            = '{}'", cfg.device_name);
    log::info!("  firmware_version       = '{}'", FIRMWARE_VERSION);
}

/// Apply a provisioning patch on top of `cfg`. Absent fields are left untouched.
/// String values are leaked (the process reboots right after a successful save).
pub fn apply_patch(cfg: &mut Config, patch: &crate::protocol::ConfigPatch) {
    fn leak(s: &str) -> &'static str {
        Box::leak(s.to_string().into_boxed_str())
    }
    if let Some(v) = &patch.wifi_ssid {
        cfg.wifi_ssid = leak(v);
    }
    if let Some(v) = &patch.wifi_pass {
        cfg.wifi_password = leak(v);
    }
    if let Some(v) = &patch.server_url {
        cfg.server_url = leak(v);
    }
    if let Some(v) = &patch.api_key {
        cfg.api_key = leak(v);
    }
    if let Some(v) = &patch.device_name {
        cfg.device_name = leak(v);
    }
    if let Some(v) = patch.scan_interval_secs {
        cfg.scan_interval_secs = v;
    }
    if let Some(v) = patch.min_samples {
        cfg.min_samples_per_color = v;
    }
    if let Some(v) = patch.ewa_alpha {
        cfg.ewa_alpha_percent = v;
    }
    if let Some(v) = patch.ota_check_interval_cycles {
        cfg.ota_check_interval_cycles = v;
    }
}

/// Persist the fields present in `patch` to NVS. Call only after the merged
/// config has passed `validate_config`.
pub fn save_patch(
    nvs_partition: &esp_idf_svc::nvs::EspDefaultNvsPartition,
    patch: &crate::protocol::ConfigPatch,
) -> anyhow::Result<()> {
    let mut nvs = esp_idf_svc::nvs::EspNvs::new(nvs_partition.clone(), NVS_NAMESPACE, true)
        .map_err(|e| anyhow::anyhow!("failed to open NVS namespace: {:?}", e))?;

    let strings: [(&str, &Option<String>); 5] = [
        ("wifi_ssid", &patch.wifi_ssid),
        ("wifi_pass", &patch.wifi_pass),
        ("server_url", &patch.server_url),
        ("api_key", &patch.api_key),
        ("device_name", &patch.device_name),
    ];
    for (key, val) in strings {
        if let Some(v) = val {
            if v.is_empty() {
                // nvs_get_string treats empty as unset, so remove the key outright.
                let _ = nvs.remove(key);
            } else {
                nvs_set_string(&mut nvs, key, v)?;
            }
        }
    }
    let numbers: [(&str, Option<u32>); 4] = [
        ("scan_interval", patch.scan_interval_secs),
        ("min_samples", patch.min_samples),
        ("ewa_alpha", patch.ewa_alpha),
        ("ota_check_interval", patch.ota_check_interval_cycles),
    ];
    for (key, val) in numbers {
        if let Some(v) = val {
            nvs_set_u32(&mut nvs, key, v)?;
        }
    }
    Ok(())
}

/// Erase every provisioned value (revert to compile-time defaults).
pub fn clear_nvs(nvs_partition: &esp_idf_svc::nvs::EspDefaultNvsPartition) -> anyhow::Result<()> {
    let nvs = esp_idf_svc::nvs::EspNvs::new(nvs_partition.clone(), NVS_NAMESPACE, true)
        .map_err(|e| anyhow::anyhow!("failed to open NVS namespace: {:?}", e))?;
    for key in [
        "wifi_ssid", "wifi_pass", "device_name", "server_url", "api_key",
        "scan_interval", "min_samples", "ewa_alpha", "ota_check_interval",
    ] {
        let _ = nvs.remove(key);
    }
    Ok(())
}
