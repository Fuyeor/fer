// compiler-rs/fer-lsp/src/formatting.rs

use syntax::{FormatError, FormatOptions, format_source_with_options};
use tower_lsp_server::ls_types::{FormattingOptions, TextEdit};
use vfs::LineIndex;

use crate::position::to_lsp_range;

/// Format one open Fer document and return a complete-document edit when changed.
pub(crate) fn format_document(
    source: &str,
    options: &FormattingOptions,
) -> Result<Option<Vec<TextEdit>>, FormatError> {
    let formatted = format_source_with_options(
        source,
        FormatOptions {
            indent_width: options.tab_size as usize,
            use_tabs: !options.insert_spaces,
        },
    )?;
    if formatted == source {
        return Ok(None);
    }

    let line_index = LineIndex::new(source);
    let full_range = to_lsp_range(line_index.range(infra::Span::new(0, source.len())));
    Ok(Some(vec![TextEdit::new(full_range, formatted)]))
}
