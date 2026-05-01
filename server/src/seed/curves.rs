//! Synthetic data curves for the seeder. Deterministic — no RNG, no time-based
//! noise — so seeded data is byte-identical across runs.

/// Sigmoid-ish gravity drop from `og` to `final_gravity` over `total_minutes`,
/// sampled at `t_minutes` from the start.
pub fn gravity_at(og: f64, final_gravity: f64, total_minutes: f64, t_minutes: f64) -> f64 {
    let progress = (t_minutes / total_minutes).clamp(0.0, 1.0);
    // Logistic curve centered at 0.5, scaled to (0, 1).
    let k = 8.0;
    let sigmoid = 1.0 / (1.0 + (-k * (progress - 0.5)).exp());
    og - (og - final_gravity) * sigmoid
}

/// Deterministic temperature wobble around `base_f` with amplitude `amplitude_f`,
/// derived from the sample index — no RNG.
pub fn temperature_at(base_f: f64, amplitude_f: f64, sample_index: usize) -> f64 {
    let phase = (sample_index as f64) * 0.137; // arbitrary irrational-ish step
    base_f + amplitude_f * phase.sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_starts_near_og_and_ends_near_final() {
        let og = 1.062;
        let fg = 1.040;
        let total = 24.0 * 60.0;
        // Logistic sigmoid at t=0 is ~0.018 (not 0), so gravity starts very close to og.
        let start = gravity_at(og, fg, total, 0.0);
        assert!(
            (start - og).abs() < 0.001,
            "start={start} too far from og={og}"
        );
        let end = gravity_at(og, fg, total, total);
        assert!((end - fg).abs() < 0.001, "end={end} too far from fg={fg}");
    }

    #[test]
    fn temperature_stays_within_amplitude() {
        for i in 0..1000 {
            let t = temperature_at(68.0, 1.5, i);
            assert!(t >= 66.5 && t <= 69.5, "got {t} at i={i}");
        }
    }
}
