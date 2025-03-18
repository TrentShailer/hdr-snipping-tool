use core::time::Duration;
use std::time::Instant;

use tracing::debug;

/// Display the duration as a string with units. Display is handled in the folloing order:
/// 1. `>= 10s` displays seconds only.
/// 1. `>= 1s` displays seconds with 1dp.
/// 1. `>= 1ms` displays milliseconds only.
/// 1. `>= 1µs` displays microseconds only.
/// 1. `< 1µs` displays nanoseconds only.
#[inline]
pub fn display_duration(duration: Duration) -> String {
    if duration.as_secs() >= 10 {
        format!("{}s", duration.as_secs())
    } else if duration.as_secs() >= 1 {
        format!("{:.1}s", duration.as_secs_f32())
    } else if duration.as_millis() >= 1 {
        format!("{}ms", duration.as_millis())
    } else if duration.as_micros() >= 1 {
        format!("{}µs", duration.as_micros())
    } else {
        format!("{}ns", duration.as_nanos())
    }
}

/// Structure that on drop, logs the time since construction.
pub struct DebugTime {
    label: String,
    start: Instant,
}

impl DebugTime {
    /// Start a new debug timer with the given label.
    /// Label is printed in the format `[Timing] {label} took {duration}`
    pub fn start<S: Into<String>>(label: S) -> Self {
        Self {
            label: label.into(),
            start: Instant::now(),
        }
    }
}

impl Drop for DebugTime {
    fn drop(&mut self) {
        debug!(
            "[Timing] {} took {}",
            self.label,
            display_duration(self.start.elapsed())
        )
    }
}
