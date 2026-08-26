# `ir`

The `ir` crate owns Fer's high-level intermediate representation and the lowering boundary from `syntax::CstNode`. It removes source-formatting details while retaining source spans for diagnostics and later IDE tooling.

## Architecture

HIR nodes use typed `u32` IDs and flat arenas. `HirArena` stores top-level nodes, bodies, expressions, match expressions, match arms, and conditions in separate `Vec` values. Nodes never own other HIR nodes through recursive pointers or smart pointers.

The lowering direction is one-way:

```text
syntax::CstFile -> ir::lowering::lower_file -> ir::hir::HirFile
```

`CstFile` is an owned syntax snapshot containing the `FileId`, source text, CST root, and CST node arena. Lowering does not re-lex or re-parse source text.

## Struct fields

The HIR makes the three valid field forms explicit with `FieldShape`:

```text
field = default-value       -> FieldShape::Inferred
field: Type = default-value -> FieldShape::Typed
field: Type                 -> FieldShape::Required
```

The former `field = type` spelling is lowered as an inferred field whose default expression is an identifier. A field with neither a type nor a default is represented by an error node and a lowering diagnostic; the invalid state is not representable by `FieldShape`.

## Match expressions

Match expressions have a dedicated `MatchId` arena. Each `MatchArm` stores an optional `ConditionId` and a `BodyId`. A missing condition is the default arm. Literal patterns lower to equality conditions, while parser-emitted `PatternCondition` nodes lower to normalized predicate operators such as `contains`, `matches`, `<`, and `>=`.

Logical condition groups lower to `ExprKind::Quantifier` with `QuantifierKind::{All, Any, One, None}` and an ordered `Vec<ExprId>`. The syntax accepts comma or skipped-trivia separators and nested quantifiers. The former `and`, `or`, and `xor` words are not lowered as logical binary operators.

## Interpolated strings

Interpolated strings lower to `ExprKind::InterpolatedString` with an ordered `Vec<InterpolatedPart>`. Text segments are owned strings, while embedded expressions reference the shared expression arena through `ExprId`; no recursive HIR pointers are introduced. The runtime evaluates all expression segments, formats their values, and applies Fer's multiline dedent and physical-line continuation rules to the combined template.

## Query integration

Call `register_queries` once on a `query::Database`. Set an owned `CstFile` with `set_cst_file`, then read `LOWER_HIR_QUERY` as a `HirFile`. The query declares its dependency on `CST_INPUT_QUERY`, so replacing the input invalidates the cached HIR result.

## Tests

`tests/lowering_snapshot_tests.rs` parses CST fixtures and verifies HIR snapshots for constants, all struct-field shapes, annotations, quantifiers, and match branches. `tests/query_tests.rs` verifies query registration, cache reuse, and input invalidation.

Run the crate checks with:

```sh
cargo fmt -p ir -- --check
cargo test -p ir --all-targets
cargo clippy -p ir --all-targets -- -D warnings
```
