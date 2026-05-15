The **vfs** crate provides the physical source of truth for all files processed by the Fer compiler.  It is the only layer that reads (and conceptually writes) source code, and it sits directly above `infra` in the dependency wall.

## Design Constraints

- **Zero external dependencies** beyond `std` and `infra`.
- Every source file is identified by a `Copy`‑able `FileId(u32)`.
- All paths are interned via `infra::Interner` – `VirtualPath` is a
  `Copy` integer and comparison is O(1).
- The overlay mechanism allows **time‑safe migration previews** and
  **formatting dry‑runs** without modifying physical files.  This is
  critical for Fer’s `migrate` feature.
- File and directory names are enforced to be **kebab‑case** (`[a-z0-9]+(-[a-z0-9]+)*`)
  with a mandatory `.fer` extension.  Uppercase, underscores, and
  parent‑traversal (`..`) are compile‑time errors.

## Modules

### `path`
- `VirtualPath` – a validated, interned path.  Created via
  `VirtualPath::new(raw, &mut Interner)` which returns `None` if the
  path violates Fer’s naming rules.

### `source`
- `FileId` – an opaque, zero‑cost handle to a source file.
- `SourceFile` – the content and path of a single file.
- `SourceMap` – the central registry that owns an `Interner` and maps
  `FileId` ↔ `VirtualPath` ↔ source text.  Also converts byte‑offset
  `Span`s into human‑readable `(line, column)` pairs.

### `overlay`
- `Vfs` – a thin wrapper around `SourceMap` that adds an in‑memory
  overlay layer.  Overlays take precedence over physical content and
  can be set/cleared per file.  This is the primary interface used by
  the rest of the compiler.

### `watcher`
- `FileWatcher` – a placeholder for future file‑system monitoring.
  Will eventually trigger incremental re‑compilation.

## Usage Example

```rust
use infra::Span;
use vfs::{SourceMap, Vfs};

// Create a VFS with a single physical file.
let mut vfs = Vfs::new(SourceMap::new());
let id = vfs.add_physical_file("src/main.fer", "constant = `hello`".into())
             .expect("valid path");

// Read physical content.
assert_eq!(vfs.read(id), "constant = `hello`");

// Span resolution: line 1, column 11.
let (line, col) = vfs.source_map().resolve(id, Span::new(11, 11)).unwrap();
assert_eq!((line, col), (1, 11));

// Preview a migration without touching the file on disk.
vfs.set_overlay(id, "constant = `migrated`".into());
assert_eq!(vfs.read(id), "constant = `migrated`");

// Revert.
vfs.clear_overlay(id);
assert_eq!(vfs.read(id), "constant = `hello`");
```