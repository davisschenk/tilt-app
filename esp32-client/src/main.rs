mod ble;
mod buffer;
mod config;
mod http;
mod ota;
mod tilt;
mod wifi;

use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use esp_idf_svc::sntp::{EspSntp, SyncStatus};

/// Reconfigure and subscribe the current task to the Task Watchdog Timer (TWDT).
///
/// `timeout_ms` is taken from `config.watchdog_timeout_secs` so it stays in
/// sync with the rest of the cycle timing. sdkconfig.defaults values are not
/// reliably picked up by the ESP-IDF build system, hence the programmatic set.
pub(crate) fn init_watchdog(timeout_ms: u32) -> Result<()> {
    unsafe {
        // Safety: esp_task_wdt_reconfigure() and esp_task_wdt_add() are safe to
        // call during single-threaded init before the main loop starts. The
        // config struct is fully initialized above with no uninitialized fields.
        let config = esp_idf_svc::sys::esp_task_wdt_config_t {
            timeout_ms,
            idle_core_mask: 0, // don't monitor any idle tasks
            trigger_panic: true,
        };
        let ret = esp_idf_svc::sys::esp_task_wdt_reconfigure(&config);
        if ret != esp_idf_svc::sys::ESP_OK {
            anyhow::bail!("Failed to reconfigure TWDT: error code {}", ret);
        }

        let ret = esp_idf_svc::sys::esp_task_wdt_add(core::ptr::null_mut());
        if ret != esp_idf_svc::sys::ESP_OK {
            anyhow::bail!("Failed to subscribe to TWDT: error code {}", ret);
        }
    }
    log::info!("Watchdog timer initialized ({}s timeout, panic enabled)", timeout_ms / 1000);
    Ok(())
}

/// Feed (reset) the Task Watchdog Timer. Must be called every scan cycle.
pub(crate) fn feed_watchdog() -> Result<()> {
    unsafe {
        // Safety: esp_task_wdt_reset() is safe to call from any task that has
        // previously subscribed via esp_task_wdt_add(). The main task subscribes
        // during init_watchdog() before this code path is reachable.
        let ret = esp_idf_svc::sys::esp_task_wdt_reset();
        if ret != esp_idf_svc::sys::ESP_OK {
            anyhow::bail!("Failed to feed TWDT: error code {}", ret);
        }
    }
    Ok(())
}

/// Feed the watchdog, logging a warning if the feed fails rather than silently
/// ignoring the error. Use this at all non-critical call sites in the main loop.
pub(crate) fn feed_watchdog_or_warn() {
    if let Err(e) = feed_watchdog() {
        log::warn!("Failed to feed watchdog: {:?}", e);
    }
}

/// Sleep for `duration`, feeding the watchdog every `chunk_secs` seconds.
///
/// A plain `thread::sleep(scan_interval)` would block for the full interval
/// without any watchdog feed — guaranteed to fire the TWDT when the timeout
/// equals the scan interval. This helper breaks the sleep into chunks so the
/// watchdog is fed throughout.
fn sleep_feeding_watchdog(duration: Duration, chunk_secs: u32) {
    let chunk = Duration::from_secs(chunk_secs as u64);
    let mut remaining = duration;
    while remaining > Duration::ZERO {
        let nap = remaining.min(chunk);
        std::thread::sleep(nap);
        remaining = remaining.saturating_sub(nap);
        feed_watchdog_or_warn();
    }
}

/// Temporarily unsubscribe the current task from the TWDT.
/// Use before long-running blocking calls (e.g. WiFi connect) where the
/// watchdog cannot be fed. Call `resume_watchdog()` afterwards.
pub(crate) fn suspend_watchdog() {
    unsafe {
        // Safety: esp_task_wdt_delete(null_mut()) unsubscribes the calling task.
        // Safe to call whenever the task is currently subscribed. Called only
        // before long blocking operations (WiFi connect) where we cannot feed.
        let ret = esp_idf_svc::sys::esp_task_wdt_delete(core::ptr::null_mut());
        if ret != esp_idf_svc::sys::ESP_OK {
            log::warn!("Failed to unsubscribe from TWDT: error code {}", ret);
        }
    }
}

