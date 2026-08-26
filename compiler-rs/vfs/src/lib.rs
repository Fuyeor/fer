// vfs/src/lib.rs

pub mod overlay;
pub mod path;
pub mod position;
pub mod source;
pub mod watcher;

pub use overlay::Vfs;
pub use path::VirtualPath;
pub use position::{LineIndex, SourcePosition, SourceRange};
pub use source::{FileId, SourceFile, SourceMap};
pub use watcher::FileWatcher;
