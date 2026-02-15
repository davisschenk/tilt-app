use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TILT_UUID_RED: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x10, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);
const TILT_UUID_GREEN: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x20, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);
const TILT_UUID_BLACK: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x30, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);
const TILT_UUID_PURPLE: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x40, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);
const TILT_UUID_ORANGE: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x50, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);
const TILT_UUID_BLUE: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x60, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);
const TILT_UUID_YELLOW: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x70, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);
const TILT_UUID_PINK: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x80, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74,
    0xDE,
]);

const ALL_TILT_UUIDS: [(Uuid, TiltColor); 8] = [
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
    pub fn uuid(&self) -> Uuid {
        match self {
            TiltColor::Red => TILT_UUID_RED,
            TiltColor::Green => TILT_UUID_GREEN,
            TiltColor::Black => TILT_UUID_BLACK,
            TiltColor::Purple => TILT_UUID_PURPLE,
            TiltColor::Orange => TILT_UUID_ORANGE,
            TiltColor::Blue => TILT_UUID_BLUE,
            TiltColor::Yellow => TILT_UUID_YELLOW,
            TiltColor::Pink => TILT_UUID_PINK,
        }
    }

    pub fn from_uuid(uuid: &Uuid) -> Option<TiltColor> {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiltReading {
    pub color: TiltColor,
    pub temperature_f: f64,
    pub gravity: f64,
    pub rssi: Option<i16>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrewStatus {
    Active,
    Completed,
    Archived,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilt_color_uuid_round_trip_all_8() {
        for color in TiltColor::all() {
            let uuid = color.uuid();
            let recovered = TiltColor::from_uuid(&uuid);
            assert_eq!(recovered, Some(*color), "Round-trip failed for {:?}", color);
        }
    }

    #[test]
    fn tilt_color_has_8_variants() {
        assert_eq!(TiltColor::all().len(), 8);
    }

    #[test]
    fn tilt_color_red_uuid_correct() {
        let expected = Uuid::parse_str("A495BB10-C5B1-4B44-B512-1370F02D74DE").unwrap();
        assert_eq!(TiltColor::Red.uuid(), expected);
    }

    #[test]
    fn tilt_color_green_uuid_correct() {
        let expected = Uuid::parse_str("A495BB20-C5B1-4B44-B512-1370F02D74DE").unwrap();
        assert_eq!(TiltColor::Green.uuid(), expected);
    }

    #[test]
    fn tilt_color_pink_uuid_correct() {
        let expected = Uuid::parse_str("A495BB80-C5B1-4B44-B512-1370F02D74DE").unwrap();
        assert_eq!(TiltColor::Pink.uuid(), expected);
    }

    #[test]
    fn tilt_color_from_unknown_uuid_returns_none() {
        let unknown = Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        assert_eq!(TiltColor::from_uuid(&unknown), None);
    }

    #[test]
    fn tilt_color_each_uuid_unique() {
        let uuids: Vec<Uuid> = TiltColor::all().iter().map(|c| c.uuid()).collect();
        for (i, a) in uuids.iter().enumerate() {
            for (j, b) in uuids.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "UUIDs for colors at index {} and {} collide", i, j);
                }
            }
        }
    }

    #[test]
    fn tilt_color_serialize_json() {
        let json = serde_json::to_string(&TiltColor::Red).unwrap();
        assert_eq!(json, "\"Red\"");
    }

    #[test]
    fn tilt_color_deserialize_json() {
        let color: TiltColor = serde_json::from_str("\"Purple\"").unwrap();
        assert_eq!(color, TiltColor::Purple);
    }
}
