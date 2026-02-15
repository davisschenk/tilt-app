use std::time::Instant;

use chrono::Utc;
use rand::RngExt;
use shared::{TiltColor, TiltReading};

/// Fermentation decay constant chosen so ~95% attenuation at 14 days.
/// e^(-k * 14_days_in_secs) ≈ 0.05  →  k ≈ 3.0 / (14 * 86400)
const DECAY_K: f64 = 3.0 / (14.0 * 86400.0);

pub struct TiltSimulator {
    colors: Vec<TiltColor>,
    og: f64,
    target_fg: f64,
    base_temp: f64,
    start: Instant,
}

impl TiltSimulator {
    pub fn new(colors: Vec<TiltColor>, og: f64, target_fg: f64, base_temp: f64) -> Self {
        Self {
            colors,
            og,
            target_fg,
            base_temp,
            start: Instant::now(),
        }
    }

    pub fn generate_readings(&self) -> Vec<TiltReading> {
        self.generate_readings_at(self.start.elapsed().as_secs_f64())
    }

    fn generate_readings_at(&self, elapsed_secs: f64) -> Vec<TiltReading> {
        let mut rng = rand::rng();

        self.colors
            .iter()
            .map(|color| {
                let base_gravity =
                    self.target_fg + (self.og - self.target_fg) * (-DECAY_K * elapsed_secs).exp();
                let gravity_noise: f64 = rng.random_range(-0.001..=0.001);
                let gravity = (base_gravity + gravity_noise).clamp(self.target_fg, self.og);

                let temp_jitter: f64 = rng.random_range(-0.5..=0.5);
                let temperature_f = self.base_temp + temp_jitter;

                let rssi: i16 = rng.random_range(-80..=-60);

                TiltReading::new(*color, temperature_f, gravity, Some(rssi), Utc::now())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_sim() -> TiltSimulator {
        TiltSimulator::new(vec![TiltColor::Red], 1.055, 1.012, 68.0)
    }

    fn multi_color_sim() -> TiltSimulator {
        TiltSimulator::new(
            vec![TiltColor::Red, TiltColor::Blue, TiltColor::Green],
            1.055,
            1.012,
            68.0,
        )
    }

    #[test]
    fn new_constructs_without_panic() {
        let _sim = default_sim();
    }

    #[test]
    fn single_color_produces_one_reading() {
        let sim = default_sim();
        let readings = sim.generate_readings();
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].color, TiltColor::Red);
    }

    #[test]
    fn multi_color_produces_correct_count() {
        let sim = multi_color_sim();
        let readings = sim.generate_readings();
        assert_eq!(readings.len(), 3);
        assert_eq!(readings[0].color, TiltColor::Red);
        assert_eq!(readings[1].color, TiltColor::Blue);
        assert_eq!(readings[2].color, TiltColor::Green);
    }

    #[test]
    fn gravity_within_og_to_target_fg_bounds() {
        let sim = default_sim();
        for elapsed in [0.0, 3600.0, 86400.0, 604800.0, 1_209_600.0] {
            for _ in 0..20 {
                let readings = sim.generate_readings_at(elapsed);
                for r in &readings {
                    assert!(
                        r.gravity >= 1.012 && r.gravity <= 1.055,
                        "gravity {} out of bounds at elapsed={}",
                        r.gravity,
                        elapsed,
                    );
                }
            }
        }
    }

    #[test]
    fn temperature_within_jitter_bounds() {
        let sim = default_sim();
        for _ in 0..100 {
            let readings = sim.generate_readings();
            for r in &readings {
                assert!(
                    r.temperature_f >= 67.0 && r.temperature_f <= 69.0,
                    "temperature {} out of bounds",
                    r.temperature_f,
                );
            }
        }
    }

    #[test]
    fn rssi_in_valid_range() {
        let sim = default_sim();
        for _ in 0..100 {
            let readings = sim.generate_readings();
            for r in &readings {
                let rssi = r.rssi.expect("RSSI should be Some");
                assert!(rssi >= -80 && rssi <= -60, "RSSI {} out of range", rssi,);
            }
        }
    }

    #[test]
    fn gravity_decreases_over_time() {
        let sim = default_sim();
        let early: f64 = (0..50)
            .map(|_| sim.generate_readings_at(0.0)[0].gravity)
            .sum::<f64>()
            / 50.0;
        let late: f64 = (0..50)
            .map(|_| sim.generate_readings_at(604800.0)[0].gravity)
            .sum::<f64>()
            / 50.0;
        assert!(
            late < early,
            "average gravity at 7d ({late}) should be less than at 0s ({early})",
        );
    }

    #[test]
    fn gravity_near_target_after_14_days() {
        let sim = default_sim();
        let fourteen_days = 14.0 * 86400.0;
        let avg: f64 = (0..50)
            .map(|_| sim.generate_readings_at(fourteen_days)[0].gravity)
            .sum::<f64>()
            / 50.0;
        assert!(
            (avg - 1.012).abs() < 0.005,
            "average gravity after 14d ({avg}) should be near target FG 1.012",
        );
    }
}
