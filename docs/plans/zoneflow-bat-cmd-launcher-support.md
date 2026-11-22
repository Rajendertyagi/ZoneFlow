# ZoneFlow — BAT/CMD launcher support (implementation plan)

Status: **plan only — not implemented.** Based on `main` at `1456d63`.
No code changes, no commits. Preserve D1–D6, the CPU/SSE fix in `1456d63`,
fixed port 4000, open API, and existing config compatibility.

## Objective

ZoneFlow must **execute the selected `.bat`/`.cmd` script** (preserving its
environment variables, arguments, working directory and multiple commands)
and use a separately-selected application `.exe` **only to identify the
resulting window**. It must never bypass the script and launch the EXE
directly.

## 0. Research — how others do it (evidence)

Authoritative sources consulted (2026-11-22):

1. **Microsoft, CreateProcessW docs** — *"To run a batch file, you must start
   the command interpreter; set `lpApplicationName` to `cmd.exe` and set
   `lpCommandLine` to `/c` plus the name of the batch file."* This is the
   **documented** way. The docs also warn that passing `NULL` for
   `lpApplicationName` with a path containing spaces can execute the wrong
   executable (`Program.exe` problem) — so pass `cmd.exe` explicitly as the
   application name.
   https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw

2. **Rust issue #94743 "Windows: Support running script files using
   `process::Command`"** (closed by the Rust libs team) — *"Running `.bat` or
   `.cmd` files may work due to an **undocumented feature** of `CreateProcessW`
   ... but it would be good to have proper, documented support."* The team
   explicitly declined to add script support to `std::process::Command` and
   left it to third-party code. They note finding `cmd.exe` via the `ComSpec`
   environment variable (with a fallback) and that `ShellExecuteExW` "can do a
   lot of the work ... but comes with major limitations (e.g. you can't
   redirect stdio)".
   https://github.com/rust-lang/rust/issues/94743

   **Implication for ZoneFlow:** our current `Command::new("script.bat")`
   relies on that undocumented behavior. We must implement the documented
   `cmd.exe /c` path ourselves — exactly what this plan proposes.

3. **Microsoft, Job Objects** — a job object captures a process **and all
   descendants created with `CreateProcess`**; `QueryInformationJobObject`
   with `JobObjectBasicProcessIdList` returns every PID in the job. Caveats
   from the docs: `JOB_OBJECT_LIMIT_BREAKAWAY_OK` /
   `SILENT_BREAKAWAY_OK` let children escape; `KILL_ON_JOB_CLOSE` would kill
   the app when ZoneFlow exits (must NOT be set); nested jobs complicate
   ownership. Many modern apps (browsers, Electron) deliberately break away
   or create their own jobs.
   https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects

4. **Microsoft, CreateProcessW remarks** — recommends `WaitForInputIdle` to
   synchronize before looking for a window belonging to a newly created
   process.

5. **Common launcher practice (AutoHotkey `Run`, Task Scheduler, Wox /
   PowerToys-style launchers)** — scripts are run through `cmd.exe /c` (or
   ShellExecute), and the resulting window is found by an **explicitly
   configured** target (window title/class in AHK, configured executable in
   task schedulers), not by guessing the child process. Process-tree
   discovery via WMI `ParentProcessId` is known to be unreliable (PID reuse,
   detached processes, broken chains).

**Conclusion:** the simplest *reliable* solution is the one in this plan —
run the script via the documented `cmd.exe /c` mechanism and identify the
resulting window by a **before/after HWND snapshot** (with an optional explicit
EXE override). Job Objects and process-tree/PID snapshots are strictly more
complex and less reliable (breakaway, PID reuse, detached children, multiple
apps). We deliberately do **not** use process names, window titles, basename
matching, or parent-PID heuristics.

## 1. Current architecture and limitations (with references)

### Data model
- `src/config.rs:84-109` — `Zone { id, label, exe, path, launch_mode, ... }`.
  `path` is documented as "full path to the executable (matchType: path parity)".
  There is **no field for a launcher script** and no field for a separate
  window-identification target.
- `src/api/dto.rs:140-144` — `AppEntryDto { label, exe, path }` (runtime picker
  entry, never persisted).
- `frontend/src/lib/api/client.ts:27-45` — `ZoneDto` mirrors `Zone`;
  `:170-175` `AssignDto { label, exe, path }`.

### Picker / Browse
- `src/apps.rs:602-620` — `browse()` uses `rfd::FileDialog::pick_file()` with
  **no file-type filter**; it stores the picked file's full path in `path` and
  its file name in `exe`. A `.bat` can be picked, but nothing distinguishes it
  from an EXE.
