//! ZoneFlow library crate.
//!
//! The binary (`src/main.rs`) consumes this crate so that integration tests in
//! `tests/` can import the real types (notably `config::AppConfig`) instead of
//! maintaining a hand-written mirror of the on-disk schema. Re-exporting the
//! modules here does not change runtime behaviour: `main.rs` still drives the
//! same threads and the same code paths, it simply links them through the
//! library target rather than declaring them as binary-local modules.

// Making the modules public (so `main.rs` and integration tests can import
// them) exposes pre-existing binary-internal types to clippy's public-API
// lints. `NativeQueue::len` is such a case; adding `is_empty` would change the
// product API, so suppress the style lint at this wrapper-crate level instead.
#![allow(clippy::len_without_is_empty)]

pub mod api;
pub mod apps;
pub mod autostart;
pub mod browser;
pub mod clipboard;
pub mod config;
pub mod engine;
pub mod engine_core;
pub mod native_loop;
pub mod system;
pub mod web;
