The **infra** crate provides the axiomatic data types and services that all other layers of the Fer compiler depend on.

## Modules

### `span`
- `Span` – a half‑open `[start, end)` byte‑offset range into a source file.
  It is the currency of diagnostics and syntax node locations.

### `symbol`
- `Symbol` – a `Copy` integer that uniquely identifies a string.
  Created exclusively by `Interner`.
- `Interner` – the single‑owner string pool.  It guarantees that two
  identical strings receive the same `Symbol`, and provides reverse
  lookup (`Symbol` → `&str`).

### `diag`
- `Severity` – enum (`Error`, `Warning`, `Note`).
- `Diagnostic` – a value object containing a severity, machine‑readable
  code (e.g. `"E0001"`), a human message, and a `Span`.
- `DiagnosticBag` – an ordered collector with severity filtering.
  Rendering is deliberately excluded from `infra` and belongs to
  a higher layer (e.g. the CLI driver).

### `profile`
- `TimingGuard` – a RAII guard that records wall‑clock duration.
- `NullProfileCollector` – a zero‑cost no‑op for release builds.

## Usage Example

```rust
use infra::{Interner, Span, Diagnostic, DiagnosticBag, Severity};

let mut interner = Interner::new();
let ident = interner.intern("main");
assert_eq!(interner.lookup(ident), Some("main"));

let mut bag = DiagnosticBag::new();
bag.add(Diagnostic::error("unexpected-token", "unexpected character".into(), span));
assert!(bag.has_errors());
```