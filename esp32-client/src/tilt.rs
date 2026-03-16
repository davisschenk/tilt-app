//! Tilt iBeacon parsing and color constants.
//!
//! Replicates the Tilt hydrometer types from the `shared` crate since the ESP32
//! client cannot depend on it (different toolchain/target). Contains TiltColor
//! enum, UUID constants, iBeacon parsing, and the TiltReading struct.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

const TILT_UUID_RED: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x10, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];
const TILT_UUID_GREEN: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x20, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];
const TILT_UUID_BLACK: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x30, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];
const TILT_UUID_PURPLE: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x40, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];
const TILT_UUID_ORANGE: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x50, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];
const TILT_UUID_BLUE: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x60, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];
const TILT_UUID_YELLOW: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x70, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];
const TILT_UUID_PINK: [u8; 16] = [
    0xA4, 0x95, 0xBB, 0x80, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
];

const ALL_TILT_UUIDS: [([u8; 16], TiltColor); 8] = [
    (TILT_UUID_RED, TiltColor::Red),
    (TILT_UUID_GREEN, TiltColor::Green),
    (TILT_UUID_BLACK, TiltColor::Black),
    (TILT_UUID_PURPLE, TiltColor::Purple),
    (TILT_UUID_ORANGE, TiltColor::Orange),
    (TILT_UUID_BLUE, TiltColor::Blue),
    (TILT_UUID_YELLOW, TiltColor::Yellow),
    (TILT_UUID_PINK, TiltColor::Pink),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TiltColor {
    Red,
    Green,
    Black,
    Purple,
    Orange,
    Blue,
    Yellow,
    Pink,
}

impl TiltColor {
    pub fn from_uuid_bytes(uuid: &[u8; 16]) -> Option<TiltColor> {
        ALL_TILT_UUIDS
            .iter()
            .find(|(u, _)| u == uuid)
            .map(|(_, color)| *color)
    }

    pub fn all() -> &'static [TiltColor] {
        &[
            TiltColor::Red,
            TiltColor::Green,
            TiltColor::Black,
            TiltColor::Purple,
            TiltColor::Orange,
            TiltColor::Blue,
            TiltColor::Yellow,
            TiltColor::Pink,
        ]
    }

    pub fn uuid_bytes(&self) -> &'static [u8; 16] {
        match self {
            TiltColor::Red => &TILT_UUID_RED,
            TiltColor::Green => &TILT_UUID_GREEN,
            TiltColor::Black => &TILT_UUID_BLACK,
            TiltColor::Purple => &TILT_UUID_PURPLE,
            TiltColor::Orange => &TILT_UUID_ORANGE,
            TiltColor::Blue => &TILT_UUID_BLUE,
            TiltColor::Yellow => &TILT_UUID_YELLOW,
            TiltColor::Pink => &TILT_UUID_PINK,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiltReading {
    pub color: TiltColor,
    pub temperature_f: f64,
    pub gravity: f64,
    pub rssi: Option<i16>,
    pub recorded_at: String,
}

impl TiltReading {
    pub fn new(
        color: TiltColor,
        temperature_f: f64,
        gravity: f64,
        rssi: Option<i16>,
        recorded_at: String,
    ) -> Self {
        Self {
            color,
            temperature_f,
            gravity,
            rssi,
            recorded_at,
        }
    }
}

pub const IBEACON_TYPE: u8 = 0x02;
pub const IBEACON_LENGTH: u8 = 0x15;

pub fn parse_ibeacon(manufacturer_data: &[u8]) -> Option<TiltReading> {
    // iBeacon manufacturer data (after company ID):
    // [0] = 0x02 (iBeacon type)
    // [1] = 0x15 (length = 21 bytes)
    // [2..18] = UUID (16 bytes)
    // [18..20] = Major (u16 big-endian) = temperature °F
    // [20..22] = Minor (u16 big-endian) = gravity * 1000
    // [22] = TX Power (i8)
    if manufacturer_data.len() < 23 {
        return None;
    }
    if manufacturer_data[0] != IBEACON_TYPE || manufacturer_data[1] != IBEACON_LENGTH {
        return None;
    }

    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&manufacturer_data[2..18]);

    let color = TiltColor::from_uuid_bytes(&uuid)?;

    let temperature_f = u16::from_be_bytes([manufacturer_data[18], manufacturer_data[19]]) as f64;
    let gravity =
        u16::from_be_bytes([manufacturer_data[20], manufacturer_data[21]]) as f64 / 1000.0;

    Some(TiltReading::new(color, temperature_f, gravity, None, String::new()))
}

/// Format a `SystemTime` as an RFC 3339 / ISO 8601 UTC timestamp string.
/// This avoids needing the `chrono` crate on ESP32.
pub fn format_timestamp(t: SystemTime) -> String {
    let dur = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    // Break epoch seconds into date/time components (UTC)
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;

    // Civil date from days since 1970-01-01 (Rata Die algorithm)
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // day of era
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hour, minute, second
    )
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

    #[test]
    fn all_8_colors_uuid_round_trip() {
        for color in TiltColor::all() {
            let uuid = color.uuid_bytes();
            let recovered = TiltColor::from_uuid_bytes(uuid);
            assert_eq!(recovered, Some(*color));
        }
    }

    #[test]
    fn unknown_uuid_returns_none() {
        let unknown = [0u8; 16];
        assert_eq!(TiltColor::from_uuid_bytes(&unknown), None);
    }

    #[test]
    fn tilt_color_has_8_variants() {
        assert_eq!(TiltColor::all().len(), 8);
    }

    #[test]
    fn red_uuid_matches_shared_crate() {
        assert_eq!(
            *TiltColor::Red.uuid_bytes(),
            [0xA4, 0x95, 0xBB, 0x10, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE]
        );
    }

    #[test]
    fn pink_uuid_matches_shared_crate() {
        assert_eq!(
            *TiltColor::Pink.uuid_bytes(),
            [0xA4, 0x95, 0xBB, 0x80, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE]
        );
    }

    #[test]
    fn each_uuid_unique() {
        let uuids: Vec<&[u8; 16]> = TiltColor::all().iter().map(|c| c.uuid_bytes()).collect();
        for (i, a) in uuids.iter().enumerate() {
            for (j, b) in uuids.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn tilt_color_serializes_json() {
        let json = serde_json::to_string(&TiltColor::Red).unwrap();
        assert_eq!(json, "\"Red\"");
    }

    #[test]
    fn tilt_reading_serializes_camel_case() {
        let reading = TiltReading::new(TiltColor::Blue, 72.0, 1.012, Some(-59), "2026-01-01T00:00:00Z".to_string());
        let json = serde_json::to_string(&reading).unwrap();
        assert!(json.contains("\"temperatureF\""));
        assert!(json.contains("\"recordedAt\""));
        assert!(json.contains("\"color\""));
        assert!(json.contains("\"gravity\""));
    }

    #[test]
    fn parse_valid_red_tilt() {
        let data = make_ibeacon_data(TILT_UUID_RED, 68, 1016, -59);
        let reading = parse_ibeacon(&data).unwrap();
        assert_eq!(reading.color, TiltColor::Red);
        assert!((reading.temperature_f - 68.0).abs() < f64::EPSILON);
        assert!((reading.gravity - 1.016).abs() < 0.0001);
    }

    #[test]
    fn parse_all_8_tilt_colors() {
        for color in TiltColor::all() {
            let data = make_ibeacon_data(*color.uuid_bytes(), 72, 1050, -60);
            let reading = parse_ibeacon(&data).unwrap();
            assert_eq!(reading.color, *color);
        }
    }

    #[test]
    fn parse_unknown_uuid_returns_none() {
        let unknown = [0u8; 16];
        let data = make_ibeacon_data(unknown, 70, 1000, 0);
        assert!(parse_ibeacon(&data).is_none());
    }

    #[test]
    fn parse_too_short_data_returns_none() {
        let data = vec![0x02, 0x15, 0x00];
        assert!(parse_ibeacon(&data).is_none());
    }

    #[test]
    fn parse_wrong_type_returns_none() {
        let mut data = make_ibeacon_data(TILT_UUID_RED, 70, 1000, 0);
        data[0] = 0xFF;
        assert!(parse_ibeacon(&data).is_none());
    }

    #[test]
    fn format_timestamp_unix_epoch() {
        let t = SystemTime::UNIX_EPOCH;
        assert_eq!(format_timestamp(t), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn format_timestamp_known_date() {
        // 2026-03-19T00:00:00Z = 1773878400 seconds since epoch
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1773878400);
        assert_eq!(format_timestamp(t), "2026-03-19T00:00:00Z");
    }

    #[test]
    fn format_timestamp_with_time() {
        // 2025-06-15T15:10:45Z = 1750000245 seconds since epoch
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1750000245);
        assert_eq!(format_timestamp(t), "2025-06-15T15:10:45Z");
    }
}
