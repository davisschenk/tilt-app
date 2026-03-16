use chrono::{DateTime, Utc};

/// A (time, gravity) data point used for curve fitting.
pub struct GravityPoint {
    pub hours: f64,
    pub gravity: f64,
}

/// Predict the date when gravity will reach `target_fg` using a log-linear
/// least-squares fit on up to the last `max_points` readings.
///
/// The model is: ln(g - target_fg) = m*t + b, solved for t when g = target_fg.
/// Returns None when:
/// - fewer than 3 points available
/// - gravity is already at or below target_fg
/// - gravity is flat (slope >= 0, not decreasing)
/// - predicted date is in the past or more than 60 days away (unreliable)
pub fn predict_fg_date(
    points: &[GravityPoint],
    target_fg: f64,
    reference_time: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if points.len() < 3 {
        return None;
    }

    let latest_gravity = points.last()?.gravity;
    if latest_gravity <= target_fg {
        return None;
    }

    // Shift gravity down by a value slightly below target_fg so that
    // (gravity - offset) > 0 for all points above target_fg and we can take ln.
    let offset = target_fg - 0.001;

    let valid: Vec<_> = points
        .iter()
        .filter(|p| p.gravity > offset + f64::EPSILON)
        .collect();
    if valid.len() < 3 {
        return None;
    }

    let n = valid.len() as f64;
    let sum_t: f64 = valid.iter().map(|p| p.hours).sum();
    let sum_y: f64 = valid.iter().map(|p| (p.gravity - offset).ln()).sum();
    let sum_tt: f64 = valid.iter().map(|p| p.hours * p.hours).sum();
    let sum_ty: f64 = valid
        .iter()
        .map(|p| p.hours * (p.gravity - offset).ln())
        .sum();

    let denom = n * sum_tt - sum_t * sum_t;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let slope = (n * sum_ty - sum_t * sum_y) / denom;
    if slope >= 0.0 {
        return None;
    }

    let intercept = (sum_y - slope * sum_t) / n;

    // Solve slope*t + intercept = ln(target_fg - offset)
    let y_target = (target_fg - offset).ln();
    let t_target = (y_target - intercept) / slope;

    let latest_hours = points.last()?.hours;
    if t_target <= latest_hours {
        return None;
    }

    let hours_remaining = t_target - latest_hours;
    if hours_remaining > 60.0 * 24.0 {
        return None;
    }

    let delta = chrono::Duration::seconds((hours_remaining * 3600.0) as i64);
    Some(reference_time + delta)
}

/// Compute ABV using the standard homebrewing formula.
/// ABV = (OG - FG) × 131.25
pub fn compute_abv(og: f64, fg: f64) -> f64 {
    (og - fg) * 131.25
}

/// Compute apparent attenuation as a percentage.
/// AA% = ((OG - current_gravity) / (OG - 1.0)) × 100
pub fn compute_apparent_attenuation(og: f64, current_gravity: f64) -> f64 {
    if (og - 1.0).abs() < f64::EPSILON {
        return 0.0;
    }
    ((og - current_gravity) / (og - 1.0)) * 100.0
}

/// Compute live ABV estimate using current gravity instead of final gravity.
pub fn compute_live_abv(og: f64, current_gravity: f64) -> f64 {
    compute_abv(og, current_gravity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_points(gravities: &[(f64, f64)]) -> Vec<GravityPoint> {
        gravities
            .iter()
            .map(|(h, g)| GravityPoint {
                hours: *h,
                gravity: *g,
            })
            .collect()
    }

    #[test]
    fn predict_fg_date_returns_future_for_active_fermentation() {
        let now = Utc::now();
        // Strong exponential decay: gravity clearly approaching 1.010 from above
        let points = make_points(&[
            (0.0, 1.060),
            (24.0, 1.045),
            (48.0, 1.032),
            (72.0, 1.022),
            (96.0, 1.016),
        ]);
        let result = predict_fg_date(&points, 1.010, now);
        assert!(result.is_some(), "expected a prediction, got None");
        assert!(result.unwrap() > now, "prediction must be in the future");
    }

    #[test]
    fn predict_fg_date_returns_none_when_gravity_already_at_target() {
        let now = Utc::now();
        let points = make_points(&[(0.0, 1.015), (12.0, 1.012), (24.0, 1.010)]);
        let result = predict_fg_date(&points, 1.010, now);
        assert!(result.is_none());
    }

    #[test]
    fn predict_fg_date_returns_none_when_gravity_flat() {
        let now = Utc::now();
        let points = make_points(&[(0.0, 1.012), (12.0, 1.012), (24.0, 1.012)]);
        let result = predict_fg_date(&points, 1.010, now);
        assert!(result.is_none(), "flat gravity should return None");
    }

    #[test]
    fn predict_fg_date_returns_none_for_fewer_than_3_points() {
        let now = Utc::now();
        let points = make_points(&[(0.0, 1.050), (12.0, 1.040)]);
        let result = predict_fg_date(&points, 1.010, now);
        assert!(result.is_none());
    }

    #[test]
    fn compute_abv_known_values() {
        let abv = compute_abv(1.060, 1.010);
        assert!((abv - 6.5625).abs() < 0.001, "expected ~6.56, got {abv}");
    }

    #[test]
    fn compute_abv_zero_attenuation() {
        let abv = compute_abv(1.050, 1.050);
        assert!((abv - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_apparent_attenuation_known_values() {
        let aa = compute_apparent_attenuation(1.060, 1.010);
        assert!((aa - 83.333_333).abs() < 0.001, "expected ~83.3%, got {aa}");
    }

    #[test]
    fn compute_apparent_attenuation_full() {
        let aa = compute_apparent_attenuation(1.060, 1.000);
        assert!((aa - 100.0).abs() < 0.001);
    }

    #[test]
    fn compute_apparent_attenuation_zero_og_guard() {
        let aa = compute_apparent_attenuation(1.0, 1.0);
        assert!((aa - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_live_abv_delegates_to_compute_abv() {
        let og = 1.055;
        let current = 1.020;
        assert!((compute_live_abv(og, current) - compute_abv(og, current)).abs() < f64::EPSILON);
    }
}
