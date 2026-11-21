# Integration tests

## Why the main tests live inline in `src/`

`screenlayout` is a **binary crate**: it has a `src/main.rs` but **no
`src/lib.rs`**. Cargo integration tests are compiled as separate crates, so a
file in `tests/` cannot do:

```rust
use screenlayout::config::AppConfig; // ERROR: no library target to import from
```

For that reason the project's unit tests are written as `#[cfg(test)]` modules
inside the source modules themselves, where they can reach private items:

- `src/config.rs` — config defaults, v0 → v1 migration, serde round-trips,
  settings/zone defaults, invariant repair.
- `src/engine_core.rs` — pure layout/geometry core.
- `src/engine.rs` — engine helpers.
- `src/system.rs` — system-level helpers.

These are the authoritative tests for the crate's internals. Run them with
`cargo test`.

## What lives in this directory

`config_file_roundtrip.rs` tests the **on-disk JSON contract** independently of
the binary's internals. It does not import the crate; instead it declares a
small local mirror of the `AppConfig` JSON schema (using `serde` / `serde_json`,
which are regular dependencies and therefore available to integration tests)
and verifies two things:

1. A **v0-style config** (missing `schema_version`, `settings`, and newer
   fields) deserializes with the documented defaults — this is the contract
   that makes migration possible.
2. A fully populated **v1 config** survives a serialize → deserialize round
   trip, including the newer fields (`notes`, `auto_launch_on_startup`,
   resolution presets, …).

This catches accidental changes to field names or default values in the
serialized format even though the test crate cannot link against the binary.

## Deferred: expose a library target

A future improvement is to add a `src/lib.rs` that re-exports the modules
(`pub mod config; pub mod engine_core; …`) and have `main.rs` consume that
library. That would let `tests/` import the real types directly and would make
the local mirror in `config_file_roundtrip.rs` unnecessary. This is **deferred**
because it is a structural change outside the scope of the current step.
