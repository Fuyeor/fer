// compiler-rs/fer/src/main.rs

use std::env;
use std::fs;
use std::path::Path;

use diagnostics::RenderedDiagnostic;
use fer::{DriverError, render_diagnostics, run_source};
use runtime::Value;

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 || arguments[0] != "run" {
        eprintln!("usage: fer run <file.fer>");
        std::process::exit(2);
    }

    let path = Path::new(&arguments[1]);
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("fer: cannot read {}: {error}", path.display());
            std::process::exit(1);
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
            println!("{}", display_value(&report.result));
        }
        Err(error) => {
            let diagnostics = match error {
                DriverError::Diagnostics(diagnostics) => diagnostics,
                DriverError::Runtime(error) => {
                    eprintln!("fer: runtime error: {error}");
                    std::process::exit(1);
                }
                DriverError::InvalidPath => {
                    eprintln!("fer: source path is not a valid virtual Fer path");
                    std::process::exit(2);
                }
            };
            match render_diagnostics(&diagnostics, &locale) {
                Ok(rendered) => print_diagnostics(virtual_name, &source, &rendered),
                Err(error) => {
                    eprintln!("fer: cannot render diagnostics: {error:?}");
                    std::process::exit(1);
                }
            }
            std::process::exit(1);
        }
    }
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

fn display_value(value: &Value) -> String {
    match value {
        Value::Unit => "()".to_owned(),
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Char(value) => value.clone(),
        Value::Regex(value) => value.clone(),
        Value::Function(item) => format!("<function:{}>", item.index()),
    }
}