- `src/apps.rs:201-214` — `verified_exe()` requires a `.exe` suffix, so
  Installed/Running enumeration never yields `.bat`/`.cmd`.
- `frontend/src/lib/components/dialogs/PickerDialog.svelte:65-75` — `assign()`
  sends `{ label, exe, path }` verbatim; no extension validation or labels
  explaining what each field does.

### Launch engine
- `src/engine_core.rs:377-383` — `launch_target(zone)` returns `zone.path` when
  non-empty, else `zone.exe`. This is the **single** launch source.
- `src/engine.rs:614-651` — `execute_zone_inner` spawns via
  `Command::new(target)` with `CREATE_NO_WINDOW`, then calls
  `resolution_from_pid(child.id())` and uses that PID's image path as the
  window-matching candidate.
- `src/engine.rs:408-436` — `resolve_zone_paths` resolves a zone to candidate
  full paths from config/running processes; window identity is strict full-path
  via `find_window_by_paths_min` (`:255-320`). No exe-name/title fallback.

### Why BAT fails today (live-verified)
- `CreateProcessW` (used by Rust `Command`) **does** accept a `.bat` path, but
  Windows spawns a transient `cmd.exe` that runs the script and **exits
  immediately** (verified: spawned PID had `HasExited=true` within 800 ms).
- `resolution_from_pid` therefore captures `cmd.exe`'s path, not the app the
  script launched. Strict full-path matching targets the wrong process → the
  real app window is never found. Result: `no_eligible_window` / `not_running`.
- There is **no silent EXE substitution** — the failure is reported honestly.
  That part is correct and must be preserved.

## 2. Recommended design (revised — user feedback 2026-11-22)

The user's BAT files already contain the exe/commands they need. Forcing a
second, duplicate EXE selection is unnecessary friction. Revised design:

### Two concepts, but the EXE is optional
- **Launch file** — the `.bat`/`.cmd` ZoneFlow executes. Stored in a new
  optional field `Zone.launcher_path`. This is the primary and often **only**
  selection the user makes.
- **Window identification** — by default **auto-detected** from the window
  that appears after the script runs. An explicit EXE (`Zone.path`) is an
  **optional override** for the case where the script focuses an already-running
  app (no new window appears). The EXE is never launched when a launcher is set.

### Window auto-detection (default, no EXE needed)
1. Before launching, snapshot all eligible top-level windows as a set of
   `(HWND, process_path)` — reuse `EnumWindows` + `is_real_window_min` +
   `process_path` (already in `engine.rs`).
2. Run the script via `cmd.exe /c`.
3. Poll (existing 8 s window, 100 ms interval) for **new** eligible windows
   whose HWND was not in the before-snapshot.
4. Decide:
   - exactly one new window → arrange it automatically;
   - more than one → report ambiguity with the new windows' process paths and
     titles, do **not** guess;
   - none → if an explicit `Zone.path` override is set, fall back to strict
     full-path matching on it; otherwise report `no_eligible_window` with a
     message naming the launcher.

This uses actual window handles — no process names, titles, basename matching,
Job Objects, or parent-PID heuristics. It handles scripts that exit immediately
and detached processes because the window persists after the script is gone.

### Launch precedence (the core guarantee)
In `launch_target` / `execute_zone_inner`:
1. If `launcher_path` is non-empty → execute **that script** via the documented
   `cmd.exe /c` mechanism. Never launch `Zone.exe`/`Zone.path` in this case.
