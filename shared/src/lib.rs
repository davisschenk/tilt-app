use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TILT_UUID_RED: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x10, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
]);
const TILT_UUID_GREEN: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x20, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
]);
const TILT_UUID_BLACK: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x30, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
]);
const TILT_UUID_PURPLE: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x40, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
]);
const TILT_UUID_ORANGE: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x50, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
]);
const TILT_UUID_BLUE: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x60, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
]);
const TILT_UUID_YELLOW: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x70, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
]);
const TILT_UUID_PINK: Uuid = Uuid::from_bytes([
    0xA4, 0x95, 0xBB, 0x80, 0xC5, 0xB1, 0x4B, 0x44, 0xB5, 0x12, 0x13, 0x70, 0xF0, 0x2D, 0x74, 0xDE,
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

    pub fn parse(s: &str) -> Option<TiltColor> {
        match s {
            "Red" => Some(TiltColor::Red),
            "Green" => Some(TiltColor::Green),
            "Black" => Some(TiltColor::Black),
            "Purple" => Some(TiltColor::Purple),
            "Orange" => Some(TiltColor::Orange),
            "Blue" => Some(TiltColor::Blue),
            "Yellow" => Some(TiltColor::Yellow),
            "Pink" => Some(TiltColor::Pink),
            _ => None,
        }
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
    pub batch_size_gallons: Option<f64>,
    pub yeast_nitrogen_requirement: Option<String>,
    pub pitch_time: Option<DateTime<Utc>>,
    pub nutrient_protocol: Option<String>,
    pub yeast_strain: Option<String>,
    pub nutrient_alert_target_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBrew {
    pub name: Option<String>,
    pub style: Option<String>,
    pub og: Option<f64>,
    pub fg: Option<f64>,
    pub target_fg: Option<f64>,
    pub status: Option<BrewStatus>,
    pub notes: Option<String>,
    pub end_date: Option<DateTime<Utc>>,
    pub batch_size_gallons: Option<f64>,
    pub yeast_nitrogen_requirement: Option<String>,
    pub pitch_time: Option<DateTime<Utc>>,
    pub nutrient_protocol: Option<String>,
    pub yeast_strain: Option<String>,
    pub nutrient_alert_target_id: Option<Uuid>,
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
    pub status: BrewStatus,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub hydrometer_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub latest_reading: Option<TiltReading>,
    pub live_abv: Option<f64>,
    pub apparent_attenuation: Option<f64>,
    pub final_abv: Option<f64>,
    pub batch_size_gallons: Option<f64>,
    pub yeast_nitrogen_requirement: Option<String>,
    pub pitch_time: Option<DateTime<Utc>>,
    pub nutrient_protocol: Option<String>,
    pub yeast_strain: Option<String>,
    pub nutrient_alert_target_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateHydrometer {
    pub color: TiltColor,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateHydrometer {
    pub name: Option<String>,
    pub temp_offset_f: Option<f64>,
    pub gravity_offset: Option<f64>,
    pub is_disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HydrometerResponse {
    pub id: Uuid,
    pub color: TiltColor,
    pub name: Option<String>,
    pub temp_offset_f: f64,
    pub gravity_offset: f64,
    pub is_disabled: bool,
    pub created_at: DateTime<Utc>,
    pub latest_reading: Option<TiltReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingResponse {
    pub id: Uuid,
    pub brew_id: Option<Uuid>,
    pub hydrometer_id: Uuid,
    pub color: TiltColor,
    pub temperature_f: f64,
    pub gravity: f64,
    pub rssi: Option<i16>,
    pub recorded_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingsQuery {
    pub brew_id: Option<Uuid>,
    pub hydrometer_id: Option<Uuid>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<u64>,
}

impl ReadingsQuery {
    pub fn limit_or_default(&self) -> u64 {
        self.limit.unwrap_or(10_000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookFormat {
    GenericJson,
    Discord,
    Slack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertMetric {
    Gravity,
    TemperatureF,
    GravityPlateau,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertOperator {
    Lte,
    Gte,
    Lt,
    Gt,
    Eq,
    Plateau,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertTargetResponse {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub format: WebhookFormat,
    pub secret_header: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlertTarget {
    pub name: String,
    pub url: String,
    pub format: WebhookFormat,
    pub secret_header: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertTarget {
    pub name: Option<String>,
    pub url: Option<String>,
    pub format: Option<WebhookFormat>,
    pub secret_header: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleResponse {
    pub id: Uuid,
    pub name: String,
    pub brew_id: Option<Uuid>,
    pub hydrometer_id: Option<Uuid>,
    pub metric: AlertMetric,
    pub operator: AlertOperator,
    pub threshold: f64,
    pub alert_target_id: Uuid,
    pub enabled: bool,
    pub cooldown_minutes: i32,
    pub window_hours: i32,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlertRule {
    pub name: String,
    pub metric: AlertMetric,
    pub operator: AlertOperator,
    pub threshold: f64,
    pub alert_target_id: Uuid,
    pub brew_id: Option<Uuid>,
    pub hydrometer_id: Option<Uuid>,
    pub cooldown_minutes: Option<i32>,
    pub window_hours: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertRule {
    pub name: Option<String>,
    pub metric: Option<AlertMetric>,
    pub operator: Option<AlertOperator>,
    pub threshold: Option<f64>,
    pub alert_target_id: Option<Uuid>,
    pub brew_id: Option<Uuid>,
    pub hydrometer_id: Option<Uuid>,
    pub cooldown_minutes: Option<i32>,
    pub window_hours: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingGap {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub duration_minutes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrewAnalytics {
    pub current_gravity: Option<f64>,
    pub current_temp_f: Option<f64>,
    pub last_reading_at: Option<DateTime<Utc>>,
    pub live_abv: Option<f64>,
    pub apparent_attenuation: Option<f64>,
    pub predicted_fg_date: Option<DateTime<Utc>>,
    pub hours_remaining: Option<f64>,
    pub gaps: Vec<ReadingGap>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrewEventType {
    YeastPitch,
    DryHop,
    FermentationComplete,
    DiacetylRest,
    ColdCrash,
    FiningAddition,
    Transfer,
    Packaged,
    GravitySample,
    TastingNote,
    TemperatureChange,
    Note,
    NutrientAddition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventAttachmentResponse {
    pub id: Uuid,
    pub event_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrewEventResponse {
    pub id: Uuid,
    pub brew_id: Uuid,
    pub event_type: BrewEventType,
    pub label: String,
    pub notes: Option<String>,
    pub gravity_at_event: Option<f64>,
    pub temp_at_event: Option<f64>,
    pub event_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub attachments: Vec<EventAttachmentResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrewEvent {
    pub brew_id: Uuid,
    pub event_type: BrewEventType,
    pub label: String,
    pub notes: Option<String>,
    pub gravity_at_event: Option<f64>,
    pub temp_at_event: Option<f64>,
    pub event_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBrewEvent {
    pub label: Option<String>,
    pub notes: Option<String>,
    pub gravity_at_event: Option<f64>,
    pub temp_at_event: Option<f64>,
    pub event_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NutrientScheduleResponse {
    pub protocol: String,
    pub additions: Vec<NutrientAddition>,
    pub total_yan_required_ppm: f64,
    pub nutrient_totals: std::collections::HashMap<String, f64>,
    pub batch_size_gallons: f64,
    pub batch_size_liters: f64,
    pub og: f64,
    pub target_fg: f64,
    pub nitrogen_requirement: String,
    pub pitch_time: DateTime<Utc>,
    pub resolved_from_strain: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientProduct {
    FermaidO,
    FermaidK,
    Dap,
    GoFerm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientProtocol {
    Tosna2,
    Tosna3,
    AdvancedSna,
}

impl NutrientProtocol {
    pub fn from_protocol_str(s: &str) -> Self {
        match s {
            "tosna_3" => Self::Tosna3,
            "advanced_sna" => Self::AdvancedSna,
            _ => Self::Tosna2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientTrigger {
    GravityThreshold,
    TimeElapsed,
    AtPitch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NutrientAddition {
    pub addition_number: u8,
    pub product: NutrientProduct,
    pub amount_grams: f64,
    pub primary_trigger: NutrientTrigger,
    pub gravity_threshold: Option<f64>,
    pub fallback_hours: Option<u32>,
    pub due_at: Option<DateTime<Utc>>,
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
        let batch = CreateReadingsBatch::new(vec![TiltReading::new(
            TiltColor::Yellow,
            75.0,
            1.060,
            None,
            now,
        )]);
        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: CreateReadingsBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized.readings()[0].color, TiltColor::Yellow);
    }

    #[test]
    fn brew_status_serialize_json() {
        assert_eq!(
            serde_json::to_string(&BrewStatus::Active).unwrap(),
            "\"Active\""
        );
        assert_eq!(
            serde_json::to_string(&BrewStatus::Completed).unwrap(),
            "\"Completed\""
        );
        assert_eq!(
            serde_json::to_string(&BrewStatus::Archived).unwrap(),
            "\"Archived\""
        );
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
            batch_size_gallons: None,
            yeast_nitrogen_requirement: None,
            pitch_time: None,
            nutrient_protocol: None,
            yeast_strain: None,
            nutrient_alert_target_id: None,
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
            status: BrewStatus::Active,
            start_date: Some(now),
            end_date: None,
            notes: None,
            hydrometer_id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            latest_reading: None,
            live_abv: Some(4.2),
            apparent_attenuation: Some(75.0),
            final_abv: None,
            batch_size_gallons: None,
            yeast_nitrogen_requirement: None,
            pitch_time: None,
            nutrient_protocol: None,
            yeast_strain: None,
            nutrient_alert_target_id: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"latestReading\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"liveAbv\""));
        assert!(json.contains("\"apparentAttenuation\""));
        let deserialized: BrewResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Pale Ale");
        assert_eq!(deserialized.status, BrewStatus::Active);
        assert!((deserialized.live_abv.unwrap() - 4.2).abs() < f64::EPSILON);
    }

    #[test]
    fn nutrient_fields_serde_round_trip() {
        let now = Utc::now();
        let json = format!(
            r#"{{"name":"Mead","hydrometerId":"a495bb10-c5b1-4b44-b512-1370f02d74de","batchSizeGallons":5.0,"yeastNitrogenRequirement":"low","pitchTime":"{}","nutrientProtocol":"tosna_2"}}"#,
            now.to_rfc3339()
        );
        let brew: CreateBrew = serde_json::from_str(&json).unwrap();
        assert!((brew.batch_size_gallons.unwrap() - 5.0).abs() < f64::EPSILON);
        assert_eq!(brew.yeast_nitrogen_requirement.as_deref(), Some("low"));
        assert!(brew.pitch_time.is_some());
        assert_eq!(brew.nutrient_protocol.as_deref(), Some("tosna_2"));
        let out = serde_json::to_string(&brew).unwrap();
        assert!(out.contains("\"batchSizeGallons\""));
        assert!(out.contains("\"yeastNitrogenRequirement\""));
        assert!(out.contains("\"pitchTime\""));
        assert!(out.contains("\"nutrientProtocol\""));
    }

    #[test]
    fn nutrient_fields_absent_are_null_in_response() {
        let now = Utc::now();
        let resp = BrewResponse {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            style: None,
            og: None,
            fg: None,
            target_fg: None,
            status: BrewStatus::Active,
            start_date: None,
            end_date: None,
            notes: None,
            hydrometer_id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            latest_reading: None,
            live_abv: None,
            apparent_attenuation: None,
            final_abv: None,
            batch_size_gallons: Some(1.0),
            yeast_nitrogen_requirement: Some("medium".to_string()),
            yeast_strain: None,
            pitch_time: Some(now),
            nutrient_protocol: Some("tosna_3".to_string()),
            nutrient_alert_target_id: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: BrewResponse = serde_json::from_str(&json).unwrap();
        assert!((back.batch_size_gallons.unwrap() - 1.0).abs() < f64::EPSILON);
        assert_eq!(back.yeast_nitrogen_requirement.as_deref(), Some("medium"));
        assert!(back.pitch_time.is_some());
        assert_eq!(back.nutrient_protocol.as_deref(), Some("tosna_3"));
    }

    #[test]
    fn create_hydrometer_required_and_optional() {
        let json = r#"{"color":"Red"}"#;
        let hydro: CreateHydrometer = serde_json::from_str(json).unwrap();
        assert_eq!(hydro.color, TiltColor::Red);
        assert!(hydro.name.is_none());
    }

    #[test]
    fn create_hydrometer_with_name() {
        let hydro = CreateHydrometer {
            color: TiltColor::Blue,
            name: Some("My Blue Tilt".to_string()),
        };
        let json = serde_json::to_string(&hydro).unwrap();
        let deserialized: CreateHydrometer = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.color, TiltColor::Blue);
        assert_eq!(deserialized.name.unwrap(), "My Blue Tilt");
    }

    #[test]
    fn update_hydrometer_all_fields_optional() {
        let update: UpdateHydrometer = serde_json::from_str("{}").unwrap();
        assert!(update.name.is_none());
        assert!(update.temp_offset_f.is_none());
        assert!(update.gravity_offset.is_none());
    }

    #[test]
    fn update_hydrometer_camel_case_fields() {
        let json = r#"{"tempOffsetF":1.5,"gravityOffset":-0.002}"#;
        let update: UpdateHydrometer = serde_json::from_str(json).unwrap();
        assert!((update.temp_offset_f.unwrap() - 1.5).abs() < f64::EPSILON);
        assert!((update.gravity_offset.unwrap() - (-0.002)).abs() < f64::EPSILON);
    }

    #[test]
    fn hydrometer_response_serde_round_trip() {
        let now = Utc::now();
        let resp = HydrometerResponse {
            id: Uuid::new_v4(),
            color: TiltColor::Green,
            name: Some("Fermenter 1".to_string()),
            temp_offset_f: 0.0,
            gravity_offset: 0.0,
            is_disabled: false,
            created_at: now,
            latest_reading: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"tempOffsetF\""));
        assert!(json.contains("\"gravityOffset\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"isDisabled\""));
        let deserialized: HydrometerResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.color, TiltColor::Green);
        assert_eq!(deserialized.name.unwrap(), "Fermenter 1");
        assert!(!deserialized.is_disabled);
    }

    #[test]
    fn reading_response_serde_round_trip() {
        let now = Utc::now();
        let resp = ReadingResponse {
            id: Uuid::new_v4(),
            brew_id: Some(Uuid::new_v4()),
            hydrometer_id: Uuid::new_v4(),
            color: TiltColor::Orange,
            temperature_f: 68.0,
            gravity: 1.050,
            rssi: Some(-59),
            recorded_at: now,
            created_at: now,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"brewId\""));
        assert!(json.contains("\"hydrometerId\""));
        assert!(json.contains("\"temperatureF\""));
        assert!(json.contains("\"recordedAt\""));
        let deserialized: ReadingResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.color, TiltColor::Orange);
        assert!((deserialized.gravity - 1.050).abs() < f64::EPSILON);
    }

    #[test]
    fn reading_response_optional_fields() {
        let now = Utc::now();
        let resp = ReadingResponse {
            id: Uuid::new_v4(),
            brew_id: None,
            hydrometer_id: Uuid::new_v4(),
            color: TiltColor::Black,
            temperature_f: 72.0,
            gravity: 1.030,
            rssi: None,
            recorded_at: now,
            created_at: now,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: ReadingResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.brew_id.is_none());
        assert!(deserialized.rssi.is_none());
    }

    #[test]
    fn readings_query_all_fields_optional() {
        let query: ReadingsQuery = serde_json::from_str("{}").unwrap();
        assert!(query.brew_id.is_none());
        assert!(query.hydrometer_id.is_none());
        assert!(query.since.is_none());
        assert!(query.until.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn readings_query_limit_default_10000() {
        let query: ReadingsQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.limit_or_default(), 10_000);
    }

    #[test]
    fn readings_query_limit_custom() {
        let json = r#"{"limit":50}"#;
        let query: ReadingsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit_or_default(), 50);
    }

    #[test]
    fn webhook_format_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&WebhookFormat::GenericJson).unwrap(),
            "\"generic_json\""
        );
        assert_eq!(
            serde_json::to_string(&WebhookFormat::Discord).unwrap(),
            "\"discord\""
        );
        assert_eq!(
            serde_json::to_string(&WebhookFormat::Slack).unwrap(),
            "\"slack\""
        );
    }

    #[test]
    fn webhook_format_deserializes() {
        let fmt: WebhookFormat = serde_json::from_str("\"generic_json\"").unwrap();
        assert_eq!(fmt, WebhookFormat::GenericJson);
        let fmt: WebhookFormat = serde_json::from_str("\"discord\"").unwrap();
        assert_eq!(fmt, WebhookFormat::Discord);
    }

    #[test]
    fn alert_metric_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&AlertMetric::Gravity).unwrap(),
            "\"gravity\""
        );
        assert_eq!(
            serde_json::to_string(&AlertMetric::TemperatureF).unwrap(),
            "\"temperature_f\""
        );
    }

    #[test]
    fn alert_metric_deserializes() {
        let m: AlertMetric = serde_json::from_str("\"gravity\"").unwrap();
        assert_eq!(m, AlertMetric::Gravity);
        let m: AlertMetric = serde_json::from_str("\"temperature_f\"").unwrap();
        assert_eq!(m, AlertMetric::TemperatureF);
    }

    #[test]
    fn alert_operator_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&AlertOperator::Lte).unwrap(),
            "\"lte\""
        );
        assert_eq!(
            serde_json::to_string(&AlertOperator::Gte).unwrap(),
            "\"gte\""
        );
        assert_eq!(serde_json::to_string(&AlertOperator::Lt).unwrap(), "\"lt\"");
        assert_eq!(serde_json::to_string(&AlertOperator::Gt).unwrap(), "\"gt\"");
        assert_eq!(serde_json::to_string(&AlertOperator::Eq).unwrap(), "\"eq\"");
    }

    #[test]
    fn alert_operator_deserializes() {
        let op: AlertOperator = serde_json::from_str("\"lte\"").unwrap();
        assert_eq!(op, AlertOperator::Lte);
        let op: AlertOperator = serde_json::from_str("\"eq\"").unwrap();
        assert_eq!(op, AlertOperator::Eq);
    }

    #[test]
    fn create_alert_target_required_and_optional() {
        let json = r#"{"name":"Discord Alerts","url":"https://discord.com/api/webhooks/123","format":"discord"}"#;
        let target: CreateAlertTarget = serde_json::from_str(json).unwrap();
        assert_eq!(target.name, "Discord Alerts");
        assert_eq!(target.format, WebhookFormat::Discord);
        assert!(target.secret_header.is_none());
        assert!(target.enabled.is_none());
    }

    #[test]
    fn create_alert_target_with_all_fields() {
        let target = CreateAlertTarget {
            name: "Slack Hook".to_string(),
            url: "https://hooks.slack.com/services/T00/B00/xxx".to_string(),
            format: WebhookFormat::Slack,
            secret_header: Some("Bearer tok123".to_string()),
            enabled: Some(false),
        };
        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("\"secretHeader\""));
        let deserialized: CreateAlertTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.format, WebhookFormat::Slack);
        assert_eq!(deserialized.secret_header.unwrap(), "Bearer tok123");
        assert_eq!(deserialized.enabled, Some(false));
    }

    #[test]
    fn update_alert_target_all_optional() {
        let update: UpdateAlertTarget = serde_json::from_str("{}").unwrap();
        assert!(update.name.is_none());
        assert!(update.url.is_none());
        assert!(update.format.is_none());
        assert!(update.secret_header.is_none());
        assert!(update.enabled.is_none());
    }

    #[test]
    fn alert_target_response_serde_round_trip() {
        let now = Utc::now();
        let resp = AlertTargetResponse {
            id: Uuid::new_v4(),
            name: "My Webhook".to_string(),
            url: "https://example.com/hook".to_string(),
            format: WebhookFormat::GenericJson,
            secret_header: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"updatedAt\""));
        let deserialized: AlertTargetResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "My Webhook");
        assert_eq!(deserialized.format, WebhookFormat::GenericJson);
    }

    #[test]
    fn create_alert_rule_required_and_optional() {
        let target_id = Uuid::new_v4();
        let json = format!(
            r#"{{"name":"Low Gravity","metric":"gravity","operator":"lte","threshold":1.010,"alertTargetId":"{}"}}"#,
            target_id
        );
        let rule: CreateAlertRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule.name, "Low Gravity");
        assert_eq!(rule.metric, AlertMetric::Gravity);
        assert_eq!(rule.operator, AlertOperator::Lte);
        assert!((rule.threshold - 1.010).abs() < f64::EPSILON);
        assert_eq!(rule.alert_target_id, target_id);
        assert!(rule.brew_id.is_none());
        assert!(rule.hydrometer_id.is_none());
        assert!(rule.cooldown_minutes.is_none());
        assert!(rule.enabled.is_none());
    }

    #[test]
    fn create_alert_rule_with_all_fields() {
        let rule = CreateAlertRule {
            name: "High Temp".to_string(),
            metric: AlertMetric::TemperatureF,
            operator: AlertOperator::Gte,
            threshold: 80.0,
            alert_target_id: Uuid::new_v4(),
            brew_id: Some(Uuid::new_v4()),
            hydrometer_id: Some(Uuid::new_v4()),
            cooldown_minutes: Some(30),
            window_hours: Some(24),
            enabled: Some(true),
        };
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("\"alertTargetId\""));
        assert!(json.contains("\"cooldownMinutes\""));
        assert!(json.contains("\"brewId\""));
        assert!(json.contains("\"hydrometerId\""));
        let deserialized: CreateAlertRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metric, AlertMetric::TemperatureF);
        assert_eq!(deserialized.cooldown_minutes, Some(30));
    }

    #[test]
    fn update_alert_rule_all_optional() {
        let update: UpdateAlertRule = serde_json::from_str("{}").unwrap();
        assert!(update.name.is_none());
        assert!(update.metric.is_none());
        assert!(update.operator.is_none());
        assert!(update.threshold.is_none());
        assert!(update.alert_target_id.is_none());
        assert!(update.brew_id.is_none());
        assert!(update.hydrometer_id.is_none());
        assert!(update.cooldown_minutes.is_none());
        assert!(update.enabled.is_none());
    }

    #[test]
    fn brew_event_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&BrewEventType::YeastPitch).unwrap(),
            "\"yeast_pitch\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::DryHop).unwrap(),
            "\"dry_hop\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::FermentationComplete).unwrap(),
            "\"fermentation_complete\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::DiacetylRest).unwrap(),
            "\"diacetyl_rest\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::ColdCrash).unwrap(),
            "\"cold_crash\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::FiningAddition).unwrap(),
            "\"fining_addition\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::Transfer).unwrap(),
            "\"transfer\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::Packaged).unwrap(),
            "\"packaged\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::GravitySample).unwrap(),
            "\"gravity_sample\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::TastingNote).unwrap(),
            "\"tasting_note\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::TemperatureChange).unwrap(),
            "\"temperature_change\""
        );
        assert_eq!(
            serde_json::to_string(&BrewEventType::Note).unwrap(),
            "\"note\""
        );
    }

    #[test]
    fn brew_event_type_deserializes_all_variants() {
        let variants = [
            ("\"yeast_pitch\"", BrewEventType::YeastPitch),
            ("\"dry_hop\"", BrewEventType::DryHop),
            (
                "\"fermentation_complete\"",
                BrewEventType::FermentationComplete,
            ),
            ("\"diacetyl_rest\"", BrewEventType::DiacetylRest),
            ("\"cold_crash\"", BrewEventType::ColdCrash),
            ("\"fining_addition\"", BrewEventType::FiningAddition),
            ("\"transfer\"", BrewEventType::Transfer),
            ("\"packaged\"", BrewEventType::Packaged),
            ("\"gravity_sample\"", BrewEventType::GravitySample),
            ("\"tasting_note\"", BrewEventType::TastingNote),
            ("\"temperature_change\"", BrewEventType::TemperatureChange),
            ("\"note\"", BrewEventType::Note),
        ];
        for (json, expected) in &variants {
            let got: BrewEventType = serde_json::from_str(json).unwrap();
            assert_eq!(got, *expected, "Failed to deserialize {json}");
        }
    }

    #[test]
    fn create_brew_event_required_fields() {
        let now = Utc::now();
        let event = CreateBrewEvent {
            brew_id: Uuid::new_v4(),
            event_type: BrewEventType::DryHop,
            label: "Citra 2oz".to_string(),
            notes: None,
            gravity_at_event: None,
            temp_at_event: None,
            event_time: now,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"brewId\""));
        assert!(json.contains("\"eventType\""));
        assert!(json.contains("\"dry_hop\""));
        assert!(json.contains("\"eventTime\""));
        let deserialized: CreateBrewEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, BrewEventType::DryHop);
        assert_eq!(deserialized.label, "Citra 2oz");
        assert!(deserialized.notes.is_none());
        assert!(deserialized.gravity_at_event.is_none());
    }

    #[test]
    fn create_brew_event_with_all_fields() {
        let now = Utc::now();
        let event = CreateBrewEvent {
            brew_id: Uuid::new_v4(),
            event_type: BrewEventType::GravitySample,
            label: "Day 3 sample".to_string(),
            notes: Some("Tastes clean".to_string()),
            gravity_at_event: Some(1.040),
            temp_at_event: Some(68.5),
            event_time: now,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"gravityAtEvent\""));
        assert!(json.contains("\"tempAtEvent\""));
        let deserialized: CreateBrewEvent = serde_json::from_str(&json).unwrap();
        assert!((deserialized.gravity_at_event.unwrap() - 1.040).abs() < f64::EPSILON);
        assert!((deserialized.temp_at_event.unwrap() - 68.5).abs() < f64::EPSILON);
    }

    #[test]
    fn update_brew_event_all_optional() {
        let update: UpdateBrewEvent = serde_json::from_str("{}").unwrap();
        assert!(update.label.is_none());
        assert!(update.notes.is_none());
        assert!(update.gravity_at_event.is_none());
        assert!(update.temp_at_event.is_none());
        assert!(update.event_time.is_none());
    }

    #[test]
    fn brew_event_response_serde_round_trip() {
        let now = Utc::now();
        let resp = BrewEventResponse {
            id: Uuid::new_v4(),
            brew_id: Uuid::new_v4(),
            event_type: BrewEventType::YeastPitch,
            label: "US-05 pitched".to_string(),
            notes: Some("Rehydrated dry yeast".to_string()),
            gravity_at_event: Some(1.055),
            temp_at_event: Some(65.0),
            event_time: now,
            created_at: now,
            attachments: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"brewId\""));
        assert!(json.contains("\"eventType\""));
        assert!(json.contains("\"yeast_pitch\""));
        assert!(json.contains("\"gravityAtEvent\""));
        assert!(json.contains("\"createdAt\""));
        assert!(json.contains("\"attachments\""));
        let deserialized: BrewEventResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, BrewEventType::YeastPitch);
        assert_eq!(deserialized.label, "US-05 pitched");
        assert!((deserialized.gravity_at_event.unwrap() - 1.055).abs() < f64::EPSILON);
        assert!(deserialized.attachments.is_empty());
    }

    #[test]
    fn alert_rule_response_serde_round_trip() {
        let now = Utc::now();
        let resp = AlertRuleResponse {
            id: Uuid::new_v4(),
            name: "FG Reached".to_string(),
            brew_id: Some(Uuid::new_v4()),
            hydrometer_id: None,
            metric: AlertMetric::Gravity,
            operator: AlertOperator::Lte,
            threshold: 1.012,
            alert_target_id: Uuid::new_v4(),
            enabled: true,
            cooldown_minutes: 60,
            window_hours: 24,
            last_triggered_at: None,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"lastTriggeredAt\""));
        assert!(json.contains("\"cooldownMinutes\""));
        assert!(json.contains("\"windowHours\""));
        let deserialized: AlertRuleResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "FG Reached");
        assert_eq!(deserialized.operator, AlertOperator::Lte);
        assert!(deserialized.last_triggered_at.is_none());
    }
}
