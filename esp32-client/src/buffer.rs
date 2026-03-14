//! Reading buffer for offline resilience.
//!
//! Bounded circular buffer that stores TiltReadings when the server is
//! unreachable. Oldest readings are dropped when the buffer is full.

use std::collections::VecDeque;

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
}
