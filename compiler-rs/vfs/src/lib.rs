// vfs/src/lib.rs
pub mod overlay;
pub mod source;

pub use overlay::Vfs;
pub use source::{FileId, SourceMap};
