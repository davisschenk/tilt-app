//! Per-account display units. Readings, thresholds and offsets are always
//! stored canonically (°F and SG); these preferences only change how values
//! are rendered in notifications and reports.

use sea_orm::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::entities::brewer_preferences::{self, ActiveModel, Column, Entity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TemperatureUnit {
    #[default]
    F,
    C,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GravityUnit {
    #[default]
    SG,
    P,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub temperature: TemperatureUnit,
    pub gravity: GravityUnit,
}

/// Specific gravity to degrees Plato (cubic approximation).
pub fn sg_to_plato(sg: f64) -> f64 {
    -616.868 + 1111.14 * sg - 630.272 * sg * sg + 135.997 * sg * sg * sg
}

/// Absolute Fahrenheit temperature to Celsius.
pub fn f_to_c(f: f64) -> f64 {
    (f - 32.0) * 5.0 / 9.0
}

impl Preferences {
    /// An absolute gravity reading, e.g. `1.0100 SG` or `2.56°P`.
    pub fn format_gravity(&self, sg: f64) -> String {
        match self.gravity {
            GravityUnit::SG => format!("{sg:.4} SG"),
            GravityUnit::P => format!("{:.2}°P", sg_to_plato(sg)),
        }
    }

    /// A gravity difference (plateau threshold, calibration offset), measured
    /// relative to SG 1.000.
    pub fn format_gravity_delta(&self, delta: f64) -> String {
        match self.gravity {
            GravityUnit::SG => format!("{delta:.4} SG"),
            GravityUnit::P => format!("{:.2}°P", sg_to_plato(1.0 + delta) - sg_to_plato(1.0)),
        }
    }

    /// An absolute temperature reading.
    pub fn format_temperature(&self, f: f64) -> String {
        match self.temperature {
            TemperatureUnit::F => format!("{f:.1}°F"),
            TemperatureUnit::C => format!("{:.1}°C", f_to_c(f)),
        }
    }

    /// A temperature difference (no 32° offset).
    pub fn format_temperature_delta(&self, delta_f: f64) -> String {
        match self.temperature {
            TemperatureUnit::F => format!("{delta_f:.1}°F"),
            TemperatureUnit::C => format!("{:.1}°C", delta_f * 5.0 / 9.0),
        }
    }

    /// Label for the temperature unit, e.g. `°F`.
    pub fn temperature_label(&self) -> &'static str {
        match self.temperature {
            TemperatureUnit::F => "°F",
            TemperatureUnit::C => "°C",
        }
    }
}

/// The owner's saved preferences, or the defaults (°F, SG) if none are saved
/// or the stored payload no longer parses.
pub async fn get(db: &DatabaseConnection, owner: Uuid) -> Result<Preferences, DbErr> {
    Ok(Entity::find_by_id(owner)
        .one(db)
        .await?
        .and_then(|r| serde_json::from_value(r.payload).ok())
        .unwrap_or_default())
}

/// Like [`get`], but never fails: background notifications fall back to the
/// defaults rather than being dropped.
pub async fn get_or_default(db: &DatabaseConnection, owner: Uuid) -> Preferences {
    get(db, owner).await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "Failed to load preferences; using defaults");
        Preferences::default()
    })
}

/// The raw stored payload, for account export.
pub async fn find_raw(
    db: &DatabaseConnection,
    owner: Uuid,
) -> Result<Option<brewer_preferences::Model>, DbErr> {
    Entity::find_by_id(owner).one(db).await
}

pub async fn set(db: &DatabaseConnection, owner: Uuid, prefs: &Preferences) -> Result<(), DbErr> {
    Entity::insert(ActiveModel {
        owner_id: Set(owner),
        payload: Set(serde_json::json!(prefs)),
    })
    .on_conflict(
        sea_query::OnConflict::column(Column::OwnerId)
            .update_column(Column::Payload)
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const METRIC: Preferences = Preferences {
        temperature: TemperatureUnit::C,
        gravity: GravityUnit::P,
    };

    #[test]
    fn defaults_are_fahrenheit_and_sg() {
        let p = Preferences::default();
        assert_eq!(p.format_gravity(1.01), "1.0100 SG");
        assert_eq!(p.format_gravity_delta(0.002), "0.0020 SG");
        assert_eq!(p.format_temperature(68.0), "68.0°F");
        assert_eq!(p.format_temperature_delta(3.6), "3.6°F");
    }

    #[test]
    fn converts_to_celsius_and_plato() {
        assert_eq!(METRIC.format_temperature(68.0), "20.0°C");
        assert_eq!(METRIC.format_temperature_delta(3.6), "2.0°C");
        assert_eq!(METRIC.format_gravity(1.05), "12.39°P");
        assert_eq!(METRIC.format_gravity_delta(0.002), "0.52°P");
        assert!(sg_to_plato(1.0).abs() < 0.01);
        assert!((f_to_c(212.0) - 100.0).abs() < 1e-9);
    }
}