/// Re-subscribe the current task to the TWDT after a `suspend_watchdog()` call.
pub(crate) fn resume_watchdog() {
    unsafe {
        // Safety: esp_task_wdt_add(null_mut()) re-subscribes the calling task.
        // Must only be called after a matching suspend_watchdog(). The reset
        // call that follows starts a fresh timeout window.
        let ret = esp_idf_svc::sys::esp_task_wdt_add(core::ptr::null_mut());
        if ret != esp_idf_svc::sys::ESP_OK {
            log::warn!("Failed to re-subscribe to TWDT: error code {}", ret);
        }
        // Immediately feed so the full timeout window starts fresh
        let _ = esp_idf_svc::sys::esp_task_wdt_reset();
    }
}

/// Shared mutable state threaded through the main scan-upload loop.
struct LoopState {
    /// Total scan cycles completed (wraps at u32::MAX).
    scan_count: u32,
    /// Number of successful upload batches.
    upload_ok_count: u32,
    /// Number of failed upload attempts.
    upload_fail_count: u32,
    /// Consecutive cycles that ended in an error (reset on any success).
    consecutive_errors: u32,
    /// Readings buffered while the server is unreachable.
    reading_buffer: buffer::ReadingBuffer,
    /// Exponential backoff state for upload retries.
    backoff: buffer::Backoff,
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
            std::thread::sleep(Duration::from_secs(1));
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

    // Initialize watchdog — use the config value so NVS overrides are respected
    init_watchdog(cfg.watchdog_timeout_secs * 1000).context("Watchdog initialization failed")?;

    // Initialize WiFi
    let peripherals =
        esp_idf_svc::hal::peripherals::Peripherals::take().context("Failed to take peripherals")?;
    let sys_loop =
        esp_idf_svc::eventloop::EspSystemEventLoop::take().context("Failed to take event loop")?;

    let mut wifi_manager =
        wifi::WifiManager::new(peripherals.modem, sys_loop, nvs, cfg.wifi_ssid, cfg.wifi_password)
            .context("Failed to create WiFi manager")?;
    wifi_manager.connect().context("Initial WiFi connection failed")?;
    feed_watchdog_or_warn();

    // Initialize SNTP time sync
    let _sntp = EspSntp::new_default().context("Failed to initialize SNTP")?;
    log::info!("SNTP initialized, waiting for time sync...");
    // Wait up to 15 seconds for initial time sync
    for i in 0..30 {
        if _sntp.get_sync_status() == SyncStatus::Completed {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
        // Feed watchdog every 5 iterations (~2.5s) during SNTP wait
        if i % 5 == 4 {
            feed_watchdog_or_warn();
        }
    }
    if _sntp.get_sync_status() == SyncStatus::Completed {
        log::info!("Time synced: {}", tilt::format_timestamp(SystemTime::now()));
    } else {
        log::warn!("SNTP sync not yet complete, timestamps may be inaccurate");
    }

    feed_watchdog_or_warn();

    // Initialize BLE scanner
    let mut ble_scanner =
        ble::BleScanner::new(cfg.max_scan_chunk_secs).context("Failed to initialize BLE scanner")?;

    feed_watchdog_or_warn();

    // Initialize HTTP uploader
    let uploader = http::HttpUploader::new(cfg.server_url, cfg.api_key);

    let mut state = LoopState {
        scan_count: 0,
        upload_ok_count: 0,
        upload_fail_count: 0,
        consecutive_errors: 0,
        reading_buffer: buffer::ReadingBuffer::new(cfg.buffer_capacity as usize),
        backoff: buffer::Backoff::new(1000, 60_000, 2),
    };

    let scan_interval = Duration::from_secs(cfg.scan_interval_secs as u64);
    let start_time = Instant::now();

    log::info!("Entering main scan-upload loop");

    loop {
        state.scan_count = state.scan_count.wrapping_add(1);
        let cycle_start = Instant::now();

        // Phase 1: Scan for Tilt hydrometers
        let readings = match run_ble_phase(&mut ble_scanner, &cfg) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("BLE scan error: {:?}", e);
                ble_scanner.attempt_recovery(&e);
                state.consecutive_errors += 1;
                if state.consecutive_errors >= config::CONSECUTIVE_ERROR_WARN_THRESHOLD {
                    log::error!(
                        "{}+ consecutive errors! wifi={}, buffer={}, errors={}",
                        config::CONSECUTIVE_ERROR_WARN_THRESHOLD,
                        wifi_manager.is_connected(),
                        state.reading_buffer.len(),
                        state.consecutive_errors,
                    );
                }
                feed_watchdog_or_warn();
                // Sleep only the time remaining in this cycle (scan already consumed some)
                let elapsed = cycle_start.elapsed();
                if let Some(remaining) = scan_interval.checked_sub(elapsed) {
                    sleep_feeding_watchdog(remaining, cfg.max_scan_chunk_secs);
                }
                continue;
            }
        };

