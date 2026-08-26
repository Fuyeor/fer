<!-- compiler-rs/fon/README.md -->

# Fer FON adapter

The `fon` crate is Fer's integration boundary for the independent [`fon-parser`](https://github.com/Fuyeor/fon-parser) crate. It keeps FON parsing independent from Fer compiler state while providing explicit adapters for Fer `FileId`, `SourceMap`, `Span`, `DiagnosticBag`, schemes, and queries.

## Integration boundary

`parse_source` parses an in-memory source with an owning Fer `FileId`. `parse_file` reads content already owned by a Fer `SourceMap`; it does not perform filesystem or network I/O. `report_diagnostics` converts parser diagnostics to Fer's diagnostic model without leaking dynamic strings. `resolve_source` delegates semantic interpretation to a caller-provided `fon_parser::SchemeResolver`.

The query integration exposes `FON_SOURCE_QUERY` and `FON_PARSE_QUERY`. Call `register_queries` once for a database, then call `set_source_file` whenever the input file changes. The parse query declares its dependency on the source input, so the existing query database invalidates cached parse results correctly.

```rust
use fon::{register_queries, set_source_file, FON_PARSE_QUERY};
use infra::Interner;
use query::Database;
use vfs::SourceMap;

let mut source_map = SourceMap::new();
let file_id = source_map
    .add_file("manifest.fer", "name = `Fuyeor`\n".into())
    .expect("valid source path");
let mut database = Database::new(source_map, Interner::new(), 2);
register_queries(&mut database);
set_source_file(&database, file_id);
let parsed: fon::ParsedFonSource = database.query(FON_PARSE_QUERY);
```

## Lossless formatting

`format_source` validates FON with the independent parser, then rewrites only canonical horizontal spacing and code indentation from source-backed token/trivia ranges. Comments, backtick strings, regular expressions, CRLF line endings, and all non-horizontal trivia remain source-owned. Invalid parser diagnostics and error tokens fail fast before a rewrite is returned. The `fer` CLI routes `.fon` files to this API and uses the Fer syntax formatter for `.fer` files.

The adapter does not interpret Webroamer behavior, execute interpolation, or hard-code Fer schemes. Those decisions remain in the caller's scheme and lowering layers.
