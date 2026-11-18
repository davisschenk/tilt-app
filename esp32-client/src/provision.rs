//! Serial provisioning: lets the web UI read and write device configuration
//! over the same USB serial port used for flashing. See `protocol.rs` for the
//! wire format.

use std::collections::VecDeque;
use std::time::Duration;

use anyhow::{bail, Result};
use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use serde_json::{json, Value};

use crate::config::{self, Config};
use crate::protocol::{self, ConfigPatch, LineAssembler, Request};
use crate::wifi::WifiManager;

const UART_PORT: esp_idf_svc::sys::uart_port_t = 0;

/// Install the UART driver so we can do blocking reads on the console port.
/// Log output (stdout) is unaffected.
pub fn init_uart() -> Result<()> {
    // Safety: plain FFI call with a null queue pointer, which the driver allows
    // when no event queue is requested (queue_size = 0).
    let ret = unsafe {
        esp_idf_svc::sys::uart_driver_install(UART_PORT, 1024, 0, 0, core::ptr::null_mut(), 0)
    };
    if ret != esp_idf_svc::sys::ESP_OK && ret != esp_idf_svc::sys::ESP_ERR_INVALID_STATE {
        bail!("uart_driver_install failed: {}", ret);
    }
    Ok(())
}

/// Reads complete lines from the console UART.
struct SerialLines {
    asm: LineAssembler,
    ready: VecDeque<String>,
}

impl SerialLines {
    fn new() -> Self {
        Self { asm: LineAssembler::default(), ready: VecDeque::new() }
    }

    /// Wait up to ~1s for a line; `None` on timeout.
    fn next_line(&mut self) -> Option<String> {
        if let Some(l) = self.ready.pop_front() {
            return Some(l);
        }
        let mut buf = [0u8; 128];
        // Safety: buf is valid for buf.len() bytes for the duration of the call.
        let n = unsafe {
            esp_idf_svc::sys::uart_read_bytes(
                UART_PORT,
                buf.as_mut_ptr().cast(),
                buf.len() as u32,
                TickType::new_millis(1000).ticks(),
            )
        };
        if n > 0 {
            self.ready.extend(self.asm.feed(&buf[..n as usize]));
        }
        self.ready.pop_front()
    }
}

fn read_mac() -> String {
    let mut mac = [0u8; 6];
    // Safety: mac is 6 bytes, as esp_read_mac requires for the STA MAC.
    let ret = unsafe {
        esp_idf_svc::sys::esp_read_mac(mac.as_mut_ptr(), esp_idf_svc::sys::esp_mac_type_t_ESP_MAC_WIFI_STA)
    };
    if ret != esp_idf_svc::sys::ESP_OK {
        return String::new();
    }
    mac.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(":")
}

fn emit(cmd: &str, body: Value) {
    println!("{}", protocol::format_reply(cmd, body));
}

fn fail(cmd: &str, msg: impl std::fmt::Display) {
    emit(cmd, json!({ "ok": false, "error": msg.to_string() }));
}

/// Handles provisioning requests against the boot-time configuration.
struct Provisioner {
    /// Effective configuration at boot (compile-time defaults + NVS).
    cfg: Config,
    nvs: EspDefaultNvsPartition,
    provisioned: bool,
}

