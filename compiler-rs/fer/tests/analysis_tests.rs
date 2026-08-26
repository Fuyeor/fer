// compiler-rs/fer/tests/analysis_tests.rs

use fer::{DriverError, analyze_source};

#[test]
fn exposes_read_only_analysis_snapshot() {
    let snapshot = analyze_source("main.fer", "main = () -> i64 { 40 + 2 }")
        .expect("valid source must produce an analysis snapshot");

    assert_eq!(snapshot.file_id, snapshot.hir.file_id);
    assert!(snapshot.diagnostics.is_empty());
    assert!(snapshot.resolution.diagnostics.is_empty());
    assert!(snapshot.types.diagnostics.is_empty());
}

#[test]
fn returns_analysis_diagnostics_without_running_code() {
    let error = analyze_source("broken.fer", "value = missing")
        .expect_err("undefined names must fail analysis");
    let DriverError::Diagnostics(diagnostics) = error else {
        panic!("expected structured analysis diagnostics");
    };

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "undefined-name")
    );
}
