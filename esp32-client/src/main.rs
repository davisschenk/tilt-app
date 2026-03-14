mod ble;
mod buffer;
mod config;
mod http;
mod tilt;
mod wifi;

use std::time::Instant;

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
    let mut cfg = config::CONFIG;

    // Take NVS partition early so we can apply overrides before validation
    let nvs = esp_idf_svc::nvs::EspDefaultNvsPartition::take()
        .context("Failed to take NVS partition")?;
    config::apply_nvs_overrides(&mut cfg, &nvs);

    config::log_config(&cfg);
    config::validate_config(&cfg).context("Configuration validation failed")?;

    // Initialize watchdog
    init_watchdog().context("Watchdog initialization failed")?;

    // Initialize WiFi
    let peripherals =
        esp_idf_svc::hal::peripherals::Peripherals::take().context("Failed to take peripherals")?;
    let sys_loop =
        esp_idf_svc::eventloop::EspSystemEventLoop::take().context("Failed to take event loop")?;

    let mut wifi_manager =
        wifi::WifiManager::new(peripherals.modem, sys_loop, nvs, cfg.wifi_ssid, cfg.wifi_password)
            .context("Failed to create WiFi manager")?;
    wifi_manager.connect().context("Initial WiFi connection failed")?;

    // Initialize BLE scanner
    let mut ble_scanner = ble::BleScanner::new().context("Failed to initialize BLE scanner")?;

    // Initialize HTTP uploader
    let uploader = http::HttpUploader::new(cfg.server_url, cfg.api_key);

    // Initialize reading buffer and backoff
    let mut reading_buffer = buffer::ReadingBuffer::new(cfg.buffer_capacity as usize);
    let mut backoff = buffer::Backoff::new(1000, 60_000, 2);

    let mut consecutive_errors: u32 = 0;
    let scan_interval = std::time::Duration::from_secs(cfg.scan_interval_secs as u64);
    let start_time = Instant::now();
    let mut total_scans: u64 = 0;
    let mut successful_uploads: u64 = 0;
    let mut failed_uploads: u64 = 0;
    let health_interval = cfg.health_report_interval_cycles as u64;

    log::info!("Entering main scan-upload loop");

    loop {
        total_scans = total_scans.wrapping_add(1);

        // Phase 1: Scan for Tilt hydrometers
        let readings = match ble_scanner.scan_for_tilts(cfg.scan_interval_secs) {
            Ok(r) => {
                ble_scanner.reset_recovery_counter();
                r
            }
            Err(e) => {
                log::warn!("BLE scan error: {:?}", e);
                ble_scanner.attempt_recovery(&e);
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
                        successful_uploads = successful_uploads.wrapping_add(1);
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
                        failed_uploads = failed_uploads.wrapping_add(1);
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

        // Phase 3: Periodic health report
        if health_interval > 0 && total_scans % health_interval == 0 {
            let uptime = start_time.elapsed();
            let hours = uptime.as_secs() / 3600;
            let minutes = (uptime.as_secs() % 3600) / 60;
            let free_heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };
            log::info!(
                "HEALTH: uptime={}h{}m scans={} uploads_ok={} uploads_fail={} buffer={} wifi={} heap={} errors={}",
                hours, minutes, total_scans, successful_uploads, failed_uploads,
                reading_buffer.len(), wifi_manager.is_connected(), free_heap, consecutive_errors,
            );
        }

        // Phase 4: Feed watchdog — always, regardless of success/failure
        let _ = feed_watchdog();

        // Phase 5: Sleep until next scan cycle
        std::thread::sleep(scan_interval);
    }
}
