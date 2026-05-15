The `query` crate provides a minimal, self-contained incremental computation database for the Fer compiler.  It replaces external frameworks like Salsa with a transparent, AI-friendly design.

## Structure

- `Database` – central state container with interner, source map, and query cache.
- `QueryId` – opaque `u32` identifier for a query.
- `QueryFn` – a type-erased function `(&Database, QueryId) -> Box<dyn Any>`.

## How to add a new query

1. Register an input or derived query with `Database::register_input` / `register_query`.
2. For derived queries, provide a closure that calls `db.query(...)` to read dependencies.
3. Call `db.query::<ReturnType>(query_id)` to get a cached or freshly computed value.

## Example

```rust
use query::{Database, QueryId};
use vfs::SourceMap;
use infra::Interner;

let source_map = SourceMap::new();
let interner = Interner::new();
let mut db = Database::new(source_map, interner, 2);

let source_id = QueryId(0);
let len_id = QueryId(1);

db.register_input(source_id);
db.register_query(len_id, std::rc::Rc::new(move |db, _| {
    let text: String = db.query(source_id);
    Box::new(text.len())
}));

db.set_input(source_id, "hello".to_string());
assert_eq!(db.query::<usize>(len_id), 5);
```

## Future enhancements

- Fine-grained dependency tracking (currently invalidation is coarse).
- Persistent caching for incremental recompilation across sessions.
- Query profiling and debugging hooks.