impl Provisioner {
    fn handle_line(&self, line: &str, scan: &mut dyn FnMut() -> Result<Vec<(String, i8, String)>>) {
        let Some(parsed) = protocol::parse_line(line) else { return };
        match parsed {
            Err(e) => fail("error", e),
            Ok(Request::Info) => emit(
                "info",
                json!({
                    "ok": true,
                    "version": config::FIRMWARE_VERSION,
                    "chip": "esp32",
                    "mac": read_mac(),
                    "provisioned": self.provisioned,
                    "deviceName": self.cfg.device_name,
                }),
            ),
            Ok(Request::GetConfig) => emit("get_config", self.config_json()),
            Ok(Request::WifiScan) => match scan() {
                Ok(aps) => {
                    let list: Vec<Value> = aps
                        .into_iter()
                        .map(|(ssid, rssi, auth)| json!({ "ssid": ssid, "rssi": rssi, "auth": auth }))
                        .collect();
                    emit("wifi_scan", json!({ "ok": true, "networks": list }));
                }
                Err(e) => fail("wifi_scan", format!("{e:#}")),
            },
            Ok(Request::SetConfig { config: patch }) => self.set_config(&patch),
            Ok(Request::FactoryReset) => match config::clear_nvs(&self.nvs) {
                Ok(()) => {
                    emit("factory_reset", json!({ "ok": true }));
                    reboot_soon();
                }
                Err(e) => fail("factory_reset", format!("{e:#}")),
            },
        }
    }

    fn config_json(&self) -> Value {
        let c = &self.cfg;
        json!({
            "ok": true,
            "provisioned": self.provisioned,
            "config": {
                "wifiSsid": c.wifi_ssid,
                "wifiPass": protocol::mask_secret(c.wifi_password),
                "serverUrl": c.server_url,
                "apiKey": protocol::mask_secret(c.api_key),
                "deviceName": c.device_name,
                "scanIntervalSecs": c.scan_interval_secs,
                "minSamples": c.min_samples_per_color,
                "ewaAlpha": c.ewa_alpha_percent,
                "otaCheckIntervalCycles": c.ota_check_interval_cycles,
            }
        })
    }

    fn set_config(&self, patch: &ConfigPatch) {
        // Validate the *merged* result so partial updates can't produce a bad config.
        let mut merged = Config { ..self.cfg };
        config::apply_patch(&mut merged, patch);
        if let Err(e) = config::validate_config(&merged) {
            fail("set_config", format!("{e:#}"));
            return;
        }
        match config::save_patch(&self.nvs, patch) {
            Ok(()) => {
                emit("set_config", json!({ "ok": true }));
                reboot_soon();
            }
            Err(e) => fail("set_config", format!("{e:#}")),
        }
    }
}

fn reboot_soon() {
    // Give the reply time to drain out of the UART before resetting.
    std::thread::sleep(Duration::from_millis(500));
    esp_idf_svc::hal::reset::restart();
}

/// Normal mode: serve provisioning requests from a background thread while the
/// main loop does its work. WiFi scanning is unavailable (the radio is in use).
pub fn spawn_listener(cfg: Config, nvs: EspDefaultNvsPartition) -> Result<()> {
    std::thread::Builder::new()
        .name("provision".into())
        .stack_size(8192)
        .spawn(move || {
            let p = Provisioner { cfg, nvs, provisioned: true };
            let mut lines = SerialLines::new();
            let mut no_scan = || -> Result<Vec<(String, i8, String)>> {
                bail!("wifi scan is only available before the device is configured")
            };
            loop {
                if let Some(line) = lines.next_line() {
                    p.handle_line(&line, &mut no_scan);
                }
            }
        })?;
    Ok(())
}

/// Unprovisioned mode: block forever serving requests, with WiFi scanning
/// available. The device reboots once a valid configuration is saved.
pub fn run_unprovisioned(
    cfg: Config,
    nvs: EspDefaultNvsPartition,
    modem: impl WifiModemPeripheral + 'static,
    sys_loop: esp_idf_svc::eventloop::EspSystemEventLoop,
) -> Result<()> {
    let mut wifi = WifiManager::new(modem, sys_loop, nvs.clone(), "", "")?;
    let p = Provisioner { cfg, nvs, provisioned: false };
    let mut lines = SerialLines::new();
    log::info!("Unprovisioned — waiting for config over serial (use the web Device Setup page)");
    let mut idle = 0u32;
    loop {
        match lines.next_line() {
            Some(line) => p.handle_line(&line, &mut || wifi.scan()),
            None => {
                idle += 1;
                if idle % 15 == 0 {
                    log::info!("Unprovisioned — waiting for config over serial");
                }
            }
        }
    }
}
