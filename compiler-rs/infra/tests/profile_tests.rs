// infra/tests/profile_tests.rs
use infra::profile::{NullProfileCollector, TimingGuard};
use std::time::Duration;

#[test]
fn timing_guard_records_elapsed_time() {
    let mut records = Vec::new();
    {
        let _guard = TimingGuard::start("parse", &mut records);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].label, "parse");
    assert!(records[0].duration.as_millis() >= 1);
}

#[test]
fn null_profile_collector_does_nothing() {
    let mut collector = NullProfileCollector::default();
    // Just ensure it doesn't panic and can be called.
    collector.record("some_phase");
}
