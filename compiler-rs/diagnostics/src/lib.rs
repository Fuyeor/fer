// compiler-rs/diagnostics/src/lib.rs

//! Structured diagnostics and locale catalog support for Fer.

mod catalog;

pub use catalog::{Catalog, CatalogError, Locale, RenderError, RenderedDiagnostic, RenderedLabel};
