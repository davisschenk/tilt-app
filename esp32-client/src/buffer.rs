//! Reading buffer for offline resilience.
//!
//! Bounded circular buffer that stores TiltReadings when the server is
//! unreachable. Oldest readings are dropped when the buffer is full.
