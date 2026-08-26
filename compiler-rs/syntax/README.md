The `syntax` crate provides the lexer and parser for the Fer programming language. It transforms source text into a source-mapped Concrete Syntax Tree (CST), and exposes a separate lossless token stream plus conservative formatter for source-preserving IDE edits.

## Design

- **Zero magic** – no parser generators, no macros.  Every token and tree
  node is explicit.
- **Source-mapped CST** – the semantic parser records exact source ranges for
  parsed constructs, while `LosslessTokenStream` retains every source gap,
  comment, and original token spelling for source-preserving tools.
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
  lossless.rs  – Source-owned token stream with exact trivia spans
  formatter.rs – Conservative lossless indentation formatter
  cst.rs       – CST node kinds and helper types (ChainExpr, NamedArg, etc.)
  parse/
    mod.rs     – Parser context, token stream, backtracking
    error.rs   – error reporting and recovery
    expr.rs     – Pratt expression parser (atoms, unary, quantifiers, binary, calls, chains)
    stmt.rs    – statement and declaration parser (function, struct, enum, const)
    module.rs  – import and export parser
    pattern.rs – pattern parser (match arms, destructuring) [not yet implemented]
```

## Compliance with Fer draft v0.0.21

- Identifiers use kebab-case; struct/enum names must be Pascal-kebab-case
  (enforced by semantic analysis).
- Functions are defined without a `function` keyword and bind through `=`:
  `my-func = (x: i32, y: i32) -> i32 { x + y }`.
- Structs and enums are assigned with `=`. Struct fields support inferred
  defaults (``name = `guest``), explicit typed defaults
  (`age: i32 = 18`), and required fields (`id: i32`).
- Declarations and struct fields support `#[name]` annotations. Annotation
  arguments may be positional or named, for example
  ``#[derive = `Debug`, mode = stable]``.
- Imports use `{ names } = @scope/pkg`; exports use `exports { names }`.
- String literals use backticks, support interpolation such as `` `Hello {name}` ``, and support multi-line auto-dedent with physical-line continuation.
- No `==`, `!=`, `&&`, `||`, `!` – conditions use comparison keywords such as
  `equals`, `contains`, and `matches`, while logical combinations use quantifiers:
  `all (...)`, `any (...)`, `one (...)`, or `none (...)`.
- Quantifier conditions may be separated by commas or skipped trivia such as
  newlines; nested quantifiers are valid. The former `and`, `or`, and `xor`
  words are contextual identifiers rather than logical keywords.

## Current limitations

- Match expressions are parsed into `MatchExpr` and `MatchArm` nodes, but
  semantic validation is deferred to later compiler layers.
- The path comment (`/// @/...`) is not extracted and stored in CST.
- Existing CST nodes still omit some concrete delimiters; the first formatter
  therefore changes only line indentation and preserves all other source bytes.
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
- Quantifier tests cover all four kinds, nested quantifiers, mixed comma/newline
  separators, and contextual treatment of `and`/`or` as identifiers.

Run with `cargo test -p syntax`.

## Future work

- Integrate with the `query` incremental database: register `parse_file`
  as a cached query.
- Associate lossless tokens with CST nodes so future formatter passes can
  safely normalize separators, delimiters, and canonical spacing.
- Implement the `migrate` transforms on top of the source-mapped CST.
- Complete pattern parsing for match arms.