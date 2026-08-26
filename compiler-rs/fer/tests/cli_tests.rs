// compiler-rs/fer/tests/cli_tests.rs

use std::process::Command;

#[test]
fn run_command_executes_the_repository_runtime_example() {
    let example = format!("{}/../../examples/runtime.fer", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args(["run", &example])
        .output()
        .expect("fer binary must start");

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[test]
fn run_command_executes_the_repository_hello_example() {
    let example = format!("{}/../../examples/hello.fer", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args(["run", &example])
        .output()
        .expect("fer binary must start");

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "Hello World"
    );
}

#[test]
fn run_command_executes_the_repository_string_example() {
    let example = format!("{}/../../examples/string.fer", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args(["run", &example])
        .output()
        .expect("fer binary must start");

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello, Fuyeor!\n1 + 1 = 2\nHello, Fuyeor!\nThis is a string\nthat spans multiple\nlines easily.\nThis is a string that spans multiple lines easily.\n"
    );
}
