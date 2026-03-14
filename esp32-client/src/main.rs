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
    let cfg = config::CONFIG;
    log::info!(
        "Config: server_url={}, scan_interval={}s, upload_interval={}s, buffer_capacity={}",
        cfg.server_url,
        cfg.scan_interval_secs,
        cfg.upload_interval_secs,
        cfg.buffer_capacity,
    );

    // Initialize watchdog
    init_watchdog().context("Watchdog initialization failed")?;

    // Initialize WiFi
    let peripherals =
        esp_idf_svc::hal::peripherals::Peripherals::take().context("Failed to take peripherals")?;
    let sys_loop =
        esp_idf_svc::eventloop::EspSystemEventLoop::take().context("Failed to take event loop")?;
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take()
        .context("Failed to take NVS partition")?;

    let mut wifi_manager =
        wifi::WifiManager::new(peripherals.modem, sys_loop, nvs, cfg.wifi_ssid, cfg.wifi_password)
            .context("Failed to create WiFi manager")?;
    wifi_manager.connect().context("Initial WiFi connection failed")?;

    // Initialize BLE scanner
    let ble_scanner = ble::BleScanner::new().context("Failed to initialize BLE scanner")?;

    // Initialize HTTP uploader
    let uploader = http::HttpUploader::new(cfg.server_url, cfg.api_key);

    // Initialize reading buffer and backoff
    let mut reading_buffer = buffer::ReadingBuffer::new(cfg.buffer_capacity as usize);
    let mut backoff = buffer::Backoff::new(1000, 60_000, 2);

    let mut consecutive_errors: u32 = 0;
    let scan_interval = std::time::Duration::from_secs(cfg.scan_interval_secs as u64);

    log::info!("Entering main scan-upload loop");

    loop {
        // Phase 1: Scan for Tilt hydrometers
        let readings = match ble_scanner.scan_for_tilts(cfg.scan_interval_secs) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("BLE scan error: {:?}", e);
                consecutive_errors += 1;
                if consecutive_errors >= 10 {
                    log::error!(
                        "10+ consecutive errors! wifi={}, buffer={}, errors={}",
                        wifi_manager.is_connected(),
                        reading_buffer.len(),
                        consecutive_errors,
                    );
                }
                // Feed watchdog and continue to next cycle
                let _ = feed_watchdog();
                std::thread::sleep(scan_interval);
                continue;
            }
        };

        // Phase 2: Upload readings if any
        if !readings.is_empty() || !reading_buffer.is_empty() {
            // Ensure WiFi is connected before uploading
            if let Err(e) = wifi_manager.ensure_connected() {
                log::warn!("WiFi reconnect failed: {:?}", e);
                reading_buffer.push_batch(&readings);
                consecutive_errors += 1;
            } else {
                // Prepend buffered readings to current batch
                let mut all_readings = reading_buffer.drain_all();
                all_readings.extend_from_slice(&readings);

                match uploader.upload_batch(&all_readings) {
                    Ok(()) => {
                        log::info!("Uploaded {} readings", all_readings.len());
                        backoff.reset();
                        consecutive_errors = 0;
                    }
                    Err(e) => {
                        log::warn!(
                            "Upload failed (backoff={}ms, buffer={}): {:?}",
                            backoff.current_delay_ms(),
                            reading_buffer.len() + all_readings.len(),
                            e,
                        );
                        // Re-buffer all readings that failed to upload
                        reading_buffer.push_batch(&all_readings);
                        let delay = backoff.next_delay();
                        std::thread::sleep(delay);
                        consecutive_errors += 1;
                    }
                }
            }

            if consecutive_errors >= 10 {
                log::error!(
                    "10+ consecutive errors! wifi={}, buffer={}, errors={}",
                    wifi_manager.is_connected(),
                    reading_buffer.len(),
                    consecutive_errors,
                );
            }
        }

        // Phase 3: Feed watchdog — always, regardless of success/failure
        let _ = feed_watchdog();

        // Phase 4: Sleep until next scan cycle
        std::thread::sleep(scan_interval);
    }
}