2. Else → current behavior unchanged (launch `path` or `exe`, identify by the
   spawned PID's image path).

This guarantees the script is executed. Selecting an EXE override can never
cause it to be launched instead of the script.

### Concrete script-launch mechanics (from the research)
- Resolve the command interpreter from `ComSpec` (per Rust issue #94743 and
  common practice), with a validated fallback to
  `GetSystemDirectoryW() + "\cmd.exe"`. Validate the value is an existing
  `.exe` before use; do not blindly trust an arbitrary env var.
- Use `Command::new(cmd_exe).arg("/c").arg(&quoted_script)` so `cmd.exe` is the
  explicit application name (avoids the `Program.exe` space-in-path trap) and
  the script path is a single argv element. Wrap the script path in quotes so
  `cmd.exe` handles spaces; escape embedded quotes per `cmd` rules.
- Set `.current_dir(script.parent())` so relative paths and `cd` inside the
  script behave as the user expects (Microsoft: `lpCurrentDirectory` controls
  the child's working directory; Rust `Command::current_dir` maps to it).
- **Terminal mode (user requirement):** for script launches, do **not** set
  `CREATE_NO_WINDOW` — the console stays visible so the user can see the BAT
  run. Add a per-zone toggle `launcher_show_console` (default `true` for
  scripts) so a script can be hidden if desired. EXE launches keep the current
  `CREATE_NO_WINDOW` behavior unchanged.
- After spawn, call `WaitForInputIdle` on the child where applicable, then run
  the window auto-detection described above (not the `cmd.exe` PID). The script
  may exit immediately — that is fine because identification never depends on
  the spawned PID.

### WebUI workflow (simplest understandable)
In the picker, make the **launcher script the primary selection** for BAT users:
- A **Browse** button labeled "Select .bat/.cmd launcher…" → fills
  `launcher_path`. This is the only required selection for the common case.
- An **optional** "App to arrange (EXE override)" field → fills `exe` + `path`.
  Leave empty to auto-detect the new window. Show a clear hint: "Leave empty to
  arrange the window the script opens; set only if the script focuses an app
  that is already running."
- A **"Show console when running script"** checkbox → `launcher_show_console`
  (default on).
- Clear labels: "Launcher script (runs instead of the app)" and "App to arrange
  (optional override)". Show the resolved values before saving.
- Validation: reject a `launcher_path` with a non-script extension; if an EXE
  override is set, verify it is an existing `.exe`.

### Why this is the simplest reliable solution
- Two new optional fields (`launcher_path`, `launcher_show_console`); no new
  profile/zone types; no change to D1–D6.
- No duplicate EXE selection in the common case — the BAT is self-contained.
- Reuses the existing `EnumWindows`/`is_real_window_min`/`process_path` and
  strict full-path matcher unchanged.
- Backward compatible: existing profiles have empty launcher fields (serde
  default) and behave exactly as today.
- Avoids fragile heuristics (process-name/title matching, "guess the child",
  Job Objects, parent-PID chains).

## 3. Files and components expected to change

- `src/config.rs` — add `#[serde(default)] pub launcher_path: String` and
  `#[serde(default = "default_launcher_show_console")] pub launcher_show_console: bool`
  to `Zone` + `Default`. No schema version bump (additive, defaulted fields).
- `src/api/dto.rs` — add `launcher_path` and `launcher_show_console` to `ZoneDto`
  and `AssignDto`.
- `src/api/commands.rs` — `ZoneAssign` stores both launcher fields; zone DTO
  conversions carry them.
- `src/engine_core.rs` — `launch_target` returns a `LaunchSpec` (script vs exe)
  instead of a bare `&str`; keep a compatibility wrapper for existing callers.
- `src/engine.rs` — `execute_zone_inner` branches on `LaunchSpec`: script path
  spawns via `cmd.exe /c` (console visible unless `launcher_show_console` is
  false), then auto-detects the new window via before/after HWND snapshot;
  optional `Zone.path` override uses strict full-path matching. EXE path
  unchanged. Add script-specific failure statuses/messages.
- `src/apps.rs` — `browse()` gains a script-mode filter (`*.bat;*.cmd`) for the
  launcher picker; keep the EXE browse unchanged.
- `frontend/src/lib/api/client.ts` — add both launcher fields to
  `ZoneDto`/`AssignDto`.
- `frontend/src/lib/components/dialogs/PickerDialog.svelte` — launcher Browse,
  optional EXE override, console checkbox, labels, validation.
- `frontend/src/lib/components/Inspector/ZoneInspector.svelte` — show/edit both
  launcher fields with clear labels.
- Tests: Rust (engine launch-spec, script quoting, before/after window
  detection, optional EXE override) + frontend (picker validation) + real
  Windows BAT scenarios (§7).

## 4. Configuration compatibility and migration

- `launcher_path` is `#[serde(default)]` → old JSON without it deserializes to
  `""`; `launcher_show_console` defaults to `true`. No migration code, no schema
  bump, no data loss.
- Existing EXE-based profiles: `launcher_path == ""` → identical behavior to
  today. Round-trip tests must confirm byte-compatible semantics for old configs.
- New profiles/zones default `launcher_path = ""`, `launcher_show_console = true`.

## 5. Process discovery and window-matching strategy

- **Script launch**: `cmd.exe /c "<script>"` with the script's directory as
  working directory. Quote the full script path; reject/escape `&`, `|`, `^`,
  `%`, `!` per Windows `cmd` rules. Preserve script arguments by storing them
  in the script itself (recommended) or in a dedicated `launcher_args` field
  only if truly needed — avoid a second new field unless required.
- **Window identification (default)**: before/after HWND snapshot — collect
  eligible windows before launch, then find new eligible windows after. Use the
  actual HWND; no process-name/title/basename matching. Optional `Zone.path`
  EXE override falls back to strict full-path matching only when no new window
  appears (e.g. script focuses an already-running app).
- **Why not Job Objects / process-tree discovery**: researched and rejected.
  Job Objects capture descendants but allow breakaway (browsers/Electron
  commonly break away), require careful limit flags, and add Win32 complexity;
  WMI parent-PID chains break on PID reuse and detached processes. The
  before/after HWND snapshot is simpler and uses the exact object we need to
  move. Document this decision in code so it is not "rediscovered" later.
- **Multiple apps / child processes**: if the script opens several new windows,
  report the ambiguity with their process paths/titles and do not guess. The
  user may set an EXE override to disambiguate. Other apps are out of scope
  (documented).
- **Already-running app**: the script always runs when a launcher is configured
  (the user asked for the script to execute). If no new window appears, use the
  optional EXE override to arrange the existing window; otherwise report
  `no_eligible_window` with a message naming the launcher.
- **Detached / immediate-exit scripts**: identification uses the window, not
  the spawned PID, so a script that exits immediately is fine — the window
  persists after the script is gone.
- **Ambiguous / missing windows**: reuse `ambiguous_path` (multiple candidates)
  and `no_eligible_window` / `not_running` statuses with messages that name the
  launcher and any override. No silent success.

## 6. Security and Windows command-line quoting

- Never build a command line by concatenating untrusted text into `cmd.exe /c`
  without quoting. Use `Command::new("cmd.exe").arg("/c").arg(&quoted_script)`
  so arguments are passed as a single argv element; wrap the script path in
  quotes and escape embedded quotes.
- Resolve the script path to an absolute path before launching; set the child's
  working directory to the script's parent so relative paths inside the script
  behave as the user expects.
- Do not add shell metacharacter interpretation beyond what `cmd /c` requires.
- No new network surface; no auth/session changes; fixed port 4000 unchanged.

## 7. Test cases and acceptance criteria (real Windows)

1. BAT with spaces in path → script executes, new window auto-detected and
   arranged (no EXE selected).
2. BAT that sets env vars + changes working directory → effects observable in
   the launched app; window arranged.
3. BAT that passes arguments → arguments received by the app.
4. BAT that launches an app and exits → app window auto-detected and arranged
   (PID is `cmd.exe`; identification uses the window, not the PID).
5. BAT that launches multiple apps → ambiguity reported with the new windows;
   no guess. With an EXE override, that app's window is arranged.
6. BAT that launches an already-running app → script still executes; with an
   EXE override the existing window is arranged; without one,
   `no_eligible_window` is reported honestly.
7. Missing script → `launch_failed` with clear message; no EXE fallback.
8. Terminal mode → with `launcher_show_console = true` the console is visible
   during execution; with `false` it is hidden. Both still arrange the window.
9. Multiple matching windows → `ambiguous_path` with paths listed.
10. Existing EXE-based profiles → unchanged behavior; round-trip tests pass.
11. **Script execution proof** in every applicable test: the BAT writes a
    sentinel file (e.g. `%TEMP%\zf_bat_ran_<id>.txt`); the test asserts it
    exists. This proves the script ran rather than the EXE being launched
    directly.

Acceptance: all 11 pass; Rust + frontend suites green; CPU/SSE fix from
`1456d63` still verified (idle `/api/state` count stays 0).

## 8. Implementation sequence (small work packages)

1. **Model** — add `launcher_path` + `launcher_show_console` to `Zone`, DTOs,
   defaults; round-trip tests.
2. **Launch spec** — `LaunchSpec` in `engine_core`; `execute_zone_inner` branch;
   script quoting + working directory + console visibility; unit tests.
3. **Window identification** — before/after HWND snapshot auto-detection;
   optional `Zone.path` EXE override via strict matcher; statuses/messages;
   unit tests.
4. **Picker/UI** — Browse script filter, optional EXE override, console checkbox,
   labels, validation; frontend tests.
5. **Real Windows BAT tests** — all 11 scenarios with sentinel proof.
6. **Independent audit + full CI** — read-only audit subagent, fmt/clippy/test/
   build (Rust + frontend), release build, CPU/SSE regression check.
7. **Commit/push** only after all findings closed and CI green (explicit user
   authorization required).

## Honest limitations

- A script that starts several apps reports ambiguity; the user can set an EXE
  override to pick one. Other apps are out of scope.
- Scripts that never produce a window report `no_eligible_window` truthfully.
- `cmd.exe /c` quoting is inherently subtle; the sentinel tests are the proof
  that execution semantics are preserved.
- No config schema version bump (additive defaulted fields) — verify old configs
  still load in the round-trip tests.
