# Fer Analysis

`analysis` is Fer's read-only semantic observation layer. It consumes the source-bearing CST and the lowered flat HIR, then produces independent semantic tables for later compiler stages. It never rewrites, annotates, or otherwise mutates the HIR.

## Name resolution

The first analysis feature is lexical name resolution in `resolve`. A `ResolutionTable` stores the target of each resolved `ExprId`, the definitions that were observed, the lexical scope tree, and source-aware diagnostics.

| Definition kind | Resolution target | Ownership |
| --- | --- | --- |
| Module or nested item | `DefTarget::Item(HirId)` | HIR item arena |
| Function parameter | `DefTarget::Param(HirId)` | HIR node arena |
| Body assignment | `DefTarget::Local(LocalId)` | Analysis layer |

`LocalId` belongs exclusively to `analysis`; no local declaration node is added to HIR. Because HIR names retain source spans rather than interned text, the resolver obtains each key by slicing the source-bearing CST text. Invalid spans produce `invalid-resolution-reference` instead of panicking.

The resolver applies the following scope rules:

1. Module items are predeclared in a first pass, so module constants and functions support forward references.
2. A function creates a parameter scope and a body scope. Match arms create child scopes. The current HIR has no separate nested-block node beyond indexed bodies, so every indexed body receives the scope supplied by its owner.
3. Quantifiers do not create scopes. Every condition in `all`, `any`, `one`, or `none` resolves in the current scope.
4. Body assignments are immutable constant declarations. The right-hand side is resolved before the left-hand name is inserted.
5. A duplicate in the same scope emits `duplicate-definition` and retains the first definition. A child scope may shadow an outer definition.
6. Type references, field names, enum variants, and annotation names remain outside the ordinary lexical value namespace in this first phase. Annotation and default-value argument expressions are still traversed.

Undefined references emit `undefined-name`. Diagnostics use stable English kebab-case codes and carry the relevant HIR source span.

## Incremental query integration

The analysis query is `RESOLVE_NAMES_QUERY` (`QueryId(2)`). The IR query IDs remain authoritative:

| Query | ID | Role |
| --- | ---: | --- |
| `ir::CST_INPUT_QUERY` | `0` | Source-bearing CST input |
| `ir::LOWER_HIR_QUERY` | `1` | Flat HIR derived from CST |
| `analysis::RESOLVE_NAMES_QUERY` | `2` | Read-only `ResolutionTable` |

Register queries in dependency order:

```rust
let mut database = query::Database::new(
    vfs::SourceMap::new(),
    infra::Interner::new(),
    3,
);
ir::register_queries(&mut database);
analysis::register_queries(&mut database);
ir::set_cst_file(&database, cst_file);

let table: analysis::ResolveNames = database.query(analysis::RESOLVE_NAMES_QUERY);
```

The resolver query reads both the CST input and lowered HIR. Replacing the CST invalidates the HIR and resolution caches through the query dependency graph.

## Tests

Run the focused test suite with:

```text
cargo test -p analysis --all-targets
cargo clippy -p analysis --all-targets -- -D warnings
```

The integration tests lower real syntax fixtures rather than constructing artificial HIR. They cover module forward references, parameters, local declaration order, first-wins duplicate handling, Match Arm shadowing, quantifiers, undefined-name spans, query caching, and CST-driven invalidation.
