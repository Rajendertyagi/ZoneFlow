# Integration tests

## Library target

`screenlayout` now exposes a **library target** (`src/lib.rs`) that re-exports
the implementation modules (`config`, `engine_core`, `engine`, `api`, `apps`,
`autostart`, `browser`, `clipboard`, `native_loop`, `system`, `web`). The binary
(`src/main.rs`) consumes that library, so integration tests in `tests/` can do:

```rust
use screenlayout::config::AppConfig; // real type, not a mirror
```

This replaced the old arrangement where the crate was binary-only and
`tests/*.rs` could not import its modules. The refactor is behaviour-preserving:
`main.rs` still drives the same threads and code paths, just linked through the
library target.

## What lives in this directory

`config_file_roundtrip.rs` tests the **on-disk JSON contract** (§10.1) against
the real `screenlayout::config` types. It verifies three things:

1. A **v0-style config** (missing `schema_version`, `settings`, and newer
   fields) deserializes with the documented defaults — the contract that makes
   migration possible.
2. An old **zone** without `notes` deserializes with `notes = ""` and the
   documented geometry defaults.
3. A fully populated **v1 config** survives a serialize → deserialize round
   trip. Every persisted field is asserted explicitly (field by field, since
   `AppConfig` has no `PartialEq` derive), including the newer fields (`path`,
   `launch_mode`, `title_match`, `url`, `tab_behavior`, `macro_enabled`,
   `trigger_title`, `notes`, all `Settings` tunables, resolution presets, …).

Because the test uses the real types and names every field, dropping a field
from the schema or changing its serde name/default fails this test (compile
error or assertion failure) instead of being silently ignored by serde's
"unknown fields are skipped" behaviour.

## Inline unit tests

The authoritative tests for crate internals remain `#[cfg(test)]` modules inside
the source files (e.g. `src/config.rs`, `src/engine_core.rs`, `src/api/dto.rs`).
Run everything with `cargo test`.
