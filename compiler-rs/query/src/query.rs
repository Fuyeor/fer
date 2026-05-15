// query/src/query.rs

/// Opaque identifier for a query in the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueryId(pub u32);

/// Marker trait for input queries.
pub trait InputQuery: 'static + Clone {}
