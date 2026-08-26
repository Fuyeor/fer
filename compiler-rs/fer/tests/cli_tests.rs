// compiler-rs/fer/tests/cli_tests.rs

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

struct TempFerFile {
    path: PathBuf,
}

impl TempFerFile {
    /// Create an isolated `.fer` fixture and remove it when the test exits.
    fn new(source: &str) -> Self {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("fer-cli-{}-{}.fer", std::process::id(), sequence));
        fs::write(&path, source).expect("temporary Fer fixture must write");
        Self { path }
    }

    fn as_path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> String {
        fs::read_to_string(&self.path).expect("temporary Fer fixture must read")
    }
}

impl Drop for TempFerFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Execute the `fer fmt` subcommand against a temporary source file.
fn run_fmt(file: &TempFerFile, check: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fer"));
    command.arg("fmt");
    if check {
        command.arg("--check");
    }
    command.arg(file.as_path());
    command.output().expect("fer binary must start")
}

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

#[test]
fn fmt_command_rewrites_a_file_without_running_the_runtime() {
    let file = TempFerFile::new("main=()->i64{\nanswer=40+2\nanswer\n}\n");
    let output = run_fmt(&file, false);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(
        file.read(),
        "main = () -> i64 {\n  answer = 40 + 2\n  answer\n}\n"
    );
}

#[test]
fn fmt_check_reports_changes_without_mutating_the_file() {
    let source = "main=()->i64{\nanswer=40+2\nanswer\n}\n";
    let file = TempFerFile::new(source);
    let output = run_fmt(&file, true);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("would reformat"));
    assert_eq!(file.read(), source);

    fs::write(
        file.as_path(),
        "main = () -> i64 {\n  answer = 40 + 2\n  answer\n}\n",
    )
    .expect("formatted fixture must write");
    let output = run_fmt(&file, true);
    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn fmt_rejects_invalid_source_without_mutating_the_file() {
    let source = "main=()->i64{\n";
    let file = TempFerFile::new(source);
    let output = run_fmt(&file, false);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot format"));
    assert_eq!(file.read(), source);
}

#[test]
fn fmt_reports_missing_files_at_the_cli_boundary() {
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "fer-cli-missing-{}-{}.fer",
        std::process::id(),
        sequence
    ));
    let _ = fs::remove_file(&path);
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args(["fmt", path.to_str().expect("temporary path must be UTF-8")])
        .output()
        .expect("fer binary must start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot read"));
}

#[test]
fn fmt_requires_a_file_operand() {
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args(["fmt"])
        .output()
        .expect("fer binary must start");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fer fmt [--check]"));
}

#[test]
fn fmt_check_accepts_check_flag_after_the_file_operand() {
    let file = TempFerFile::new("main = () -> i64 {\n  42\n}\n");
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args([
            "fmt",
            file.as_path()
                .to_str()
                .expect("temporary path must be UTF-8"),
            "--check",
        ])
        .output()
        .expect("fer binary must start");

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
}

#[cfg(unix)]
#[test]
fn fmt_rejects_read_only_files_without_mutating_the_file() {
    let source = "main=()->i64{\nanswer=40+2\nanswer\n}\n";
    let file = TempFerFile::new(source);
    let mut permissions = fs::metadata(file.as_path())
        .expect("temporary Fer fixture metadata must read")
        .permissions();
    let original_mode = permissions.mode();
    permissions.set_mode(original_mode & !0o222);
    fs::set_permissions(file.as_path(), permissions).expect("fixture must become read-only");

    let output = run_fmt(&file, false);

    let mut writable_permissions = fs::metadata(file.as_path())
        .expect("temporary Fer fixture metadata must read")
        .permissions();
    writable_permissions.set_mode(original_mode);
    fs::set_permissions(file.as_path(), writable_permissions)
        .expect("fixture permissions must be restored");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot write"));
    assert_eq!(file.read(), source);
}
