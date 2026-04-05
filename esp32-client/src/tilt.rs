//! Tilt iBeacon parsing and color constants.
//!
//! Replicates the Tilt hydrometer types from the `shared` crate since the ESP32
//! client cannot depend on it (different toolchain/target). Contains TiltColor
//! enum, UUID constants, iBeacon parsing, and the TiltReading struct.

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

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

/// Accumulates raw BLE advertisement samples for each Tilt color during a scan window.
///
/// Multiple advertisements from the same Tilt are collected here so that
/// `reduce()` / `reduce_all()` can compute a median and reject outliers,
/// rather than simply keeping the last-seen advertisement.
pub struct ReadingAccumulator {
    samples: HashMap<TiltColor, Vec<(f64, f64, i16)>>,
}

impl ReadingAccumulator {
    /// Create an empty accumulator.
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    /// Push a raw (temp_f, gravity, rssi) sample for the given color.
    pub fn add(&mut self, color: TiltColor, temp_f: f64, gravity: f64, rssi: i16) {
        self.samples
            .entry(color)
            .or_insert_with(Vec::new)
            .push((temp_f, gravity, rssi));
    }

    /// Return the number of samples collected for `color` (0 if unseen).
    pub fn len(&self, color: TiltColor) -> usize {
        self.samples.get(&color).map_or(0, |v: &Vec<(f64, f64, i16)>| v.len())
    }

    /// Compute the linear-interpolation percentile of a **sorted** slice.
    fn percentile(sorted: &[f64], p: f64) -> f64 {
        let n = sorted.len();
        if n == 0 {
            return 0.0;
        }
        if n == 1 {
            return sorted[0];
        }
        let rank = p * (n - 1) as f64;
        let lo = rank as usize;
        let hi = (lo + 1).min(n - 1);
        let frac = rank - lo as f64;
        sorted[lo] + frac * (sorted[hi] - sorted[lo])
    }

    /// Compute the median of a **sorted** slice.
    fn median_sorted(sorted: &[f64]) -> f64 {
        let n = sorted.len();
        if n == 0 {
            return 0.0;
        }
        if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        }
    }

    /// Apply Tukey IQR fence to a sorted slice of values, returning inlier indices.
    ///
    /// When IQR == 0 (tight cluster) the fence is ±0, so all values equal to the
    /// median pass. This prevents rejecting an entire uniform cluster.
    fn iqr_inlier_mask(sorted: &[f64]) -> Vec<bool> {
        let q1 = Self::percentile(sorted, 0.25);
        let q3 = Self::percentile(sorted, 0.75);
        let iqr = q3 - q1;
        let lo = q1 - 1.5 * iqr;
        let hi = q3 + 1.5 * iqr;
        sorted.iter().map(|v| *v >= lo && *v <= hi).collect()
    }

    /// Reduce all samples for `color` into a single `TiltReading` using median
    /// gravity and temperature after IQR outlier rejection.
    ///
    /// Returns `None` when:
    /// - `color` has not been seen, or
    /// - fewer than `min_samples` inliers remain after outlier rejection.
    ///
    /// RSSI is the mean of **all** raw samples (pre-filter).
    pub fn reduce(&self, color: TiltColor, min_samples: usize, recorded_at: &str) -> Option<TiltReading> {
        let raw = self.samples.get(&color)?;
        if raw.is_empty() {
            return None;
        }

        // Mean RSSI over all raw samples before any filtering.
        let mean_rssi = (raw.iter().map(|(_, _, r)| *r as f64).sum::<f64>() / raw.len() as f64)
            .round() as i16;

        // Sort by gravity for IQR computation.
        let mut by_gravity: Vec<(f64, f64)> = raw.iter().map(|(t, g, _)| (*t, *g)).collect();
        by_gravity.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let gravities: Vec<f64> = by_gravity.iter().map(|(_, g)| *g).collect();
        let grav_mask = Self::iqr_inlier_mask(&gravities);

        // Collect gravity inliers and their paired temperatures.
        let grav_inliers: Vec<f64> = gravities
            .iter()
            .zip(grav_mask.iter())
            .filter_map(|(g, ok)| if *ok { Some(*g) } else { None })
            .collect();
        let temp_candidates: Vec<f64> = by_gravity
            .iter()
            .zip(grav_mask.iter())
            .filter_map(|((t, _), ok)| if *ok { Some(*t) } else { None })
            .collect();

        // Apply IQR fence to temperature independently on the gravity-inlier set.
        let mut sorted_temps = temp_candidates.clone();
        sorted_temps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let temp_mask = Self::iqr_inlier_mask(&sorted_temps);
        let temp_inliers: Vec<f64> = sorted_temps
            .iter()
            .zip(temp_mask.iter())
            .filter_map(|(t, ok)| if *ok { Some(*t) } else { None })
            .collect();

        let raw_count = raw.len();
        let grav_outliers = raw_count - grav_inliers.len();

        if grav_inliers.len() < min_samples || temp_inliers.len() < min_samples {
            log::info!(
                "Tilt {:?}: only {} samples after outlier rejection, skipping (min={})",
                color,
                grav_inliers.len().min(temp_inliers.len()),
                min_samples,
            );
            return None;
        }

        // grav_inliers is already sorted (came from sorted gravities).
        let median_gravity = Self::median_sorted(&grav_inliers);
        let median_temp = Self::median_sorted(&temp_inliers);

        log::info!(
            "Tilt {:?}: {} samples, {} outliers rejected -> temp={:.1}°F gravity={:.4}",
            color,
            raw_count,
            grav_outliers,
            median_temp,
            median_gravity,
        );

        Some(TiltReading::new(
            color,
            median_temp,
            median_gravity,
            Some(mean_rssi),
            recorded_at.to_string(),
        ))
    }

    /// Reduce all colors that have accumulated samples, returning one
    /// `TiltReading` per color that meets `min_samples` after outlier rejection.
    pub fn reduce_all(&self, min_samples: usize, recorded_at: &str) -> Vec<TiltReading> {
        self.samples
            .keys()
            .filter_map(|color| self.reduce(*color, min_samples, recorded_at))
            .collect()
    }

    /// Return the raw sample count for every color seen during the scan window.
    /// Useful for diagnostics and logging outside of `reduce()`.
    pub fn sample_stats(&self) -> Vec<(TiltColor, usize)> {
        self.samples
            .iter()
            .map(|(color, samples)| (*color, samples.len()))
            .collect()
    }
}

