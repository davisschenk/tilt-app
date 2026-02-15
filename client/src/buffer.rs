use std::collections::VecDeque;
use std::time::Duration;

use shared::TiltReading;

pub struct ReadingBuffer {
    buffer: VecDeque<TiltReading>,
    capacity: usize,
}

impl ReadingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push_batch(&mut self, readings: &[TiltReading]) {
        for reading in readings {
            if self.buffer.len() >= self.capacity {
                self.buffer.pop_front();
            }
            self.buffer.push_back(reading.clone());
        }
    }

    pub fn drain_all(&mut self) -> Vec<TiltReading> {
        self.buffer.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

pub struct Backoff {
    current: Duration,
    initial: Duration,
    max: Duration,
    factor: u32,
}

impl Backoff {
    pub fn new(initial: Duration, max: Duration, factor: u32) -> Self {
        Self {
            current: initial,
            initial,
            max,
            factor,
        }
    }

    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = (self.current * self.factor).min(self.max);
        delay
    }

    pub fn reset(&mut self) {
        self.current = self.initial;
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(1),
            Duration::from_secs(60),
            2,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shared::TiltColor;

    fn make_reading(temp: f64) -> TiltReading {
        TiltReading::new(TiltColor::Red, temp, 1.050, None, Utc::now())
    }

    #[test]
    fn buffer_new_is_empty() {
        let buf = ReadingBuffer::new(10);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn buffer_push_and_drain() {
        let mut buf = ReadingBuffer::new(10);
        buf.push_batch(&[make_reading(68.0), make_reading(69.0)]);
        assert_eq!(buf.len(), 2);

        let drained = buf.drain_all();
        assert_eq!(drained.len(), 2);
        assert!(buf.is_empty());
    }

    #[test]
    fn buffer_drops_oldest_when_full() {
        let mut buf = ReadingBuffer::new(3);
        buf.push_batch(&[
            make_reading(1.0),
            make_reading(2.0),
            make_reading(3.0),
        ]);
        assert_eq!(buf.len(), 3);

        buf.push_batch(&[make_reading(4.0), make_reading(5.0)]);
        assert_eq!(buf.len(), 3);

        let drained = buf.drain_all();
        assert!((drained[0].temperature_f - 3.0).abs() < f64::EPSILON);
        assert!((drained[1].temperature_f - 4.0).abs() < f64::EPSILON);
        assert!((drained[2].temperature_f - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn backoff_doubles_delay() {
        let mut bo = Backoff::new(
            Duration::from_secs(1),
            Duration::from_secs(60),
            2,
        );
        assert_eq!(bo.next_delay(), Duration::from_secs(1));
        assert_eq!(bo.next_delay(), Duration::from_secs(2));
        assert_eq!(bo.next_delay(), Duration::from_secs(4));
        assert_eq!(bo.next_delay(), Duration::from_secs(8));
    }

    #[test]
    fn backoff_caps_at_max() {
        let mut bo = Backoff::new(
            Duration::from_secs(16),
            Duration::from_secs(60),
            2,
        );
        assert_eq!(bo.next_delay(), Duration::from_secs(16));
        assert_eq!(bo.next_delay(), Duration::from_secs(32));
        assert_eq!(bo.next_delay(), Duration::from_secs(60));
        assert_eq!(bo.next_delay(), Duration::from_secs(60));
    }

    #[test]
    fn backoff_resets_to_initial() {
        let mut bo = Backoff::new(
            Duration::from_secs(1),
            Duration::from_secs(60),
            2,
        );
        bo.next_delay();
        bo.next_delay();
        bo.next_delay();
        bo.reset();
        assert_eq!(bo.next_delay(), Duration::from_secs(1));
    }
}
