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

struct TempFerWorkspace {
    path: PathBuf,
}

impl TempFerWorkspace {
    /// Create an isolated workspace fixture with nested source files.
    fn new() -> Self {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fer-workspace-cli-{}-{}",
            std::process::id(),
            sequence
        ));
        fs::create_dir_all(&path).expect("temporary workspace must create");
        Self { path }
    }

    fn write(&self, relative_path: &str, source: &str) -> PathBuf {
        let path = self.path.join(relative_path);
        fs::create_dir_all(path.parent().expect("fixture path must have a parent"))
            .expect("temporary workspace parent must create");
        fs::write(&path, source).expect("temporary workspace file must write");
        path
    }

    fn read(&self, relative_path: &str) -> String {
        fs::read_to_string(self.path.join(relative_path))
            .expect("temporary workspace file must read")
    }
}

impl Drop for TempFerWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Execute workspace formatting against an isolated directory.
fn run_fmt_workspace(workspace: &TempFerWorkspace, check: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fer"));
    command.args(["fmt", "--workspace"]);
    if check {
        command.arg("--check");
    }
    command.arg(workspace.path.as_path());
    command.output().expect("fer binary must start")
}

#[test]
fn fmt_workspace_skips_legacy_ferry_without_mutating_it() {
    let workspace = TempFerWorkspace::new();
    let legacy = "name = @fer/compiler\nversion = 0.0.0\nlicense = .mit\n";
    workspace.write("compiler/ferry.fer", legacy);
    workspace.write("src/main.fer", "answer=40+2\n");

    let output = run_fmt_workspace(&workspace, false);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(workspace.read("compiler/ferry.fer"), legacy);
    assert_eq!(workspace.read("src/main.fer"), "answer = 40 + 2\n");
}

#[test]
fn fmt_rejects_legacy_ferry_as_a_single_file() {
    let workspace = TempFerWorkspace::new();
    let legacy = "name = @fer/compiler\nversion = 0.0.0\nlicense = .mit\n";
    let path = workspace.write("compiler/ferry.fer", legacy);
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args(["fmt", path.to_str().expect("legacy path must be UTF-8")])
        .output()
        .expect("fer binary must start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("legacy compiler/ferry.fer"));
    assert_eq!(workspace.read("compiler/ferry.fer"), legacy);
}

#[test]
fn fmt_workspace_rewrites_nested_fer_and_fon_files() {
    let workspace = TempFerWorkspace::new();
    workspace.write("src/main.fer", "main=()->i64{\nanswer=40+2\nanswer\n}\n");
    workspace.write(
        "locale/locales.fon",
        "analysis={\nundefined-name={\nen=`cannot resolve {name}`\n}\n}\n",
    );
    workspace.write("target/generated.fer", "main=()->i64{42}\n");
    workspace.write("README.txt", "not a source file\n");

    let output = run_fmt_workspace(&workspace, false);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(
        workspace.read("src/main.fer"),
        "main = () -> i64 {\n  answer = 40 + 2\n  answer\n}\n"
    );
    assert_eq!(
        workspace.read("locale/locales.fon"),
        "analysis = {\n  undefined-name = {\n    en = `cannot resolve {name}`\n  }\n}\n"
    );
    assert_eq!(workspace.read("target/generated.fer"), "main=()->i64{42}\n");

    let check = run_fmt_workspace(&workspace, true);
    assert!(check.status.success(), "stderr: {:?}", check.stderr);
}

#[test]
fn fmt_workspace_check_reports_changes_without_mutating_files() {
    let workspace = TempFerWorkspace::new();
    let fer_source = "main=()->i64{\nanswer=40+2\nanswer\n}\n";
    let fon_source = "name=`Fuyeor`\n";
    workspace.write("main.fer", fer_source);
    workspace.write("manifest.fon", fon_source);

    let output = run_fmt_workspace(&workspace, true);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("main.fer"));
    assert!(stderr.contains("manifest.fon"));
    assert_eq!(workspace.read("main.fer"), fer_source);
    assert_eq!(workspace.read("manifest.fon"), fon_source);
}

#[test]
fn fmt_workspace_validates_all_files_before_writing_any_file() {
    let workspace = TempFerWorkspace::new();
    let valid_path = workspace.write("valid.fer", "main=()->i64{\nanswer=40+2\nanswer\n}\n");
    workspace.write("invalid.fer", "main=()->i64{\n");
    workspace.write("invalid.fon", "name =\n");

    let output = run_fmt_workspace(&workspace, false);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot format"));
    assert_eq!(
        fs::read_to_string(valid_path).expect("valid source must remain readable"),
        "main=()->i64{\nanswer=40+2\nanswer\n}\n"
    );
}

#[test]
fn fmt_workspace_requires_a_directory_root() {
    let file = TempFerFile::new("name = `Fuyeor`\n");
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args([
            "fmt",
            "--workspace",
            file.as_path()
                .to_str()
                .expect("temporary path must be UTF-8"),
        ])
        .output()
        .expect("fer binary must start");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("workspace root"));
}

#[cfg(unix)]
#[test]
fn fmt_workspace_stages_all_files_before_replacing_any_file() {
    let workspace = TempFerWorkspace::new();
    let first_source = "main=()->i64{\nanswer=40+2\nanswer\n}\n";
    let second_source = "main=()->i64{\n42\n}\n";
    workspace.write("a.fer", first_source);
    let second_path = workspace.write("b.fer", second_source);
    let mut permissions = fs::metadata(&second_path)
        .expect("workspace fixture metadata must read")
        .permissions();
    let original_mode = permissions.mode();
    permissions.set_mode(original_mode & !0o222);
    fs::set_permissions(&second_path, permissions).expect("fixture must become read-only");

    let output = run_fmt_workspace(&workspace, false);

    let mut writable_permissions = fs::metadata(&second_path)
        .expect("workspace fixture metadata must read")
        .permissions();
    writable_permissions.set_mode(original_mode);
    fs::set_permissions(&second_path, writable_permissions)
        .expect("fixture permissions must be restored");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot write workspace"));
    assert_eq!(workspace.read("a.fer"), first_source);
    assert_eq!(workspace.read("b.fer"), second_source);
}

#[test]
fn fmt_workspace_check_reports_repository_source_differences_without_errors() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_fer"))
        .args([
            "fmt",
            "--workspace",
            "--check",
            repository.to_str().expect("repository path must be UTF-8"),
        ])
        .output()
        .expect("fer binary must start");

    assert!(
        !output.status.success(),
        "d11 examples are source-preserved fixtures, not canonical fixtures"
    );
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("condition.fer"), "stderr: {stderr}");
    assert!(
        !stderr.contains("ferry.fer"),
        "legacy ferry must be excluded: {stderr}"
    );
    assert!(
        !stderr.contains("cannot "),
        "workspace check must not fail validation: {stderr}"
    );
}
