use std::time::Instant;

use chrono::Utc;
use rand::RngExt;
use shared::{TiltColor, TiltReading};

/// Full sine wave period in seconds (3 minutes).
const PERIOD_SECS: f64 = 180.0;

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

        let color_count = self.colors.len() as f64;

        self.colors
            .iter()
            .enumerate()
            .map(|(i, color)| {
                let color_offset = (i as f64 / color_count) * std::f64::consts::TAU;
                let phase = (elapsed_secs / PERIOD_SECS) * std::f64::consts::TAU + color_offset;
                let mid_gravity = (self.og + self.target_fg) / 2.0;
                let amplitude = (self.og - self.target_fg) / 2.0;
                let base_gravity = mid_gravity + amplitude * phase.sin();
                let gravity_noise: f64 = rng.random_range(-0.001..=0.001);
                let gravity = (base_gravity + gravity_noise).clamp(self.target_fg, self.og);

                let temp_jitter: f64 = rng.random_range(-0.5..=0.5);
                let temperature_f = self.base_temp + 2.0 * phase.cos() + temp_jitter;

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
        for elapsed in [0.0, 45.0, 90.0, 135.0, 180.0, 270.0] {
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
                    r.temperature_f >= 65.0 && r.temperature_f <= 71.0,
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
    fn gravity_peaks_at_quarter_period() {
        let sim = default_sim();
        let quarter = PERIOD_SECS / 4.0; // sin peaks at π/2
        let avg: f64 = (0..50)
            .map(|_| sim.generate_readings_at(quarter)[0].gravity)
            .sum::<f64>()
            / 50.0;
        assert!(
            (avg - 1.055).abs() < 0.003,
            "average gravity at quarter period ({avg}) should be near OG 1.055",
        );
    }

    #[test]
    fn gravity_troughs_at_three_quarter_period() {
        let sim = default_sim();
        let three_quarter = PERIOD_SECS * 3.0 / 4.0; // sin troughs at 3π/2
        let avg: f64 = (0..50)
            .map(|_| sim.generate_readings_at(three_quarter)[0].gravity)
            .sum::<f64>()
            / 50.0;
        assert!(
            (avg - 1.012).abs() < 0.003,
            "average gravity at 3/4 period ({avg}) should be near target FG 1.012",
        );
    }
}
