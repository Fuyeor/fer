// compiler-rs/fer/src/lib.rs

use std::sync::Arc;

use diagnostics::{Catalog, CatalogError, Locale, RenderError, RenderedDiagnostic};
use infra::{Diagnostic, DiagnosticBag, DiagnosticValue, Interner, MessageId};
use ir::hir::{HirNode, ItemKind};
use ir::lowering::CstFile;
use query::Database;
use runtime::{ExecutionReport, RuntimeError};
use syntax::{Lexer, Parser};
use vfs::FileId;

/// Errors reported by the Fer driver before or during execution.
#[derive(Debug)]
pub enum DriverError {
    InvalidPath,
    Diagnostics(Vec<Diagnostic>),
    Runtime(RuntimeError),
}

/// Run one source string through parsing, lowering, analysis, and runtime.
pub fn run_source(path: &str, source_text: &str) -> Result<ExecutionReport, DriverError> {
    let mut source_map = vfs::SourceMap::new();
    let file_id = source_map
        .add_file(path, source_text.to_owned())
        .ok_or(DriverError::InvalidPath)?;
    let source: Arc<str> = Arc::from(source_text);
    let cst = parse_cst(file_id, source.clone())?;

    let mut database = Database::new(source_map, Interner::new(), 5);
    ir::register_queries(&mut database);
    analysis::register_queries(&mut database);
    analysis::set_cst_file(&database, cst);

    let hir: ir::HirFile = database.query(ir::LOWER_HIR_QUERY);
    if has_errors(&hir.diagnostics) {
        return Err(DriverError::Diagnostics(hir.diagnostics));
    }
    let resolution: analysis::ResolutionTable = database.query(analysis::RESOLVE_NAMES_QUERY);
    if has_errors(&resolution.diagnostics) {
        return Err(DriverError::Diagnostics(resolution.diagnostics));
    }
    let types: analysis::TypeTable = database.query(analysis::TYPE_ANALYSIS_QUERY);
    if has_errors(&types.diagnostics) {
        return Err(DriverError::Diagnostics(types.diagnostics));
    }

    let mut interpreter = runtime::Interpreter::new(&hir, &resolution);
    match find_main_function(&hir, source.as_ref()) {
        Some(item_id) => {
            interpreter
                .run_function(item_id, Vec::new())
                .map(|result| ExecutionReport {
                    result,
                    output: Vec::new(),
                })
        }
        None => interpreter.run(),
    }
    .map_err(DriverError::Runtime)
}

/// Render structured diagnostics through the requested locale catalog.
pub fn render_diagnostics(
    diagnostics: &[Diagnostic],
    locale: &str,
) -> Result<Vec<RenderedDiagnostic>, DriverRenderError> {
    let catalog = Catalog::embedded().map_err(DriverRenderError::Catalog)?;
    diagnostics
        .iter()
        .map(|diagnostic| {
            catalog
                .render(diagnostic, Locale::new(locale))
                .map_err(DriverRenderError::Render)
        })
        .collect()
}

/// Errors raised while converting structured diagnostics into localized output.
#[derive(Debug)]
pub enum DriverRenderError {
    Catalog(CatalogError),
    Render(RenderError),
}

fn parse_cst(file_id: FileId, source: Arc<str>) -> Result<CstFile, DriverError> {
    let mut interner = Interner::new();
    let lexer = Lexer::new(source.as_ref(), &mut interner);
    let mut nodes = Vec::new();
    let mut diagnostics = DiagnosticBag::new();
    let mut parser = Parser::new(lexer, &mut nodes, &mut diagnostics, file_id);
    let root = parser.parse_file().map_err(|error| {
        DriverError::Diagnostics(vec![
            Diagnostic::error(
                "parse-error",
                MessageId::new("syntax.parse-error"),
                error.span,
            )
            .with_arg("message", DiagnosticValue::Text(error.message)),
        ])
    })?;
    let parser_diagnostics = diagnostics.into_diagnostics();
    if has_errors(&parser_diagnostics) {
        return Err(DriverError::Diagnostics(parser_diagnostics));
    }
    Ok(CstFile {
        file_id,
        source,
        root,
        nodes,
    })
}

fn find_main_function(hir: &ir::HirFile, source: &str) -> Option<ir::hir::HirId> {
    hir.items.iter().copied().find(|item_id| {
        let Some(HirNode::Item(item)) = hir.arena.node(*item_id) else {
            return false;
        };
        let ItemKind::Function(function) = &item.kind else {
            return false;
        };
        source
            .get(function.name.span.start..function.name.span.end)
            .is_some_and(|name| name == "main")
    })
}

fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == infra::Severity::Error)
}
