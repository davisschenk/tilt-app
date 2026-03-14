//! Configuration loading and validation.
//!
//! Loads compile-time configuration from cfg.toml (via build.rs) and optional
//! runtime overrides from NVS. Validates all config values at startup.

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_password: &'static str,
    #[default("http://192.168.1.100:8000")]
    server_url: &'static str,
    #[default("")]
    api_key: &'static str,
    #[default(15)]
    scan_interval_secs: u32,
    #[default(60)]
    upload_interval_secs: u32,
    #[default(50)]
    buffer_capacity: u32,
    #[default(120)]
    watchdog_timeout_secs: u32,
    #[default(60)]
    health_report_interval_cycles: u32,
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
    if let Some(val) = nvs_get_u32(&nvs, "upload_interval") {
        log::info!("NVS override: upload_interval_secs = {}", val);
        cfg.upload_interval_secs = val;
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
    if cfg.upload_interval_secs < cfg.scan_interval_secs {
        anyhow::bail!(
            "upload_interval_secs ({}) must be >= scan_interval_secs ({})",
            cfg.upload_interval_secs,
            cfg.scan_interval_secs
        );
    }
    if cfg.buffer_capacity < 10 || cfg.buffer_capacity > 500 {
        anyhow::bail!(
            "buffer_capacity must be between 10 and 500, got {}",
            cfg.buffer_capacity
        );
    }
    if cfg.watchdog_timeout_secs < cfg.scan_interval_secs * 2 {
        anyhow::bail!(
            "watchdog_timeout_secs ({}) must be >= 2 * scan_interval_secs ({})",
            cfg.watchdog_timeout_secs,
            cfg.scan_interval_secs * 2
        );
    }
    Ok(())
}

fn mask(s: &str) -> &str {
    if s.is_empty() { "<empty>" } else { "***" }
}

pub fn log_config(cfg: &Config) {
    log::info!("Configuration:");
    log::info!("  wifi_ssid          = '{}'", cfg.wifi_ssid);
    log::info!("  wifi_password      = {}", mask(cfg.wifi_password));
    log::info!("  server_url         = '{}'", cfg.server_url);
    log::info!("  api_key            = {}", mask(cfg.api_key));
    log::info!("  scan_interval_secs = {}", cfg.scan_interval_secs);
    log::info!("  upload_interval    = {}s", cfg.upload_interval_secs);
    log::info!("  buffer_capacity    = {}", cfg.buffer_capacity);
    log::info!("  watchdog_timeout   = {}s", cfg.watchdog_timeout_secs);
    log::info!("  health_interval    = {} cycles", cfg.health_report_interval_cycles);
}
