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

## BAT/CMD launcher — manual Windows acceptance criteria

The pure logic (script-path validation, ComSpec basename validation, job-PID →
window filtering, console exclusion, one/many/none attribution, launch
precedence and outcome mapping) is covered by Rust and frontend unit tests.
**Real BAT execution cannot run headlessly and has NOT been executed by the
automated suite.** The following must be verified manually on Windows before
this feature is considered accepted:

1. **Script executes, no EXE substitution.** A `.bat` with spaces in its path
   writes a sentinel file (`%TEMP%\zf_bat_ran_<id>.txt`) and launches an app.
   After Launch & Arrange the sentinel exists and the app window is snapped.
   Configure both `launcher_path` and a bogus `path`/`exe`; confirm the bogus
   EXE is never launched (no process with that image appears).
2. **Environment / working directory preserved.** A script that sets an env
   var and `cd`s to a directory before launching an app produces observable
   effects in the app (e.g. it opens a file relative to that directory).
3. **Arguments preserved.** A script that passes arguments to an app results in
   those arguments being visible in the app.
4. **Immediate-exit script.** A script that launches an app and exits is still
   attributed correctly — the app window is found via the Job Object PID list
   even though the spawned `cmd.exe` PID is long gone.
5. **Multiple apps → ambiguity.** A script that opens two apps reports
   `ambiguous_window` with both windows' details; neither is guessed. Adding an
   EXE override then arranges exactly that app's window.
6. **Already-running app.** A script that focuses an already-running app still
   executes; with an EXE override that window is arranged; without one
   `no_eligible_window` is reported honestly.
7. **Missing / invalid script.** A missing script, a non-`.bat/.cmd` extension
   or a path containing `% ! & | ^ < >` reports `launcher_failed` with a clear
   message and never falls back to launching the EXE.
8. **Console visibility.** With `launcher_show_console = true` the console is
   visible while the script runs; with `false` it is hidden. Both still arrange
   the app window. EXE launches remain hidden in both cases.
9. **Console never arranged.** The visible `cmd.exe` console and `conhost.exe`
   window are excluded from attribution in every case above.
10. **Job lifetime.** Closing ZoneFlow while a script-launched app is running
    does NOT kill the app (no `KILL_ON_JOB_CLOSE`).
11. **Backward compatibility.** Existing EXE-based profiles behave exactly as
    before; old config files load with `launcher_path = ""` and
    `launcher_show_console = true` (covered by round-trip tests).

Attribution uses `CreateJobObjectW` + `CREATE_SUSPENDED` +
`AssignProcessToJobObject` before `ResumeThread`, then
`QueryInformationJobObject(JobObjectBasicProcessIdList)`. There is deliberately
no before/after HWND snapshot: the console is itself a new eligible window and
unrelated windows can appear during the poll, so a snapshot cannot guarantee
attribution. If Job Object assignment fails on a given machine (e.g. an
incompatible parent job), the outcome is `launcher_failed` — never a silent
downgrade to a snapshot heuristic.
