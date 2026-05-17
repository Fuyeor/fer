The `syntax` crate provides the lexer and parser for the Fer programming language. It transforms source text into a Lossless Concrete Syntax Tree (CST) that preserves every byte of the original file (whitespace, comments, etc.) and is suitable for formatting, migration, and IDE tooling.

## Design

- **Zero magic** – no parser generators, no macros.  Every token and tree
  node is explicit.
- **Lossless CST** – all trivia is retained via token spans; the tree
  records the exact source range of every construct.
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
  lex.rs       – Lexer (mode stack for strings, comment skipping, interned identifiers)
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
- Structs and enums are assigned with `=`:
  `Point = struct { x = i32 y = i32 }`.
- Imports use `{ names } = @scope/pkg`; exports use `exports { names }`.
- String literals use backticks and support multi-line with auto-dedent.
- No `==`, `!=`, `&&`, `||`, `!` – the language uses English keywords
  `equals`, `not`, `and`, `or`.

## Current limitations

- String interpolation (`` `Hello {name}` ``) is not yet implemented;
  currently only simple strings are parsed.
- Match expressions (`value { pattern => body }`) are not yet implemented.
- The path comment (`/// @/...`) is not extracted and stored in CST.
- Index expressions (`a[i]`) produce a placeholder error; the node kind is
  defined but not built by the parser.
- Error recovery is basic; synchronization token sets may be incomplete.
- Semantic restrictions (e.g., ≥2 function arguments must be named) are
  not enforced in the parser – they will be checked in the `analysis` layer.

## Testing

Tests are split by component:

- `tests/lex_tests.rs` – token recognition
- `tests/parse_expr_tests.rs` – expression parsing
- `tests/parse_stmt_tests.rs` – statements and declarations
- `tests/parse_module_tests.rs` – imports and exports
- `tests/cst_tests.rs` – (placeholder for tree structure tests)

Run with `cargo test -p syntax`.

## Future work

- Integrate with the `query` incremental database: register `parse_file`
  as a cached query.
- Implement the `migrate` and `fmt` transforms on top of the lossless CST.
- Enhance the lexer/parser to support full string interpolation.
- Complete pattern parsing for match arms.