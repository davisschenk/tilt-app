use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;

use btleplug::api::{Central, CentralEvent, Manager as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::{Stream, StreamExt};
use shared::{TiltColor, TiltReading};
use uuid::Uuid;

const APPLE_COMPANY_ID: u16 = 0x004C;
const IBEACON_TYPE: u8 = 0x02;
const IBEACON_LENGTH: u8 = 0x15;

pub struct TiltScanner {
    adapter: Adapter,
    /// Event stream created once and kept alive for the lifetime of the scanner.
    /// Dropping it triggers a bluez-async panic (D-Bus match cleanup race),
    /// so we hold it forever and reuse it across scan cycles.
    events: Pin<Box<dyn Stream<Item = CentralEvent> + Send>>,
}

impl TiltScanner {
    pub async fn new() -> anyhow::Result<Self> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No Bluetooth adapter found"))?;
        tracing::info!("Using BLE adapter: {:?}", adapter.adapter_info().await?);

        let events = adapter.events().await
            .map_err(|e| anyhow::anyhow!("failed to get event stream: {e:#}"))?;

        Ok(Self { adapter, events })
    }

    pub async fn scan_once(&mut self, duration: Duration) -> anyhow::Result<Vec<TiltReading>> {
        // Stop any previous scan to reset BlueZ discovery state.
        // This forces re-discovery and fresh ManufacturerDataAdvertisement events.
        let _ = self.adapter.stop_scan().await;

        self.adapter.start_scan(ScanFilter::default()).await
            .map_err(|e| anyhow::anyhow!("start_scan failed: {e:#}"))?;

        let mut latest: HashMap<TiltColor, TiltReading> = HashMap::new();
        let deadline = tokio::time::Instant::now() + duration;

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                event = self.events.next() => {
                    match event {
                        Some(CentralEvent::ManufacturerDataAdvertisement { manufacturer_data, .. }) => {
                            if let Some(data) = manufacturer_data.get(&APPLE_COMPANY_ID) {
                                if let Some(reading) = parse_ibeacon_tilt(data) {
                                    tracing::debug!(
                                        color = ?reading.color,
                                        temp = reading.temperature_f,
                                        gravity = reading.gravity,
                                        "Tilt advertisement"
                                    );
                                    latest.insert(reading.color, reading);
                                }
                            }
                        }
                        None => {
                            tracing::warn!("BLE event stream ended unexpectedly");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        let _ = self.adapter.stop_scan().await;

        Ok(latest.into_values().collect())
    }
}

pub fn parse_ibeacon_tilt(data: &[u8]) -> Option<TiltReading> {
    // iBeacon manufacturer data (after company ID):
    // [0] = 0x02 (iBeacon type)
    // [1] = 0x15 (length = 21 bytes)
    // [2..18] = UUID (16 bytes)
    // [18..20] = Major (u16 big-endian) = temperature °F
    // [20..22] = Minor (u16 big-endian) = gravity * 1000
    // [22] = TX Power (i8)
    if data.len() < 23 {
        return None;
    }
    if data[0] != IBEACON_TYPE || data[1] != IBEACON_LENGTH {
        return None;
    }

    let uuid = Uuid::from_bytes([
        data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
        data[12], data[13], data[14], data[15], data[16], data[17],
    ]);

    let color = TiltColor::from_uuid(&uuid)?;

    let temperature_f = u16::from_be_bytes([data[18], data[19]]) as f64;
    let gravity = u16::from_be_bytes([data[20], data[21]]) as f64 / 1000.0;
    let _tx_power = data[22] as i8;

    Some(TiltReading::new(
        color,
        temperature_f,
        gravity,
        None,
        chrono::Utc::now(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ibeacon_data(uuid_bytes: [u8; 16], major: u16, minor: u16, tx_power: i8) -> Vec<u8> {
        let mut data = vec![IBEACON_TYPE, IBEACON_LENGTH];
        data.extend_from_slice(&uuid_bytes);
        data.extend_from_slice(&major.to_be_bytes());
        data.extend_from_slice(&minor.to_be_bytes());
        data.push(tx_power as u8);
        data
    }

    fn red_uuid_bytes() -> [u8; 16] {
        *TiltColor::Red.uuid().as_bytes()
    }

    #[test]
    fn parse_valid_red_tilt() {
        let data = make_ibeacon_data(red_uuid_bytes(), 68, 1016, -59);
        let reading = parse_ibeacon_tilt(&data).unwrap();
        assert_eq!(reading.color, TiltColor::Red);
        assert!((reading.temperature_f - 68.0).abs() < f64::EPSILON);
        assert!((reading.gravity - 1.016).abs() < 0.0001);
    }

    #[test]
    fn parse_temperature_big_endian() {
        // 0x00, 0x44 = 68 in big-endian
        let data = make_ibeacon_data(red_uuid_bytes(), 0x0044, 1000, 0);
        let reading = parse_ibeacon_tilt(&data).unwrap();
        assert!((reading.temperature_f - 68.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_gravity_big_endian() {
        // 0x03, 0xF8 = 1016 in big-endian → 1.016
        let data = make_ibeacon_data(red_uuid_bytes(), 70, 0x03F8, 0);
        let reading = parse_ibeacon_tilt(&data).unwrap();
        assert!((reading.gravity - 1.016).abs() < 0.0001);
    }

    #[test]
    fn parse_all_8_tilt_colors() {
        for color in TiltColor::all() {
            let uuid_bytes = *color.uuid().as_bytes();
            let data = make_ibeacon_data(uuid_bytes, 72, 1050, -60);
            let reading = parse_ibeacon_tilt(&data).unwrap();
            assert_eq!(reading.color, *color);
        }
    }

    #[test]
    fn parse_unknown_uuid_returns_none() {
        let unknown = [0u8; 16];
        let data = make_ibeacon_data(unknown, 70, 1000, 0);
        assert!(parse_ibeacon_tilt(&data).is_none());
    }

    #[test]
    fn parse_too_short_data_returns_none() {
        let data = vec![0x02, 0x15, 0x00];
        assert!(parse_ibeacon_tilt(&data).is_none());
    }

    #[test]
    fn parse_wrong_type_returns_none() {
        let mut data = make_ibeacon_data(red_uuid_bytes(), 70, 1000, 0);
        data[0] = 0xFF; // wrong type
        assert!(parse_ibeacon_tilt(&data).is_none());
    }
}
