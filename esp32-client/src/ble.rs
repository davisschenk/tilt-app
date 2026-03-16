//! BLE scanning module.
//!
//! Handles NimBLE BLE stack initialization, passive scanning for iBeacon
//! advertisements, and integration with the Tilt parser to produce readings.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use esp32_nimble::{BLEDevice, BLEScan};
use esp_idf_svc::hal::task::block_on;

use crate::tilt::{self, TiltColor, TiltReading};

const MAX_RECOVERY_FAILURES: u32 = 3;

pub struct BleScanner {
    ble_device: &'static BLEDevice,
    scan_interval: u16,
    scan_window: u16,
    consecutive_recovery_failures: u32,
}

impl BleScanner {
    pub fn new() -> Result<Self> {
        let ble_device = BLEDevice::take();
        log::info!("NimBLE BLE stack initialized");
        Ok(Self {
            ble_device,
            scan_interval: 100,
            scan_window: 99,
            consecutive_recovery_failures: 0,
        })
    }

    /// Attempt to recover the BLE stack after a scan failure.
    /// Deinits and reinits the NimBLE host stack. After 3 consecutive
    /// recovery failures, triggers a full device reboot via esp_restart().
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

    pub fn scan_for_tilts(&mut self, duration_secs: u32) -> Result<Vec<TiltReading>> {
        let readings: Arc<Mutex<HashMap<TiltColor, TiltReading>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let readings_clone = readings.clone();

        log::debug!("Starting BLE scan for {}s", duration_secs);

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
                    (duration_secs * 1000) as i32,
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
