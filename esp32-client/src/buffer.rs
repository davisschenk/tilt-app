//! Reading buffer for offline resilience.
//!
//! Bounded circular buffer that stores TiltReadings when the server is
//! unreachable. Oldest readings are dropped when the buffer is full.

use std::collections::VecDeque;
use std::time::Duration;

use crate::tilt::TiltReading;

pub struct ReadingBuffer {
    readings: VecDeque<TiltReading>,
    capacity: usize,
}

impl ReadingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            readings: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push_batch(&mut self, readings: &[TiltReading]) {
        for reading in readings {
            if self.readings.len() >= self.capacity {
                self.readings.pop_front();
            }
            self.readings.push_back(reading.clone());
        }
    }

    pub fn drain_all(&mut self) -> Vec<TiltReading> {
        self.readings.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.readings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.readings.is_empty()
    }
}

pub struct Backoff {
    initial_ms: u64,
    max_ms: u64,
    factor: u64,
    current_ms: u64,
}

impl Backoff {
    pub fn new(initial_ms: u64, max_ms: u64, factor: u64) -> Self {
        Self {
            initial_ms,
            max_ms,
            factor,
            current_ms: initial_ms,
        }
    }

    pub fn next_delay(&mut self) -> Duration {
        let delay = Duration::from_millis(self.current_ms);
        self.current_ms = (self.current_ms.saturating_mul(self.factor)).min(self.max_ms);
        delay
    }

    pub fn reset(&mut self) {
        self.current_ms = self.initial_ms;
    }

    pub fn current_delay_ms(&self) -> u64 {
        self.current_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tilt::TiltColor;

    fn make_reading(temp: f64) -> TiltReading {
        TiltReading::new(TiltColor::Red, temp, 1.050, None, String::new())
    }

    #[test]
    fn empty_buffer() {
        let buf = ReadingBuffer::new(10);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn push_and_drain() {
        let mut buf = ReadingBuffer::new(10);
        let readings = vec![make_reading(68.0), make_reading(69.0), make_reading(70.0)];
        buf.push_batch(&readings);
        assert_eq!(buf.len(), 3);

        let drained = buf.drain_all();
        assert_eq!(drained.len(), 3);
        assert!(buf.is_empty());
        assert!((drained[0].temperature_f - 68.0).abs() < f64::EPSILON);
        assert!((drained[2].temperature_f - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn capacity_overflow_drops_oldest() {
        let mut buf = ReadingBuffer::new(3);
        buf.push_batch(&[make_reading(1.0), make_reading(2.0), make_reading(3.0)]);
        assert_eq!(buf.len(), 3);

        buf.push_batch(&[make_reading(4.0), make_reading(5.0)]);
        assert_eq!(buf.len(), 3);

        let drained = buf.drain_all();
        assert!((drained[0].temperature_f - 3.0).abs() < f64::EPSILON);
        assert!((drained[1].temperature_f - 4.0).abs() < f64::EPSILON);
        assert!((drained[2].temperature_f - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn drain_empty_returns_empty_vec() {
        let mut buf = ReadingBuffer::new(5);
        let drained = buf.drain_all();
        assert!(drained.is_empty());
    }

    #[test]
    fn multiple_push_drain_cycles() {
        let mut buf = ReadingBuffer::new(5);
        buf.push_batch(&[make_reading(1.0), make_reading(2.0)]);
        assert_eq!(buf.drain_all().len(), 2);
        assert!(buf.is_empty());

        buf.push_batch(&[make_reading(3.0)]);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.drain_all().len(), 1);
    }

    #[test]
    fn backoff_doubles_each_call() {
        let mut b = Backoff::new(1000, 60000, 2);
        assert_eq!(b.next_delay(), Duration::from_millis(1000));
        assert_eq!(b.next_delay(), Duration::from_millis(2000));
        assert_eq!(b.next_delay(), Duration::from_millis(4000));
        assert_eq!(b.next_delay(), Duration::from_millis(8000));
    }

    #[test]
    fn backoff_caps_at_max() {
        let mut b = Backoff::new(1000, 5000, 2);
        b.next_delay(); // 1000
        b.next_delay(); // 2000
        b.next_delay(); // 4000
        let d = b.next_delay(); // should be capped at 5000
        assert_eq!(d, Duration::from_millis(5000));
        let d2 = b.next_delay(); // still capped
        assert_eq!(d2, Duration::from_millis(5000));
    }

    #[test]
    fn backoff_reset() {
        let mut b = Backoff::new(1000, 60000, 2);
        b.next_delay(); // 1000
        b.next_delay(); // 2000
        b.reset();
        assert_eq!(b.next_delay(), Duration::from_millis(1000));
    }

    #[test]
    fn backoff_no_overflow() {
        // Use large but reasonable values to verify saturating_mul doesn't panic
        let mut b = Backoff::new(30_000, 60_000, 2);
        b.next_delay(); // 30_000
        b.next_delay(); // 60_000 (capped)
        // Should stay at max without overflow
        assert_eq!(b.next_delay(), Duration::from_millis(60_000));
        assert_eq!(b.current_delay_ms(), 60_000);

        // Verify saturating_mul with extreme values doesn't panic
        let mut b2 = Backoff::new(u64::MAX / 4, u64::MAX, 2);
        b2.next_delay();
        b2.next_delay(); // saturating_mul should cap, not panic
    }
}
