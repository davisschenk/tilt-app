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

pub struct BleScanner {
    ble_device: &'static BLEDevice,
    scan_interval: u16,
    scan_window: u16,
}

impl BleScanner {
    pub fn new() -> Result<Self> {
        let ble_device = BLEDevice::take();
        log::info!("NimBLE BLE stack initialized");
        Ok(Self {
            ble_device,
            scan_interval: 100,
            scan_window: 99,
        })
    }

    pub fn scan_for_tilts(&self, duration_secs: u32) -> Result<Vec<TiltReading>> {
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
                    |_device, data| {
                        let rssi = data.rssi();
                        if let Some(mfg_data) = data.manufacture_data() {
                            // Apple company ID is 0x004C (little-endian: 0x4C, 0x00)
                            if mfg_data.len() >= 2
                                && mfg_data[0] == 0x4C
                                && mfg_data[1] == 0x00
                            {
                                // Pass data after company ID to iBeacon parser
                                if let Some(mut reading) = tilt::parse_ibeacon(&mfg_data[2..]) {
                                    reading.rssi = Some(rssi as i8);
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
