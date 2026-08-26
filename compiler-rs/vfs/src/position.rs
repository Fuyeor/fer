// compiler-rs/vfs/src/position.rs

use infra::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: u32,
    pub utf16_column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

/// Converts source byte offsets into stable zero-based source positions.
#[derive(Debug, Clone)]
pub struct LineIndex {
    source: String,
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Builds a line index from one complete source snapshot.
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        let bytes = source.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => {
                    line_starts.push(index + 1);
                    index += 1;
                }
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    line_starts.push(index + 2);
                    index += 2;
                }
                _ => index += 1,
            }
        }
        Self {
            source: source.to_owned(),
            line_starts,
        }
    }

    /// Converts a UTF-8 byte offset to a zero-based UTF-16 source position.
    pub fn position(&self, byte_offset: usize) -> SourcePosition {
        let mut offset = byte_offset.min(self.source.len());
        while offset > 0 && !self.source.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self
            .line_starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line];
        let column_end = offset.min(self.line_content_end(line));
        let prefix = &self.source[line_start..column_end];
        SourcePosition {
            line: line as u32,
            utf16_column: prefix.encode_utf16().count() as u32,
        }
    }

    /// Converts a compiler span to a clamped zero-based source range.
    pub fn range(&self, span: Span) -> SourceRange {
        SourceRange {
            start: self.position(span.start),
            end: self.position(span.end),
        }
    }

    fn line_content_end(&self, line: usize) -> usize {
        let next_start = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.source.len());
        let segment = &self.source[self.line_starts[line]..next_start];
        let newline_length = if segment.ends_with("\r\n") {
            2
        } else if segment.ends_with(['\n', '\r']) {
            1
        } else {
            0
        };
        next_start - newline_length
    }
}

#[cfg(test)]
mod tests {
    use super::LineIndex;
    use infra::Span;

    #[test]
    fn converts_ascii_and_crlf_positions() {
        let index = LineIndex::new("alpha\r\nbeta");

        assert_eq!(index.position(0).line, 0);
        assert_eq!(index.position(5).utf16_column, 5);
        assert_eq!(index.position(7).line, 1);
        assert_eq!(index.position(7).utf16_column, 0);
        assert_eq!(index.position(11).utf16_column, 4);
    }

    #[test]
    fn counts_utf16_code_units_for_unicode() {
        let index = LineIndex::new("中文😀x");

        assert_eq!(index.position("中文".len()).utf16_column, 2);
        assert_eq!(index.position("中文😀".len()).utf16_column, 4);
        assert_eq!(index.position("中文😀x".len()).utf16_column, 5);
    }

    #[test]
    fn clamps_out_of_bounds_and_invalid_utf8_offsets() {
        let index = LineIndex::new("😀value");
        let range = index.range(Span::new(1, 100));

        assert_eq!(range.start.utf16_column, 0);
        assert_eq!(range.end.utf16_column, 7);
        assert_eq!(index.position(2).utf16_column, 0);
    }
}
