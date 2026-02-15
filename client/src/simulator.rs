use std::time::Instant;

use chrono::Utc;
use rand::Rng;
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
    pub fn new(
        colors: Vec<TiltColor>,
        og: f64,
        target_fg: f64,
        base_temp: f64,
    ) -> Self {
        Self {
            colors,
            og,
            target_fg,
            base_temp,
            start: Instant::now(),
        }
    }

    pub fn generate_readings(&self) -> Vec<TiltReading> {
        let elapsed_secs = self.start.elapsed().as_secs_f64();
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
