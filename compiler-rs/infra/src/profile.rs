// infra/src/profile.rs
/// Infrastructure for performance measurement.
///
/// This module defines a zero-cost (when disabled) profiling hook.
/// Currently it provides a manual `TimingRecord` that can be aggregated
/// by a higher-level profiler (e.g. Salsa's built-in or a custom one).
use std::time::{Duration, Instant};

/// A single timing measurement.
/// Created by `TimingGuard::start` and finalized on drop.
#[derive(Debug, Clone)]
pub struct TimingRecord {
    /// Human-readable label, e.g. "hir_lowering"
    pub label: &'static str,
    /// Duration in wall-clock time.
    pub duration: Duration,
}

/// A guard that measures elapsed time from construction to drop.
/// When dropped, it pushes a `TimingRecord` into the provided `Vec`.
/// This is intentionally simple: later we can integrate with tracing.
pub struct TimingGuard<'a> {
    label: &'static str,
    start: Instant,
    collector: &'a mut Vec<TimingRecord>,
}

impl<'a> TimingGuard<'a> {
    /// Start timing. The record will be pushed to `collector` when this guard is dropped.
    pub fn start(label: &'static str, collector: &'a mut Vec<TimingRecord>) -> Self {
        Self {
            label,
            start: Instant::now(),
            collector,
        }
    }
}

impl<'a> Drop for TimingGuard<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.collector.push(TimingRecord {
            label: self.label,
            duration,
        });
    }
}

/// A dummy profile collector that can be used when profiling is disabled.
/// All methods are no-ops, and the compiler should optimize them away.
#[derive(Debug, Default)]
pub struct NullProfileCollector;

impl NullProfileCollector {
    pub fn record(&mut self, _label: &'static str) {
        // no-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_guard_records_duration() {
        let mut records = Vec::new();
        {
            let _guard = TimingGuard::start("test_phase", &mut records);
            // simulate some work
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].label, "test_phase");
        assert!(records[0].duration.as_millis() >= 1);
    }
}
