//! BLE scanning module.
//!
//! Handles NimBLE BLE stack initialization, passive scanning for iBeacon
//! advertisements, and integration with the Tilt parser to produce readings.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use anyhow::{Context, Result};
use esp32_nimble::{BLEDevice, BLEScan};
use esp_idf_svc::hal::task::block_on;

use crate::tilt::{self, TiltColor, TiltReading};

const MAX_RECOVERY_FAILURES: u32 = 3;

/// Manages the NimBLE BLE stack and scans for Tilt hydrometer advertisements.
pub struct BleScanner {
    ble_device: &'static BLEDevice,
    /// BLE scan interval in units of 0.625 ms (100 = 62.5 ms).
    scan_interval: u16,
    /// BLE scan window in units of 0.625 ms. Must be ≤ scan_interval.
    scan_window: u16,
    consecutive_recovery_failures: u32,
    /// Maximum duration of a single scan chunk in seconds.
    /// Long scans are broken into chunks so the watchdog can be fed between them.
    max_scan_chunk_secs: u32,
}

impl BleScanner {
    /// Initialize the NimBLE BLE stack and return a ready-to-use scanner.
    ///
    /// `max_scan_chunk_secs` controls how long each internal scan chunk runs
    /// before the watchdog is fed. Set from `config.max_scan_chunk_secs`.
    pub fn new(max_scan_chunk_secs: u32) -> Result<Self> {
        let ble_device = BLEDevice::take();
        log::info!("NimBLE BLE stack initialized");
        Ok(Self {
            ble_device,
            scan_interval: 100,
            scan_window: 99,
            consecutive_recovery_failures: 0,
            max_scan_chunk_secs,
        })
    }

    /// Attempt to recover the BLE stack after a scan failure.
    ///
    /// Deinits and reinits the NimBLE host stack. After `MAX_RECOVERY_FAILURES`
    /// (3) consecutive recovery failures — indicating the BLE hardware is in an
    /// unrecoverable state — triggers a full device reboot via `esp_restart()`.
    pub fn attempt_recovery(&mut self, original_error: &anyhow::Error) {
        log::warn!(
            "Attempting BLE stack recovery (failure #{}) due to: {:?}",
            self.consecutive_recovery_failures + 1,
            original_error,
        );

        // Deinitialize NimBLE
        if let Err(e) = BLEDevice::deinit() {
            log::warn!("BLE deinit failed: {:?}", e);
            self.consecutive_recovery_failures += 1;
            if self.consecutive_recovery_failures >= MAX_RECOVERY_FAILURES {
                log::error!(
                    "BLE recovery failed {} times, rebooting device!",
                    self.consecutive_recovery_failures,
                );
                unsafe {
                    // Safety: esp_restart() triggers a full SoC reset. No
                    // memory safety invariants apply — the device reboots
                    // immediately. Called only as a last resort after
                    // MAX_RECOVERY_FAILURES consecutive BLE stack failures.
                    esp_idf_svc::sys::esp_restart();
                }
            }
            return;
        }

        // Reinitialize NimBLE
        self.ble_device = BLEDevice::take();
        self.consecutive_recovery_failures = 0;
        log::info!("BLE stack recovery successful");
    }

    /// Reset the recovery failure counter after a successful scan.
    pub fn reset_recovery_counter(&mut self) {
        self.consecutive_recovery_failures = 0;
    }

    /// Scan for Tilt hydrometer BLE advertisements for `duration_secs` seconds.
    ///
    /// Long scans are split into chunks of `max_scan_chunk_secs` with watchdog
    /// feeds between each chunk. Returns one reading per Tilt color detected;
    /// if a color is seen multiple times the last advertisement wins.
    pub fn scan_for_tilts(&mut self, duration_secs: u32) -> Result<Vec<TiltReading>> {
        let readings: Arc<Mutex<HashMap<TiltColor, TiltReading>>> =
            Arc::new(Mutex::new(HashMap::new()));

        log::debug!("Starting BLE scan for {}s", duration_secs);

        // Break long scans into chunks of max_scan_chunk_secs, feeding the
        // watchdog between each chunk so the TWDT doesn't fire.
        let mut remaining = duration_secs;
        while remaining > 0 {
            let chunk = remaining.min(self.max_scan_chunk_secs);
            remaining -= chunk;

            let readings_clone = readings.clone();
            block_on(async {
                let mut ble_scan = BLEScan::new();
                ble_scan
                    .active_scan(false)
                    .interval(self.scan_interval)
                    .window(self.scan_window)
                    .filter_duplicates(false);

                ble_scan
                    .start(
                        self.ble_device,
                        (chunk * 1000) as i32,
                        |device, data| {
                            let rssi = device.rssi();
                            if let Some(mfg_data) = data.manufacture_data() {
                                // Apple company ID is 0x004C
                                if mfg_data.company_identifier == 0x004C {
                                    if let Some(mut reading) =
                                        tilt::parse_ibeacon(mfg_data.payload)
                                    {
                                        reading.rssi = Some(rssi as i16);
                                        log::info!(
                                            "Tilt {:?}: temp={:.1}°F gravity={:.4} rssi={}",
                                            reading.color,
                                            reading.temperature_f,
                                            reading.gravity,
                                            rssi
                                        );
                                        if let Ok(mut map) = readings_clone.lock() {
                                            map.insert(reading.color, reading);
                                        }
                                    }
                                }
                            }
                            None::<()>
                        },
                    )
                    .await
                    .context("BLE scan failed")?;

                Ok::<(), anyhow::Error>(())
            })?;

            // Stamp recorded_at for any readings that were captured in this chunk.
            // Done here (on the main task stack) rather than inside the nimble_host
            // callback to avoid blowing the NimBLE host task stack.
            let chunk_ts = tilt::format_timestamp(SystemTime::now());
            if let Ok(mut map) = readings.lock() {
                for reading in map.values_mut() {
                    if reading.recorded_at.is_empty() {
                        reading.recorded_at = chunk_ts.clone();
                    }
                }
            }

            // Feed the watchdog between scan chunks
            crate::feed_watchdog_or_warn();
        }

        let result: Vec<TiltReading> = readings
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock readings: {}", e))?
            .drain()
            .map(|(_, v)| v)
            .collect();

        log::debug!("BLE scan complete: {} Tilt(s) found", result.len());
        Ok(result)
    }
}
