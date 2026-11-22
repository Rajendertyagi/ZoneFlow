# ZoneFlow — Path-based exe resolution & matching (no title matching)

Status: **plan only — not implemented.**

## Context / why

Launch & Arrange works mechanically (verified: `explorer.exe` snapped to the
bottom-right zone at 2220,1162 1627x957), but silently fails for most configured
apps and then reports success, so it looks like nothing happens.

Live API evidence (app running on `http://127.0.0.1:4000/`):

- Active profile `sdfsdfsds` (one zone -> `Antigravity.exe`):
  `HTTP 200 {"launched":true,"message":"Antigravity.exe: Antigravity.exe window not found"}`
- Profile `Research & Browser`:
  `HTTP 200 {"launched":true,"message":"Helium.exe: failed to launch Helium.exe; Obsidian.exe: failed to launch Obsidian.exe; snapped explorer.exe to 2227,1162 1613x950"}`

### Root causes

1. **False success.** `src/api/commands.rs:207` sets `launched: launch` (hardcoded
   `true` for any Launch & Arrange call). Per-zone errors are swallowed into the
   `message` string (`commands.rs:189-192`). Frontend `actions.ts:168` never reads
   `launched` and shows `toast('success', ...)` for every HTTP 200. Total failure
   is indistinguishable from success.

2. **Empty paths in config.** `src/apps.rs:291-298` — `enumerate_running_apps()`
   stores `path: ""` for every running process. Picker/drag-drop do pass `path`
   (`PickerDialog.svelte:71`, `AppLibrary.svelte:71`), but the running-app source
   supplies none, so `zone.path` ends up empty and matching falls back to fragile
   exe-name comparison. Installed apps (from `.lnk`/registry) do get real paths.

3. **Strict window matching.** `engine.rs:224-242` requires a visible, non-tool
   window >150px; `engine.rs:273-301` requires the owning process exe name to
   match `zone.exe` exactly. Enumeration of all 133 top-level windows (17 visible)
   showed:
   - `Antigravity.exe`: 11 processes, **zero top-level windows**; manual launch
     produced only a 0x0 window even after 16s -> filtered out. No window to snap.
   - `Helium.exe`: wrong exe name — real process is `chrome.exe` (window titled
     "MarketHubTS - Helium"). Not on PATH -> "failed to launch".
   - `Obsidian.exe`: not installed/on PATH -> "failed to launch".

4. **8-second timeout** (`engine.rs:405-412`) misses late windows.
   `CREATE_NO_WINDOW` (`engine.rs:401`) is worth questioning for GUI apps, though
   a manual launch without it also gave 0x0, so it is not the primary cause.

## User direction

- **No title matching.** Match by the actual exe only.
- **Resolve through PATH or use the exact path** — nothing fuzzy.

## Proposed plan

### 1. Make running-app enumeration carry the real path (`src/apps.rs`)
- `enumerate_running_apps()` currently emits `path: ""`. Resolve each process's
  full image path via `QueryFullProcessImageNameW` (logic already in
  `engine.rs:165`), falling back to `SearchPathW` if that fails.
- Result: picking a running app stores the actual exe path, so `zone.path` is
  populated going forward.

### 2. Resolve once, match by full path (`src/engine.rs`, `src/engine_core.rs`)
- Add `resolve_exe_path(exe, path) -> Option<String>`:
  - `path` non-empty -> use it as the exact path (normalize separators/case).
  - `path` empty -> `SearchPathW(exe)` (same search order as `CreateProcess`) to
    get the full path.
- `find_window_by_path_min` matches windows strictly by
  `process_path(pid) == resolved path` (normalized). Remove the exe-name-only
  fallback and all title logic.
- `execute_zone_inner` resolves once, launches via the resolved full path, and
  polls for a window on that exact path.

### 3. Honest reporting (`src/api/commands.rs`, `frontend/src/lib/actions.ts`)
- Count real per-zone successes; `launched` reflects actual outcome, and the UI
  shows a warning toast when zones fail instead of unconditional success.

### 4. Existing configs
- Resolve empty `path` at runtime via `SearchPathW` (no forced migration), so the
  current config works without manual edits.

## Open questions for the user

1. Strict path-only, or keep exe-name as a last-resort fallback when `SearchPathW`
   cannot resolve? (Strict is cleaner but reports failure for apps whose exe is
   not on PATH.)
2. Include #3 honest reporting in this change, or keep this pass scoped to path
   resolution only?
3. Backfill empty paths in existing configs on load, or resolve at runtime only?

## Honest caveat

This will not make `Antigravity.exe` arrangeable — it has zero top-level windows
(only a 0x0 one after launch). Correct path resolution will report that
accurately, but there is no window to snap until Antigravity's UI is actually open.

## Relevant files

- `src/apps.rs` — running/installed app enumeration, `path` population
- `src/engine.rs` — `process_path`, `find_window_by_path_min`, `execute_zone_inner`
- `src/engine_core.rs` — `launch_target`, pure helpers
- `src/api/commands.rs` — `apply_profile`, `launched` field, error aggregation
- `src/api/dto.rs` — `ApplyResultDto`
- `frontend/src/lib/actions.ts` — `doApply`, success toast
- `frontend/src/lib/components/dialogs/PickerDialog.svelte` — assignment path
- `frontend/src/lib/components/LeftRail/AppLibrary.svelte` — drag-drop path
