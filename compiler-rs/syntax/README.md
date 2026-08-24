The `syntax` crate provides the lexer and parser for the Fer programming language. It transforms source text into a source-mapped Concrete Syntax Tree (CST) whose nodes retain exact source spans and are suitable for formatting, migration, and IDE tooling.

## Design

- **Zero magic** – no parser generators, no macros.  Every token and tree
  node is explicit.
- **Source-mapped CST** – trivia is skipped by the lexer while the tree
  records the exact source range of every parsed construct.
- **Indexed storage** – tree nodes are stored in a flat `Vec`, referenced by
  `NodeId(u32)`.  No recursive pointers, easy to serialize, cache-friendly.
- **Pratt parsing** – expressions are parsed with operator precedence and
  associativity defined in `grammar.rs`.
- **Recursive descent** – statements, declarations, and modules are parsed
  top-down with one-token lookahead and checkpoint-based backtracking.
- **Error recovery** – the parser attempts to continue after the first
  error, emitting diagnostics and inserting placeholder nodes.

## Module structure

```
syntax/
  grammar.rs   – TokenKind enum, keyword table, precedence table
  lex.rs       – Lexer (backtick strings, comments, regex mode, interned identifiers)
  cst.rs       – CST node kinds and helper types (ChainExpr, NamedArg, etc.)
  parse/
    mod.rs     – Parser context, token stream, backtracking
    error.rs   – error reporting and recovery
    expr.rs    – Pratt expression parser (atoms, unary, binary, calls, chains)
    stmt.rs    – statement and declaration parser (function, struct, enum, const)
    module.rs  – import and export parser
    pattern.rs – pattern parser (match arms, destructuring) [not yet implemented]
```

## Compliance with Fer draft v0.0.11

- Identifiers use kebab-case; struct/enum names must be Pascal-kebab-case
  (enforced by semantic analysis).
- Functions are defined without a `function` keyword:
  `my-func(x: i32, y: i32) -> i32 { x + y }`.
- Structs and enums are assigned with `=`. Struct fields support inferred
  defaults (``name = `guest``), explicit typed defaults
  (`age: i32 = 18`), and required fields (`id: i32`).
- Declarations and struct fields support `#[name]` annotations. Annotation
  arguments may be positional or named, for example
  ``#[derive = `Debug`, mode = stable]``.
- Imports use `{ names } = @scope/pkg`; exports use `exports { names }`.
- String literals use backticks and support multi-line with auto-dedent.
- No `==`, `!=`, `&&`, `||`, `!` – the language uses English keywords
  `equals`, `not`, `and`, `or`.

## Current limitations

- String interpolation (`` `Hello {name}` ``) is not yet implemented;
  currently only simple strings are parsed.
- Match expressions are parsed into `MatchExpr` and `MatchArm` nodes, but
  semantic validation is deferred to later compiler layers.
- The path comment (`/// @/...`) is not extracted and stored in CST.
- Import/export annotations are rejected explicitly; annotations currently
  target declarations and struct fields.
- Error recovery is basic; synchronization token sets may be incomplete.
- Semantic restrictions (e.g., ≥2 function arguments must be named) are
  not enforced in the parser – they will be checked in the `analysis` layer.

## Testing

Tests are split by component:

- Inline lexer tests in `src/lex.rs` – token recognition
- `tests/parse_expr_tests.rs` – expression parsing
- `tests/parse_stmt_tests.rs` – statements and declarations
- `tests/parse_module_tests.rs` – imports and exports
- CST assertions are included with the corresponding parser tests.

Run with `cargo test -p syntax`.

## Future work

- Integrate with the `query` incremental database: register `parse_file`
  as a cached query.
- Implement the `migrate` and `fmt` transforms on top of the source-mapped CST.
- Enhance the lexer/parser to support full string interpolation.
- Complete pattern parsing for match arms.