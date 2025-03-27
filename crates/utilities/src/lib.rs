use core::time::Duration;
use std::time::Instant;

use tracing::debug;

/// Display the duration as a string with units. Display is handled in the folloing order:
/// 1. `>= 10s` displays seconds only.
/// 1. `>= 1s` displays seconds with 1dp.
/// 1. `>= 1ms` displays milliseconds only.
/// 1. `>= 1μs` displays microseconds only.
/// 1. `< 1μs` displays nanoseconds only.
#[inline]
pub fn display_duration(duration: Duration) -> String {
    if duration.as_secs() >= 10 {
        format!("{}s", duration.as_secs())
    } else if duration.as_secs() >= 1 {
        format!("{:.1}s", duration.as_secs_f32())
    } else if duration.as_millis() >= 1 {
        format!("{}ms", duration.as_millis())
    } else if duration.as_micros() >= 1 {
        format!("{}μs", duration.as_micros())
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

#[cfg(test)]
mod test {
    use core::time::Duration;

    use crate::display_duration;

    #[test]
    fn displays_duration_gte_10s() {
        assert_eq!(display_duration(Duration::from_secs_f64(12.5555)), "12s");
    }

    #[test]
    fn displays_duration_gte_1s() {
        assert_eq!(display_duration(Duration::from_secs_f64(2.5555)), "2.6s");
    }

    #[test]
    fn displays_duration_gte_1ms() {
        assert_eq!(display_duration(Duration::from_secs_f64(0.500555)), "500ms");
    }

    #[test]
    fn displays_duration_gte_1μs() {
        assert_eq!(
            display_duration(Duration::from_secs_f64(0.000500555)),
            "500μs"
        );
    }

    #[test]
    fn displays_duration_lt_1μs() {
        assert_eq!(
            display_duration(Duration::from_secs_f64(0.000000555)),
            "555ns"
        );
    }
}
