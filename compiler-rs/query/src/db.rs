// query/src/db.rs
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use infra::symbol::Interner;
use vfs::source::SourceMap;

use crate::query::QueryId;

/// The core of the Fer incremental compiler.
///
/// `Database` holds all program state (source files, interner, cached query
/// results) and enforces the "single source of truth" rule.
///
/// Queries are registered ahead of time and identified by `QueryId`.
/// Input queries are set explicitly; derived queries are computed on demand
/// and automatically re-computed when their dependencies change.
pub struct Database {
    /// Global revision counter. Incremented every time an input changes.
    generation: Cell<u64>,

    /// Registered queries. `None` for input queries, `Some(fn)` for derived.
    query_fns: Vec<Option<QueryFn>>,

    /// Cached results: `(value, generation_when_cached)`.
    cache: RefCell<Vec<Option<CachedValue>>>,

    /// Dependency graph: for each query, a list of queries that depend on it.
    dependents: RefCell<Vec<Vec<QueryId>>>,

    /// The source of all source texts.
    pub source_map: SourceMap,

    /// The global string interner.
    pub interner: Interner,
}

/// A boxed function that computes a derived query.
type QueryFn = Rc<dyn Fn(&Database, QueryId) -> Box<dyn Any>>;

/// A cached value, tagged with the generation it was computed in.
struct CachedValue {
    value: Box<dyn Any>,
    generation: u64,
}

impl Database {
    pub fn new(source_map: SourceMap, interner: Interner, query_count: usize) -> Self {
        Self {
            generation: Cell::new(1),
            query_fns: vec![None; query_count],
            cache: RefCell::new((0..query_count).map(|_| None).collect()),
            dependents: RefCell::new(vec![Vec::new(); query_count]),
            source_map,
            interner,
        }
    }

    /// Register an input query (no function, set externally).
    pub fn register_input(&mut self, id: QueryId) {
        self.query_fns[id.0 as usize] = None;
    }

    /// Register a derived query with a function.
    pub fn register_query(&mut self, id: QueryId, f: QueryFn) {
        self.query_fns[id.0 as usize] = Some(f);
    }

    /// Set an input value. Invalidates all transitive dependents.
    pub fn set_input<T: 'static>(&self, id: QueryId, value: T) {
        assert!(
            self.query_fns[id.0 as usize].is_none(),
            "Not an input query"
        );

        let new_gen = self.generation.get() + 1;
        self.generation.set(new_gen);
        self.cache.borrow_mut()[id.0 as usize] = Some(CachedValue {
            value: Box::new(value),
            generation: new_gen,
        });
        self.invalidate_downstream(id);
    }

    /// Get the value of a query, computing it if necessary.
    pub fn query<T: 'static + Clone>(&self, id: QueryId) -> T {
        // Check cache
        {
            let cache = self.cache.borrow();
            if let Some(cached) = &cache[id.0 as usize] {
                if cached.generation == self.generation.get() {
                    return cached.value.downcast_ref::<T>().unwrap().clone();
                }
            }
        }

        // Must be a derived query, compute it
        let f = self.query_fns[id.0 as usize]
            .as_ref()
            .expect("Derived query function not registered");
        let result: Box<dyn Any> = f(self, id);
        let value = result.downcast_ref::<T>().unwrap().clone();

        // Store in cache
        self.cache.borrow_mut()[id.0 as usize] = Some(CachedValue {
            value: Box::new(value.clone()),
            generation: self.generation.get(),
        });
        value
    }

    fn invalidate_downstream(&self, from: QueryId) {
        let deps = self.dependents.borrow()[from.0 as usize].clone();
        for dep in deps {
            self.cache.borrow_mut()[dep.0 as usize] = None;
            self.invalidate_downstream(dep);
        }
    }
}
