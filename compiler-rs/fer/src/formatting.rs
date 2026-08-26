// compiler-rs/fer/src/formatting.rs

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use fon::{FormatError as FonFormatError, format_source as format_fon_source};
use syntax::{FormatError as FerFormatError, format_source as format_fer_source};

const TEMP_FILE_ATTEMPTS: usize = 32;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

/// Format one source file in place without invoking Fer analysis or runtime.
pub(crate) fn format_file(path: &Path, check: bool) -> i32 {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("fer: cannot read {}: {error}", path.display());
            return 1;
        }
    };
    let formatted = match format_source_for_path(path, &source) {
        Ok(formatted) => formatted,
        Err(error) => {
            eprintln!("fer: cannot format {}: {error}", path.display());
            return 1;
        }
    };
    if formatted == source {
        return 0;
    }
    if check {
        eprintln!("fer: {} would reformat", path.display());
        return 1;
    }
    let pending = PendingFile {
        path: path.to_owned(),
        contents: formatted,
    };
    match write_files_atomically(std::slice::from_ref(&pending)) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("fer: cannot write {}: {error}", path.display());
            1
        }
    }
}

/// Format every supported source below a workspace after validating all files first.
pub(crate) fn format_workspace(root: &Path, check: bool) -> i32 {
    let paths = match discover_source_files(root) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("fer: cannot scan workspace {}: {error}", root.display());
            return 1;
        }
    };
    let mut changed_files = Vec::new();

    for path in paths {
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("fer: cannot read {}: {error}", path.display());
                return 1;
            }
        };
        let formatted = match format_source_for_path(&path, &source) {
            Ok(formatted) => formatted,
            Err(error) => {
                eprintln!("fer: cannot format {}: {error}", path.display());
                return 1;
            }
        };
        if formatted != source {
            changed_files.push(PendingFile {
                path,
                contents: formatted,
            });
        }
    }

    if check {
        for file in &changed_files {
            eprintln!("fer: {} would reformat", file.path.display());
        }
        return i32::from(!changed_files.is_empty());
    }
    if changed_files.is_empty() {
        return 0;
    }
    match write_files_atomically(&changed_files) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("fer: cannot write workspace {}: {error}", root.display());
            1
        }
    }
}

/// Select the independent FON formatter for `.fon` and the Fer formatter otherwise.
fn format_source_for_path(path: &Path, source: &str) -> Result<String, String> {
    if path.extension().and_then(OsStr::to_str) == Some("fon") {
        format_fon_source(source).map_err(format_fon_error)
    } else {
        format_fer_source(source).map_err(format_fer_error)
    }
}

fn format_fer_error(error: FerFormatError) -> String {
    format!("{error:?}")
}

fn format_fon_error(error: FonFormatError) -> String {
    format!("{error:?}")
}

#[derive(Debug)]
struct PendingFile {
    path: PathBuf,
    contents: String,
}

#[derive(Debug)]
struct StagedFile {
    path: PathBuf,
    temporary_path: PathBuf,
}

/// Discover regular `.fer` and `.fon` files while excluding generated/dependency trees.
fn discover_source_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "workspace root must be a directory",
        ));
    }

    let mut directories = vec![root.to_owned()];
    let mut files = Vec::new();
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.file_type().is_dir() {
                if !is_excluded_directory(&path) {
                    directories.push(path);
                }
                continue;
            }
            if metadata.file_type().is_file() && is_source_file(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("fer" | "fon")
    )
}

fn is_excluded_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| matches!(name, ".git" | "node_modules" | "target"))
}

/// Stage every replacement before renaming any target file.
fn write_files_atomically(files: &[PendingFile]) -> io::Result<()> {
    let mut staged_files = Vec::with_capacity(files.len());
    for file in files {
        match stage_file(file) {
            Ok(staged) => staged_files.push(staged),
            Err(error) => {
                cleanup_staged_files(&staged_files);
                return Err(error);
            }
        }
    }

    let result = staged_files
        .iter()
        .try_for_each(|staged| fs::rename(&staged.temporary_path, &staged.path));
    cleanup_staged_files(&staged_files);
    result
}

/// Write one same-directory temporary file with preserved permissions and durability.
fn stage_file(file: &PendingFile) -> io::Result<StagedFile> {
    let metadata = regular_file_metadata(&file.path)?;
    if metadata.permissions().readonly() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "source file is read-only",
        ));
    }
    let parent = file.path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = file.path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "source path has no file name")
    })?;
    let (temporary_path, mut temporary) = create_temporary_file(parent, file_name)?;
    let cleanup_path = temporary_path.clone();
    let result = (|| {
        temporary.write_all(file.contents.as_bytes())?;
        temporary.sync_all()?;
        fs::set_permissions(&temporary_path, metadata.permissions())?;
        drop(temporary);
        Ok(StagedFile {
            path: file.path.clone(),
            temporary_path,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(cleanup_path);
    }
    result
}

fn regular_file_metadata(path: &Path) -> io::Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "formatter requires a regular file",
        ));
    }
    Ok(metadata)
}

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

fn cleanup_staged_files(staged_files: &[StagedFile]) {
    for staged in staged_files {
        let _ = fs::remove_file(&staged.temporary_path);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{is_excluded_directory, is_source_file};

    #[test]
    fn recognizes_supported_source_extensions() {
        assert!(is_source_file(Path::new("main.fer")));
        assert!(is_source_file(Path::new("locale.fon")));
        assert!(!is_source_file(Path::new("README.md")));
    }

    #[test]
    fn excludes_generated_and_dependency_directories() {
        assert!(is_excluded_directory(Path::new("target")));
        assert!(is_excluded_directory(Path::new("node_modules")));
        assert!(is_excluded_directory(Path::new(".git")));
        assert!(!is_excluded_directory(Path::new("src")));
    }
}
