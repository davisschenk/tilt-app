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