        // Phase 2: Upload readings
        run_upload_phase(&mut state, &uploader, &mut wifi_manager, &readings, &cfg);

        // Phase 3: Periodic health report
        run_health_phase(&state, &start_time, &wifi_manager, &cfg);

        // Phase 4: Periodic OTA version check
        if let Err(e) = run_ota_phase(&uploader, &cfg, state.scan_count) {
            log::warn!("OTA phase error: {:?}", e);
        }

        // Phase 5: Feed watchdog — always, regardless of success/failure
        feed_watchdog_or_warn();

        // Phase 6: Sleep for the remainder of the scan interval.
        // The BLE scan phase already consumed scan_interval_secs, so we only
        // sleep whatever time is left after upload/health/OTA phases completed.
        // This prevents the cycle from taking 2× scan_interval.
        let elapsed = cycle_start.elapsed();
        if let Some(remaining) = scan_interval.checked_sub(elapsed) {
            sleep_feeding_watchdog(remaining, cfg.max_scan_chunk_secs);
        }
    }
}

/// Phase 1: Run a BLE scan and return all Tilt readings found.
///
/// Resets the recovery counter on success so that the next failure starts
/// counting from zero rather than carrying over stale state.
fn run_ble_phase(
    scanner: &mut ble::BleScanner,
    cfg: &config::Config,
) -> Result<Vec<tilt::TiltReading>> {
    let readings = scanner.scan_for_tilts(cfg.scan_interval_secs, cfg.min_samples_per_color as usize)?;
    scanner.reset_recovery_counter();
    Ok(readings)
}

/// Phase 2: Drain the buffer and upload all readings (buffered + current) to the server.
///
/// On WiFi failure, current readings are buffered for the next cycle.
/// On upload failure, all readings are re-buffered and the backoff timer advances.
/// On success, the backoff resets and the consecutive error counter clears.
fn run_upload_phase(
    state: &mut LoopState,
    uploader: &http::HttpUploader,
    wifi: &mut wifi::WifiManager,
    readings: &[tilt::TiltReading],
    cfg: &config::Config,
) {
    if readings.is_empty() && state.reading_buffer.is_empty() {
        return;
    }

    if let Err(e) = wifi.ensure_connected() {
        log::warn!("WiFi reconnect failed: {:?}", e);
        state.reading_buffer.push_batch(&readings);
        state.consecutive_errors += 1;
        if state.consecutive_errors >= config::CONSECUTIVE_ERROR_WARN_THRESHOLD {
            log::error!(
                "{}+ consecutive errors! wifi={}, buffer={}, errors={}",
                config::CONSECUTIVE_ERROR_WARN_THRESHOLD,
                wifi.is_connected(),
                state.reading_buffer.len(),
                state.consecutive_errors,
            );
        }
        return;
    }

    // Prepend buffered readings to current batch
    let mut all_readings = state.reading_buffer.drain_all();
    all_readings.extend_from_slice(&readings);

    // Stamp readings with current time
    let now_ts = tilt::format_timestamp(SystemTime::now());
    for r in &mut all_readings {
        if r.recorded_at.is_empty() {
            r.recorded_at = now_ts.clone();
        }
    }

    match uploader.upload_batch(&all_readings) {
        Ok(()) => {
            log::info!("Uploaded {} readings", all_readings.len());
            state.backoff.reset();
            state.consecutive_errors = 0;
            state.upload_ok_count = state.upload_ok_count.wrapping_add(1);
        }
        Err(e) => {
            log::warn!(
                "Upload failed (backoff={}ms, buffer={}): {:?}",
                state.backoff.current_delay_ms(),
                all_readings.len(),
                e,
            );
            // Re-buffer all readings that failed to upload
            state.reading_buffer.push_batch(&all_readings);
            let delay = state.backoff.next_delay();
            // Use sleep_feeding_watchdog so the TWDT is fed during the
            // backoff wait — a plain sleep() would starve it on long delays.
            sleep_feeding_watchdog(delay, cfg.max_scan_chunk_secs);
            state.consecutive_errors += 1;
            state.upload_fail_count = state.upload_fail_count.wrapping_add(1);
        }
    }

    if state.consecutive_errors >= config::CONSECUTIVE_ERROR_WARN_THRESHOLD {
        log::error!(
            "{}+ consecutive errors! wifi={}, buffer={}, errors={}",
            config::CONSECUTIVE_ERROR_WARN_THRESHOLD,
            wifi.is_connected(),
            state.reading_buffer.len(),
            state.consecutive_errors,
        );
    }
}