/// Exponential weighted average smoother for Tilt gravity and temperature readings.
///
/// Maintains per-color smoothed state across scan cycles to suppress the
/// quantization oscillation (square-wave pattern) that occurs when a Tilt sits
/// on the boundary between two adjacent integer values.
///
/// Formula applied each cycle:
/// ```text
/// smoothed = α * new_value + (1 - α) * previous_smoothed
/// ```
/// α = 1.0 on the first reading (no prior state), producing a pass-through.
/// Typical α: 0.3 (heavy smoothing) to 0.7 (light smoothing).
pub struct EwaSmoother {
    /// Smoothed (gravity, temperature) state per color.
    state: HashMap<TiltColor, (f64, f64)>,
    /// Smoothing factor α ∈ (0, 1].
    alpha: f64,
}

impl EwaSmoother {
    /// Create a new smoother with the given α.
    ///
    /// α = 1.0 disables smoothing (pass-through).
    /// α = 0.3 is a good default for suppressing 1-unit quantization noise.
    pub fn new(alpha: f64) -> Self {
        Self {
            state: HashMap::new(),
            alpha: alpha.clamp(0.01, 1.0),
        }
    }

    /// Apply EWA smoothing to a reading in-place and update stored state.
    ///
    /// On the first reading for a color there is no prior state, so the raw
    /// value is stored as-is (equivalent to α = 1.0 for that cycle).
    pub fn smooth(&mut self, reading: &mut TiltReading) {
        let alpha = self.alpha;
        let new_g = reading.gravity;
        let new_t = reading.temperature_f;

        let (sg, st) = self
            .state
            .entry(reading.color)
            .or_insert((new_g, new_t));

        *sg = alpha * new_g + (1.0 - alpha) * *sg;
        *st = alpha * new_t + (1.0 - alpha) * *st;

        reading.gravity = *sg;
        reading.temperature_f = *st;
    }

