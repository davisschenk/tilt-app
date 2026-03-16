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
