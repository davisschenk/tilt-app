use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
            TiltColor::Red => Uuid::parse_str("A495BB10-C5B1-4B44-B512-1370F02D74DE").unwrap(),
            TiltColor::Green => Uuid::parse_str("A495BB20-C5B1-4B44-B512-1370F02D74DE").unwrap(),
            TiltColor::Black => Uuid::parse_str("A495BB30-C5B1-4B44-B512-1370F02D74DE").unwrap(),
            TiltColor::Purple => Uuid::parse_str("A495BB40-C5B1-4B44-B512-1370F02D74DE").unwrap(),
            TiltColor::Orange => Uuid::parse_str("A495BB50-C5B1-4B44-B512-1370F02D74DE").unwrap(),
            TiltColor::Blue => Uuid::parse_str("A495BB60-C5B1-4B44-B512-1370F02D74DE").unwrap(),
            TiltColor::Yellow => Uuid::parse_str("A495BB70-C5B1-4B44-B512-1370F02D74DE").unwrap(),
            TiltColor::Pink => Uuid::parse_str("A495BB80-C5B1-4B44-B512-1370F02D74DE").unwrap(),
        }
    }

    pub fn from_uuid(uuid: &Uuid) -> Option<TiltColor> {
        let s = uuid.to_string().to_uppercase();
        match s.as_str() {
            "A495BB10-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Red),
            "A495BB20-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Green),
            "A495BB30-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Black),
            "A495BB40-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Purple),
            "A495BB50-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Orange),
            "A495BB60-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Blue),
            "A495BB70-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Yellow),
            "A495BB80-C5B1-4B44-B512-1370F02D74DE" => Some(TiltColor::Pink),
            _ => None,
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
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrewStatus {
    Active,
    Completed,
    Archived,
}
