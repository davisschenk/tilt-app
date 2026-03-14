mod ble;
mod buffer;
mod config;
mod http;
mod tilt;
mod wifi;

use anyhow::{Context, Result};

/// Subscribe the current task to the Task Watchdog Timer (TWDT).
/// The TWDT is configured in sdkconfig.defaults with a 120s timeout.
/// If `feed_watchdog()` is not called within the timeout, the ESP32 reboots.
fn init_watchdog() -> Result<()> {
    unsafe {
        let ret = esp_idf_svc::sys::esp_task_wdt_add(core::ptr::null_mut());
        if ret != esp_idf_svc::sys::ESP_OK {
            anyhow::bail!("Failed to subscribe to TWDT: error code {}", ret);
        }
    }
    log::info!("Watchdog timer initialized (timeout configured in sdkconfig.defaults)");
    Ok(())
}

/// Feed (reset) the Task Watchdog Timer. Must be called every scan cycle.
fn feed_watchdog() -> Result<()> {
    unsafe {
        let ret = esp_idf_svc::sys::esp_task_wdt_reset();
        if ret != esp_idf_svc::sys::ESP_OK {
            anyhow::bail!("Failed to feed TWDT: error code {}", ret);
        }
    }
    Ok(())
}

fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Tilt ESP32 client starting...");

    if let Err(e) = run() {
        log::error!("Fatal error: {:?}", e);
        // Allow the watchdog to reboot us
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

fn run() -> Result<()> {
    // Initialize watchdog
    init_watchdog().context("Watchdog initialization failed")?;

    // TODO: Initialize WiFi, BLE scanner, HTTP uploader, and enter main loop
    // This will be implemented in the "Structured main loop" task

    feed_watchdog()?;

    Ok(())
}