/// Phase 3: Log a health report every `health_report_interval_cycles` scans.
///
/// Includes uptime, scan/upload counts, buffer depth, WiFi status, free heap,
/// and the consecutive error count. No-ops when the interval is 0.
fn run_health_phase(
    state: &LoopState,
    start_time: &Instant,
    wifi: &wifi::WifiManager,
    cfg: &config::Config,
) {
    let interval = cfg.health_report_interval_cycles;
    if interval == 0 || state.scan_count % interval != 0 {
        return;
    }
    let uptime = start_time.elapsed();
    let hours = uptime.as_secs() / 3600;
    let minutes = (uptime.as_secs() % 3600) / 60;
    // Safety: esp_get_free_heap_size() is a pure read of an atomic counter
    // maintained by the heap allocator. Safe to call from any context.
    let free_heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };
    log::info!(
        "HEALTH: uptime={}h{}m scans={} uploads_ok={} uploads_fail={} buffer={} wifi={} heap={} errors={}",
        hours, minutes, state.scan_count, state.upload_ok_count, state.upload_fail_count,
        state.reading_buffer.len(), wifi.is_connected(), free_heap, state.consecutive_errors,
    );
}

/// Phase 4: Check for an OTA firmware update every `ota_check_interval_cycles` scans.
///
/// Fetches the server's OTA manifest and compares the `version` field against the
/// running firmware version. If a newer version is available and the `url` field is
/// a valid HTTP/HTTPS URL, downloads and flashes the firmware then reboots.
///
/// Returns `Err` if the OTA manifest is missing required fields or the URL scheme
/// is invalid. Network failures are logged and swallowed (return `Ok(())`).
fn run_ota_phase(
    uploader: &http::HttpUploader,
    cfg: &config::Config,
    cycle: u32,
) -> Result<()> {
    let interval = cfg.ota_check_interval_cycles;
    if interval == 0 || cycle % interval != 0 {
        return Ok(());
    }

    feed_watchdog_or_warn();

    let ota_url = format!("{}/api/v1/ota/firmware", cfg.server_url);
    let body = match uploader.get_json(&ota_url) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("OTA check failed: {:?}", e);
            return Ok(());
        }
    };

    let server_version = body
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("OTA response missing 'version' field"))?;

    let firmware_url = body
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("OTA response missing 'url' field"))?;

    if !firmware_url.starts_with("http://") && !firmware_url.starts_with("https://") {
        anyhow::bail!("OTA firmware URL must be HTTP/HTTPS, got: '{}'", firmware_url);
    }

    if server_version == cfg.firmware_version {
        log::debug!("OTA: firmware up to date ({})", cfg.firmware_version);
        feed_watchdog_or_warn();
        return Ok(());
    }

    log::info!(
        "OTA: server version='{}' current='{}', starting update",
        server_version, cfg.firmware_version
    );

    match ota::OtaUpdater::perform_update(firmware_url) {
        Ok(()) => {
            log::info!("OTA complete, rebooting");
            esp_idf_svc::hal::reset::restart();
        }
        Err(e) => log::error!("OTA update failed: {:?}", e),
    }

    feed_watchdog_or_warn();
    Ok(())
}
