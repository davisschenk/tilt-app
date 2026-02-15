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

impl TiltReading {
    pub fn new(
        color: TiltColor,
        temperature_f: f64,
        gravity: f64,
        rssi: Option<i16>,
        recorded_at: DateTime<Utc>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReadingsBatch(pub Vec<TiltReading>);

impl CreateReadingsBatch {
    pub fn new(readings: Vec<TiltReading>) -> Self {
        Self(readings)
    }

    pub fn readings(&self) -> &[TiltReading] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrewStatus {
    Active,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrew {
    pub name: String,
    pub hydrometer_id: Uuid,
    pub style: Option<String>,
    pub og: Option<f64>,
    pub target_fg: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBrew {
    pub name: Option<String>,
    pub style: Option<String>,
    pub og: Option<f64>,
    pub fg: Option<f64>,
    pub target_fg: Option<f64>,
    pub abv: Option<f64>,
    pub status: Option<BrewStatus>,
    pub notes: Option<String>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrewResponse {
    pub id: Uuid,
    pub name: String,
    pub style: Option<String>,
    pub og: Option<f64>,
    pub fg: Option<f64>,
    pub target_fg: Option<f64>,
    pub abv: Option<f64>,
    pub status: BrewStatus,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub hydrometer_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub latest_reading: Option<TiltReading>,
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

    #[test]
    fn tilt_reading_new_constructs_valid_instance() {
        let now = Utc::now();
        let reading = TiltReading::new(TiltColor::Red, 68.0, 1.050, Some(-59), now);
        assert_eq!(reading.color, TiltColor::Red);
        assert!((reading.temperature_f - 68.0).abs() < f64::EPSILON);
        assert!((reading.gravity - 1.050).abs() < f64::EPSILON);
        assert_eq!(reading.rssi, Some(-59));
        assert_eq!(reading.recorded_at, now);
    }

    #[test]
    fn tilt_reading_serializes_camel_case() {
        let now = Utc::now();
        let reading = TiltReading::new(TiltColor::Blue, 72.0, 1.012, None, now);
        let json = serde_json::to_string(&reading).unwrap();
        assert!(json.contains("\"temperatureF\""));
        assert!(json.contains("\"recordedAt\""));
        assert!(json.contains("\"color\""));
        assert!(json.contains("\"gravity\""));
    }

    #[test]
    fn tilt_reading_serde_round_trip() {
        let now = Utc::now();
        let reading = TiltReading::new(TiltColor::Green, 65.0, 1.045, Some(-70), now);
        let json = serde_json::to_string(&reading).unwrap();
        let deserialized: TiltReading = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.color, reading.color);
        assert!((deserialized.temperature_f - reading.temperature_f).abs() < f64::EPSILON);
        assert!((deserialized.gravity - reading.gravity).abs() < f64::EPSILON);
        assert_eq!(deserialized.rssi, reading.rssi);
    }

    #[test]
    fn create_readings_batch_wraps_vec() {
        let now = Utc::now();
        let readings = vec![
            TiltReading::new(TiltColor::Red, 68.0, 1.050, None, now),
            TiltReading::new(TiltColor::Blue, 70.0, 1.040, Some(-55), now),
        ];
        let batch = CreateReadingsBatch::new(readings);
        assert_eq!(batch.len(), 2);
        assert!(!batch.is_empty());
        assert_eq!(batch.readings()[0].color, TiltColor::Red);
    }

    #[test]
    fn create_readings_batch_empty() {
        let batch = CreateReadingsBatch::new(vec![]);
        assert_eq!(batch.len(), 0);
        assert!(batch.is_empty());
    }

    #[test]
    fn create_readings_batch_serde_round_trip() {
        let now = Utc::now();
        let batch = CreateReadingsBatch::new(vec![
            TiltReading::new(TiltColor::Yellow, 75.0, 1.060, None, now),
        ]);
        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: CreateReadingsBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized.readings()[0].color, TiltColor::Yellow);
    }

    #[test]
    fn brew_status_serialize_json() {
        assert_eq!(serde_json::to_string(&BrewStatus::Active).unwrap(), "\"Active\"");
        assert_eq!(serde_json::to_string(&BrewStatus::Completed).unwrap(), "\"Completed\"");
        assert_eq!(serde_json::to_string(&BrewStatus::Archived).unwrap(), "\"Archived\"");
    }

    #[test]
    fn brew_status_deserialize_json() {
        let status: BrewStatus = serde_json::from_str("\"Active\"").unwrap();
        assert_eq!(status, BrewStatus::Active);
    }

    #[test]
    fn create_brew_required_and_optional_fields() {
        let json = r#"{"name":"IPA","hydrometerId":"a495bb10-c5b1-4b44-b512-1370f02d74de"}"#;
        let brew: CreateBrew = serde_json::from_str(json).unwrap();
        assert_eq!(brew.name, "IPA");
        assert!(brew.style.is_none());
        assert!(brew.og.is_none());
        assert!(brew.target_fg.is_none());
        assert!(brew.notes.is_none());
    }

    #[test]
    fn create_brew_with_all_fields() {
        let id = Uuid::new_v4();
        let brew = CreateBrew {
            name: "Stout".to_string(),
            hydrometer_id: id,
            style: Some("Imperial Stout".to_string()),
            og: Some(1.090),
            target_fg: Some(1.020),
            notes: Some("Dark and rich".to_string()),
        };
        let json = serde_json::to_string(&brew).unwrap();
        assert!(json.contains("\"hydrometerId\""));
        assert!(json.contains("\"targetFg\""));
        let deserialized: CreateBrew = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Stout");
    }

    #[test]
    fn update_brew_all_fields_optional() {
        let json = "{}";
        let update: UpdateBrew = serde_json::from_str(json).unwrap();
        assert!(update.name.is_none());
        assert!(update.style.is_none());
        assert!(update.og.is_none());
        assert!(update.fg.is_none());
        assert!(update.target_fg.is_none());
        assert!(update.abv.is_none());
        assert!(update.status.is_none());
        assert!(update.notes.is_none());
        assert!(update.end_date.is_none());
    }

    #[test]
    fn brew_response_serde_round_trip() {
        let now = Utc::now();
        let resp = BrewResponse {
            id: Uuid::new_v4(),
            name: "Pale Ale".to_string(),
            style: Some("APA".to_string()),
            og: Some(1.055),
            fg: None,
            target_fg: Some(1.012),
            abv: None,
            status: BrewStatus::Active,
            start_date: Some(now),
            end_date: None,
            notes: None,
            hydrometer_id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            latest_reading: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"latestReading\""));
        assert!(json.contains("\"createdAt\""));
        let deserialized: BrewResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Pale Ale");
        assert_eq!(deserialized.status, BrewStatus::Active);
    }
}
