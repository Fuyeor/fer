// compiler-rs/fer/src/main.rs

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use diagnostics::RenderedDiagnostic;
use fer::{DriverError, render_diagnostics, run_source};
use runtime::Value;
use syntax::format_source;

const USAGE: &str = "usage: fer run <file.fer>\n       fer fmt [--check] <file.fer>";
const FMT_CHECK_MESSAGE: &str = "would reformat";
const TEMP_FILE_ATTEMPTS: usize = 32;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
enum Command {
    Run(PathBuf),
    Fmt { path: PathBuf, check: bool },
}

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let exit_code = match parse_arguments(&arguments) {
        Ok(Command::Run(path)) => run_command(&path),
        Ok(Command::Fmt { path, check }) => format_command(&path, check),
        Err(message) => {
            eprintln!("{message}");
            2
        }
    };
    if exit_code != 0 {
        process::exit(exit_code);
    }
}

/// Parse the small, dependency-free command-line grammar at the CLI boundary.
fn parse_arguments(arguments: &[String]) -> Result<Command, &'static str> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(USAGE);
    };
    match command {
        "run" if arguments.len() == 2 => Ok(Command::Run(PathBuf::from(&arguments[1]))),
        "run" => Err(USAGE),
        "fmt" => parse_fmt_arguments(arguments),
        _ => Err(USAGE),
    }
}

/// Parse `fmt` flags without accepting ambiguous or silently ignored operands.
fn parse_fmt_arguments(arguments: &[String]) -> Result<Command, &'static str> {
    let mut check = false;
    let mut path = None;
    for argument in arguments.iter().skip(1) {
        if argument == "--check" {
            if check {
                return Err(USAGE);
            }
            check = true;
            continue;
        }
        if argument.starts_with('-') || path.is_some() {
            return Err(USAGE);
        }
        path = Some(PathBuf::from(argument));
    }
    path.map_or(Err(USAGE), |path| Ok(Command::Fmt { path, check }))
}

/// Read, format, and execute the existing Fer runtime command.
fn run_command(path: &Path) -> i32 {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("fer: cannot read {}: {error}", path.display());
            return 1;
        }
    };
    let virtual_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("main.fer");
    let locale = env::var("FER_LOCALE").unwrap_or_else(|_| "en".to_owned());

    match run_source(virtual_name, &source) {
        Ok(report) => {
            for line in report.output {
                println!("{line}");
            }
            if report.result != Value::Unit {
                println!("{}", report.result);
            }
            0
        }
        Err(error) => {
            let diagnostics = match error {
                DriverError::Diagnostics(diagnostics) => diagnostics,
                DriverError::Runtime(error) => {
                    eprintln!("fer: runtime error: {error}");
                    return 1;
                }
                DriverError::InvalidPath => {
                    eprintln!("fer: source path is not a valid virtual Fer path");
                    return 2;
                }
            };
            match render_diagnostics(&diagnostics, &locale) {
                Ok(rendered) => print_diagnostics(virtual_name, &source, &rendered),
                Err(error) => {
                    eprintln!("fer: cannot render diagnostics: {error:?}");
                    return 1;
                }
            }
            1
        }
    }
}

/// Format a file in place or report whether it would change under `--check`.
fn format_command(path: &Path, check: bool) -> i32 {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("fer: cannot read {}: {error}", path.display());
            return 1;
        }
    };
    let formatted = match format_source(&source) {
        Ok(formatted) => formatted,
        Err(error) => {
            eprintln!("fer: cannot format {}: {error:?}", path.display());
            return 1;
        }
    };
    if formatted == source {
        return 0;
    }
    if check {
        eprintln!("fer: {} {FMT_CHECK_MESSAGE}", path.display());
        return 1;
    }
    match write_file_atomically(path, &formatted) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("fer: cannot write {}: {error}", path.display());
            1
        }
    }
}

/// Replace a regular file through a same-directory temporary file and atomic rename.
fn write_file_atomically(path: &Path, contents: &str) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "formatter requires a regular file",
        ));
    }
    if metadata.permissions().readonly() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "source file is read-only",
        ));
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "source path has no file name")
    })?;
    let (temporary_path, mut temporary) = create_temporary_file(parent, file_name)?;
    let result = (|| {
        temporary.write_all(contents.as_bytes())?;
        temporary.sync_all()?;
        fs::set_permissions(&temporary_path, metadata.permissions())?;
        drop(temporary);
        fs::rename(&temporary_path, path)
    })();
    let _ = fs::remove_file(&temporary_path);
    result
}

/// Create a same-directory temporary file without ever replacing an existing path.
fn create_temporary_file(parent: &Path, file_name: &OsStr) -> io::Result<(PathBuf, File)> {
    for _ in 0..TEMP_FILE_ATTEMPTS {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(file_name);
        temporary_name.push(format!(".fer-fmt-{}-{}.tmp", process::id(), sequence));
        let temporary_path = parent.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique formatter temporary file",
    ))
}

fn print_diagnostics(path: &str, source: &str, diagnostics: &[RenderedDiagnostic]) {
    for diagnostic in diagnostics {
        let (line, column) = line_column(source, diagnostic.primary.start);
        eprintln!(
            "{path}:{line}:{column}: {:?}[{}]: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
        for label in &diagnostic.labels {
            let (label_line, label_column) = line_column(source, label.span.start);
            eprintln!("  {path}:{label_line}:{label_column}: {}", label.message);
        }
        for note in &diagnostic.notes {
            eprintln!("  note: {note}");
        }
        for suggestion in &diagnostic.suggestions {
            eprintln!(
                "  suggestion: replace {}..{} with `{}`",
                suggestion.span.start, suggestion.span.end, suggestion.replacement
            );
        }
    }
}

fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = source.get(..byte_offset).unwrap_or(source);
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rfind('\n')
        .map_or(prefix.len(), |index| prefix.len() - index - 1);
    (line, column + 1)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Command, parse_arguments};

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_run_command() {
        assert!(matches!(
            parse_arguments(&arguments(&["run", "main.fer"])),
            Ok(Command::Run(path)) if path == Path::new("main.fer")
        ));
    }

    #[test]
    fn parses_fmt_check_before_or_after_path() {
        for values in [
            ["fmt", "--check", "main.fer"],
            ["fmt", "main.fer", "--check"],
        ] {
            assert!(matches!(
                parse_arguments(&arguments(&values)),
                Ok(Command::Fmt { path, check }) if path == Path::new("main.fer") && check
            ));
        }
    }

    #[test]
    fn rejects_missing_or_ambiguous_fmt_operands() {
        for values in [
            vec!["fmt"],
            vec!["fmt", "--check", "--check", "main.fer"],
            vec!["fmt", "main.fer", "other.fer"],
            vec!["fmt", "--unknown", "main.fer"],
        ] {
            assert!(parse_arguments(&arguments(&values)).is_err());
        }
    }
}
