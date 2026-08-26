// compiler-rs/fer/tests/driver_tests.rs

use fer::{DriverError, run_source};
use runtime::Value;

#[test]
fn runs_a_source_file_through_the_compiler_pipeline() {
    let report = run_source("main.fer", "main() -> i64 { 40 + 2 }").expect("valid source must run");
    assert_eq!(report.result, Value::Integer(42));
}

#[test]
fn returns_structured_diagnostics_before_execution() {
    let error = run_source("broken.fer", "value = missing").expect_err("undefined name must fail");
    assert!(
        matches!(error, DriverError::Diagnostics(diagnostics) if diagnostics.iter().any(|diagnostic| diagnostic.code == "undefined-name"))
    );
}

#[test]
fn renders_structured_diagnostics_in_chinese() {
    let DriverError::Diagnostics(diagnostics) =
        run_source("broken.fer", "value = missing").expect_err("undefined name must fail")
    else {
        panic!("expected structured diagnostics");
    };

    let rendered = fer::render_diagnostics(&diagnostics, "zh-hans").expect("locale must render");
    assert_eq!(rendered[0].message, "无法解析名称 missing");
}