    /// Apply smoothing to every reading in a batch in-place.
    pub fn smooth_batch(&mut self, readings: &mut Vec<TiltReading>) {
        for reading in readings.iter_mut() {
            self.smooth(reading);
        }
    }
}

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

    #[test]
    fn accumulator_new_is_empty() {
        let acc = ReadingAccumulator::new();
        assert_eq!(acc.len(TiltColor::Red), 0);
        assert_eq!(acc.len(TiltColor::Green), 0);
    }

    #[test]
    fn accumulator_add_increments_len() {
        let mut acc = ReadingAccumulator::new();
        acc.add(TiltColor::Red, 68.0, 1.050, -60);
        assert_eq!(acc.len(TiltColor::Red), 1);
        acc.add(TiltColor::Red, 69.0, 1.049, -61);
        assert_eq!(acc.len(TiltColor::Red), 2);
        assert_eq!(acc.len(TiltColor::Green), 0);
    }

    #[test]
    fn accumulator_add_separate_colors() {
        let mut acc = ReadingAccumulator::new();
        acc.add(TiltColor::Red, 68.0, 1.050, -60);
        acc.add(TiltColor::Blue, 70.0, 1.040, -55);
        assert_eq!(acc.len(TiltColor::Red), 1);
        assert_eq!(acc.len(TiltColor::Blue), 1);
        assert_eq!(acc.len(TiltColor::Green), 0);
    }

    #[test]
    fn accumulator_len_unseen_color_is_zero() {
        let acc = ReadingAccumulator::new();
        for color in TiltColor::all() {
            assert_eq!(acc.len(*color), 0);
        }
    }

    #[test]
    fn reduce_unseen_color_returns_none() {
        let acc = ReadingAccumulator::new();
        assert!(acc.reduce(TiltColor::Red, 3, "ts").is_none());
    }

    #[test]
    fn reduce_too_few_samples_returns_none() {
        let mut acc = ReadingAccumulator::new();
        acc.add(TiltColor::Red, 68.0, 1.050, -60);
        acc.add(TiltColor::Red, 68.0, 1.050, -60);
        // 2 samples, min_samples=3 → None
        assert!(acc.reduce(TiltColor::Red, 3, "ts").is_none());
    }

    #[test]
    fn reduce_median_gravity_five_clean_samples() {
        let mut acc = ReadingAccumulator::new();
        for (t, g) in [(68.0, 1.010), (68.0, 1.011), (68.0, 1.012), (68.0, 1.013), (68.0, 1.014)] {
            acc.add(TiltColor::Red, t, g, -60);
        }
        let r = acc.reduce(TiltColor::Red, 3, "ts").unwrap();
        assert!((r.gravity - 1.012).abs() < 1e-9, "expected 1.012, got {}", r.gravity);
    }

    #[test]
    fn reduce_median_temperature_five_clean_samples() {
        let mut acc = ReadingAccumulator::new();
        for (t, g) in [(66.0, 1.050), (67.0, 1.050), (68.0, 1.050), (69.0, 1.050), (70.0, 1.050)] {
            acc.add(TiltColor::Red, t, g, -60);
        }
        let r = acc.reduce(TiltColor::Red, 3, "ts").unwrap();
        assert!((r.temperature_f - 68.0).abs() < 1e-9, "expected 68.0, got {}", r.temperature_f);
    }

    #[test]
    fn reduce_spike_gravity_rejected_by_iqr() {
        let mut acc = ReadingAccumulator::new();
        // 10 clean readings around 1.050, plus one huge spike
        for _ in 0..10 {
            acc.add(TiltColor::Green, 68.0, 1.050, -60);
        }
        acc.add(TiltColor::Green, 68.0, 1.999, -60); // spike
        let r = acc.reduce(TiltColor::Green, 3, "ts").unwrap();
        assert!((r.gravity - 1.050).abs() < 1e-9, "spike should be rejected, got {}", r.gravity);
    }

    #[test]
    fn reduce_rssi_is_mean_of_all_raw_samples() {
        let mut acc = ReadingAccumulator::new();
        acc.add(TiltColor::Blue, 68.0, 1.050, -50);
        acc.add(TiltColor::Blue, 68.0, 1.050, -60);
        acc.add(TiltColor::Blue, 68.0, 1.050, -70);
        let r = acc.reduce(TiltColor::Blue, 3, "ts").unwrap();
        // mean = (-50 + -60 + -70) / 3 = -60
        assert_eq!(r.rssi, Some(-60));
    }

    #[test]
    fn reduce_all_returns_one_per_color_with_sufficient_samples() {
        let mut acc = ReadingAccumulator::new();
        for _ in 0..3 {
            acc.add(TiltColor::Red, 68.0, 1.050, -60);
            acc.add(TiltColor::Green, 70.0, 1.040, -55);
        }
        // Blue only has 2 samples — should be excluded
        acc.add(TiltColor::Blue, 72.0, 1.030, -50);
        acc.add(TiltColor::Blue, 72.0, 1.030, -50);
        let results = acc.reduce_all(3, "ts");
        assert_eq!(results.len(), 2);
        let colors: Vec<TiltColor> = results.iter().map(|r| r.color).collect();
        assert!(colors.contains(&TiltColor::Red));
        assert!(colors.contains(&TiltColor::Green));
        assert!(!colors.contains(&TiltColor::Blue));
    }

    #[test]
    fn reduce_timestamp_propagated() {
        let mut acc = ReadingAccumulator::new();
        for _ in 0..3 {
            acc.add(TiltColor::Red, 68.0, 1.050, -60);
        }
        let r = acc.reduce(TiltColor::Red, 3, "2026-04-05T12:00:00Z").unwrap();
        assert_eq!(r.recorded_at, "2026-04-05T12:00:00Z");
    }

    #[test]
    fn reduce_zero_iqr_uniform_cluster_returns_common_value() {
        let mut acc = ReadingAccumulator::new();
        // All samples identical — IQR = 0, no fence applied, all pass through.
        for _ in 0..5 {
            acc.add(TiltColor::Purple, 68.0, 1.050, -60);
        }
        let r = acc.reduce(TiltColor::Purple, 3, "ts").unwrap();
        assert!((r.gravity - 1.050).abs() < 1e-9, "zero-IQR cluster should return 1.050, got {}", r.gravity);
        assert!((r.temperature_f - 68.0).abs() < 1e-9);
    }

    #[test]
    fn reduce_exactly_min_samples_returns_some() {
        let mut acc = ReadingAccumulator::new();
        // Exactly 3 samples, min_samples=3 — should succeed.
        acc.add(TiltColor::Orange, 68.0, 1.050, -60);
        acc.add(TiltColor::Orange, 68.0, 1.051, -60);
        acc.add(TiltColor::Orange, 68.0, 1.052, -60);
        assert!(acc.reduce(TiltColor::Orange, 3, "ts").is_some());
    }

    #[test]
    fn reduce_one_below_min_samples_returns_none() {
        let mut acc = ReadingAccumulator::new();
        // 2 samples, min_samples=3 — should fail.
        acc.add(TiltColor::Yellow, 68.0, 1.050, -60);
        acc.add(TiltColor::Yellow, 68.0, 1.051, -60);
        assert!(acc.reduce(TiltColor::Yellow, 3, "ts").is_none());
    }

    #[test]
    fn reduce_full_realistic_sg_range_no_false_rejections() {
        let mut acc = ReadingAccumulator::new();
        // Realistic Tilt range 1.000 to 1.120 — these are genuinely different brews.
        // A tight cluster within this range should never trigger false outlier rejection.
        let gravities = [1.048, 1.049, 1.050, 1.051, 1.052];
        for g in gravities {
            acc.add(TiltColor::Black, 68.0, g, -60);
        }
        let r = acc.reduce(TiltColor::Black, 5, "ts").unwrap();
        // Median of sorted [1.048, 1.049, 1.050, 1.051, 1.052] = 1.050
        assert!((r.gravity - 1.050).abs() < 1e-9, "expected 1.050, got {}", r.gravity);
    }

    #[test]
    fn reduce_nine_clean_plus_one_spike_rejected() {
        let mut acc = ReadingAccumulator::new();
        for _ in 0..9 {
            acc.add(TiltColor::Pink, 68.0, 1.050, -60);
        }
        acc.add(TiltColor::Pink, 68.0, 0.999, -60); // low spike
        let r = acc.reduce(TiltColor::Pink, 3, "ts").unwrap();
        assert!((r.gravity - 1.050).abs() < 1e-9, "spike 0.999 should be rejected, got {}", r.gravity);
    }

    #[test]
    fn reduce_all_two_colors_both_returned() {
        let mut acc = ReadingAccumulator::new();
        for _ in 0..4 {
            acc.add(TiltColor::Red, 68.0, 1.060, -60);
            acc.add(TiltColor::Green, 70.0, 1.045, -55);
        }
        let results = acc.reduce_all(3, "ts");
        assert_eq!(results.len(), 2);
        let has_red = results.iter().any(|r| r.color == TiltColor::Red);
        let has_green = results.iter().any(|r| r.color == TiltColor::Green);
        assert!(has_red && has_green);
    }

    #[test]
    fn sample_stats_returns_raw_counts() {
        let mut acc = ReadingAccumulator::new();
        acc.add(TiltColor::Red, 68.0, 1.050, -60);
        acc.add(TiltColor::Red, 68.0, 1.050, -60);
        acc.add(TiltColor::Blue, 70.0, 1.040, -55);
        let stats = acc.sample_stats();
        let red_count = stats.iter().find(|(c, _)| *c == TiltColor::Red).map(|(_, n)| *n);
        let blue_count = stats.iter().find(|(c, _)| *c == TiltColor::Blue).map(|(_, n)| *n);
        let green_count = stats.iter().find(|(c, _)| *c == TiltColor::Green).map(|(_, n)| *n);
        assert_eq!(red_count, Some(2));
        assert_eq!(blue_count, Some(1));
        assert_eq!(green_count, None);
    }

    fn make_reading(color: TiltColor, temp: f64, gravity: f64) -> TiltReading {
        TiltReading::new(color, temp, gravity, Some(-60), "ts".to_string())
    }

    #[test]
    fn ewa_first_reading_passes_through() {
        let mut smoother = EwaSmoother::new(0.3);
        let mut r = make_reading(TiltColor::Red, 68.0, 1.050);
        smoother.smooth(&mut r);
        // First reading: no prior state, passes through unchanged.
        assert!((r.gravity - 1.050).abs() < 1e-9);
        assert!((r.temperature_f - 68.0).abs() < 1e-9);
    }

    #[test]
    fn ewa_smooths_oscillating_gravity() {
        let mut smoother = EwaSmoother::new(0.3);
        // Simulate square-wave oscillation between 1.050 and 1.051.
        let values = [1.050, 1.051, 1.050, 1.051, 1.050, 1.051, 1.050, 1.051];
        let mut last = 0.0;
        for g in values {
            let mut r = make_reading(TiltColor::Red, 68.0, g);
            smoother.smooth(&mut r);
            last = r.gravity;
        }
        // After 8 cycles the smoothed value should be between the two extremes,
        // not oscillating. It should be closer to 1.0505 than to either extreme.
        assert!(last > 1.050 && last < 1.051, "expected value between extremes, got {}", last);
    }

    #[test]
    fn ewa_alpha_100_is_passthrough() {
        let mut smoother = EwaSmoother::new(1.0);
        let mut r1 = make_reading(TiltColor::Green, 68.0, 1.050);
        smoother.smooth(&mut r1);
        let mut r2 = make_reading(TiltColor::Green, 68.0, 1.060);
        smoother.smooth(&mut r2);
        // α=1.0: output always equals input.
        assert!((r2.gravity - 1.060).abs() < 1e-9);
    }

    #[test]
    fn ewa_independent_per_color() {
        let mut smoother = EwaSmoother::new(0.3);
        let mut red1 = make_reading(TiltColor::Red, 68.0, 1.050);
        let mut green1 = make_reading(TiltColor::Green, 70.0, 1.040);
        smoother.smooth(&mut red1);
        smoother.smooth(&mut green1);
        // Both pass through on first reading.
        let mut red2 = make_reading(TiltColor::Red, 68.0, 1.060);
        let mut green2 = make_reading(TiltColor::Green, 70.0, 1.030);
        smoother.smooth(&mut red2);
        smoother.smooth(&mut green2);
        // Red and green states are independent — red smoothed up, green smoothed down.
        assert!(red2.gravity > 1.050, "red should have increased toward 1.060");
        assert!(green2.gravity < 1.040, "green should have decreased toward 1.030");
    }

    #[test]
    fn ewa_smooth_batch_applies_to_all() {
        let mut smoother = EwaSmoother::new(0.3);
        let mut readings = vec![
            make_reading(TiltColor::Red, 68.0, 1.050),
            make_reading(TiltColor::Blue, 70.0, 1.040),
        ];
        smoother.smooth_batch(&mut readings);
        // First reading for each — pass-through.
        assert!((readings[0].gravity - 1.050).abs() < 1e-9);
        assert!((readings[1].gravity - 1.040).abs() < 1e-9);
    }
}
