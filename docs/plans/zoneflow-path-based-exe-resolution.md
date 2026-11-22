# ZoneFlow — Path-based exe resolution & matching (corrected plan)

Status: **approved for implementation.** Supersedes the earlier draft in this file.
No auth/session/security redesign. No change to fixed port 4000, open API, D1–D6,
current UI, or the on-disk config schema.

## Why this exists

Launch & Arrange works mechanically (verified: `explorer.exe` snapped to the
bottom-right zone) but silently fails for most configured apps and then reports
success, so it looks like nothing happens. Root causes, grounded in code:

1. `src/api/commands.rs:207` sets `launched: launch` — operation intent, not an
   outcome. Per-zone errors are swallowed into `message` (`commands.rs:189-192`).
   Frontend `actions.ts:168` shows a success toast for every HTTP 200.
2. `src/apps.rs:291-298` — `enumerate_running_apps()` keeps only the first CSV
   column (image name) from `tasklist`, discarding the PID and therefore the
   full path. `src/apps.rs:181-188` and `:214-221` — installed-app enumeration
   guesses `exe` from the shortcut stem and stores `path: ""`.
3. `engine.rs:273-301` matches windows by exe-name with an exe-name fallback;
   `engine.rs:405-412` waits 8 s. The earlier draft proposed `SearchPathW` for
   resolution — that is wrong (see Decision 3).

## Decisions

### Decision 1 — Running-process PID/path (fix the discarded PID)

`tasklist /FO CSV /NH` emits `"Image Name","PID","Session Name","Session#","Mem Usage"`.
The current code takes only field 0. Correct behavior:

- Parse field 1 as the PID (use a small quoted-CSV field parser, not a naive
  `split(',')`, so names containing commas are safe).
- For each PID, resolve the authoritative full image path with
  `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` +
  `QueryFullProcessImageNameW`. Expose the existing private `engine::process_path`
  as `pub(crate)` and reuse it — do not duplicate the Win32 code.
- Store that path in `AppEntry.path`; derive `AppEntry.exe` from the path's file
  name, never from the display label.
- If path lookup fails, keep the entry with `path: ""` and `exe` = process name
  so it remains selectable; runtime resolution (Decision 4) will retry.

### Decision 2 — Installed-app path assumption (stop guessing)

`scan_lnk_dir` currently does `format!("{clean}.exe")` from the shortcut stem;
`scan_uninstall_key` stores an empty exe/path. Both are guesses, not facts.

- Resolve `.lnk` targets properly with `IShellLinkW::GetPath` (COM via
  `CoInitializeEx` + `CoCreateInstance(CLSID_ShellLink)`). The `windows` crate
  features `Win32_UI_Shell` and `Win32_System_Ole` are already enabled in
  `Cargo.toml` — no new dependency.
- Derive `exe` from the resolved target's file name; store the full path.
- For registry Uninstall entries, use `DisplayIcon` (strip a trailing `,0`
  index) and/or `InstallLocation` only when the result is an existing `.exe`.
- **Never guess an exe from a label.** If no verifiable exe path is obtained,
  skip the installed entry entirely — a wrong path is worse than no entry.
  Running and Browse sources still cover those apps.
- Update `push_app` dedup: an entry with a non-empty path always beats one
  without, regardless of label length/priority. Entries with *different*
  verified full paths are kept as separate entries even when label and exe name
  are identical (Finding 2).
- **Known limitation:** only `IShellLinkW::GetPath` is read. `GetArguments` and
  `GetWorkingDirectory` are deliberately dropped: `AppEntry` stores one exe path
  and `config::Zone` has no argument/working-directory fields, so preserving
  them would require a config-schema and UI change (out of scope). The resolved
  exe is launched with no arguments and the default working directory. See the
  "Known limitations" section below.

### Decision 3 — SearchPathW search-order issue (do not use it)

`SearchPathW` with `lpPath = NULL` uses a **registry-dependent** search order
(`HKLM\...\Session Manager\SafeProcessSearchMode`): with the value 1 it searches
system folders before the current folder; with 0 the reverse. `CreateProcess`
uses a different order (application directory, current directory, system32,
Windows directory, PATH, App Paths). Resolving with `SearchPathW` can therefore
yield a path that disagrees with the image actually launched — then strict path
matching fails for a reason that looks like a bug.

Resolution: **eliminate `SearchPathW` from the design entirely.** Obtain full
paths from process identity instead:

- Running apps: PID → `QueryFullProcessImageNameW` (Decision 1).
- Launch: `Command::new(target).spawn()` → `Child::id()` →
  `QueryFullProcessImageNameW` on that PID. This is the exact image
  `CreateProcess` launched, by construction.
- Legacy configs with empty `path` resolve at runtime from running processes
  (Decision 4). This exe-name lookup is **resolution**, not window matching.

### Decision 4 — Strict full-path matching, no fallbacks

- Replace `find_window_by_path_min(exe, path, min_px)` with
  `find_window_by_paths_min(candidates: &[String], min_px) -> Option<HWND>`.
  It compares `normalize_path(process_path(pid))` against each normalized
  candidate. Remove `FindCtx.target_exe` and `FindCtx.exe_match` entirely.
- No executable-name comparison and no title comparison at match time, ever.
  `zone.title_match` stays only for the existing URL/tab/macro logic, never for
  window identity.
- `is_real_window_min` (visible, non-tool, > `min_window_px`) is unchanged.

### Decision 5 — Runtime resolution preserves config compatibility

No config schema change; `zone.path` stays optional/empty. Resolution for a zone:

1. `path` non-empty → use it as the exact path (source `ExactConfig`).
2. Else `exe` empty → unassigned (no resolution).
3. Else collect full paths of running processes whose exe basename equals
   `zone.exe` (case-insensitive) via Decision 1 (source `RunningProcess`).
4. Launch mode only: if no *running* candidate yet, spawn the target
   (CreateProcess resolves it), take `Child::id()` → full path (source
   `LaunchedProcess`). An exact configured `zone.path` is still a matching
   candidate even when that process is not currently running — in Launch &
   Arrange such a zone is spawned using that exact path. Here "no candidate"
   means no candidate whose process is currently running, not "no path value".
5. Attach mode with no candidate → unresolved / not running.

Existing configs keep working: `explorer.exe` resolves from its running
process and snaps; a wrong exe name (`Helium.exe` when the process is
`chrome.exe`) resolves to nothing and is reported accurately instead of being
silently faked. The user can re-assign via the picker, which now stores real
paths.

### Decision 6 — Intent separate from outcomes; accurate results for both verbs

Replace `ApplyResultDto { profile, message, launched }` with:

```rust
pub enum ApplyIntent { Apply, LaunchArrange }          // "apply" | "launch_arrange"
pub enum ZoneOutcomeStatus {
    Snapped, LaunchedAndSnapped, Unassigned, Unresolved,
    NotRunning, NoEligibleWindow, LaunchFailed, Error,
}
pub struct ZoneOutcomeDto { zone_id, label, exe, status, message }
pub struct ApplyResultDto {
    profile, intent, outcomes: Vec<ZoneOutcomeDto>, message,
    succeeded: usize, failed: usize, skipped: usize,
}
```

- `intent` records what was asked; `outcomes` record what actually happened.
  Remove `launched` (no longer a truthful signal). Update the frontend
  `ApplyResultDto` type and `doApply` accordingly.
- `message` is a summary derived from outcomes (e.g. "2 snapped, 1 no eligible
  window"), not a success claim.
- Apply (`launch=false`) never spawns; Launch & Arrange (`launch=true`) spawns
  only for zones with no running candidate. Both report the same outcome shape.
- Keep the D4 inactive result shape unchanged (`inactive: true, applied: false`).

### Decision 7 — Defined behavior for edge cases

| Case | Status | Counted as | Message |
|---|---|---|---|
| `exe` and `path` both empty | `unassigned` | skipped | "zone N has no application assigned" |
| exe non-empty, path empty, no running process, attach mode | `unresolved` | failed | "could not resolve <exe> to a full path" |
| exe non-empty, launch spawn fails | `launch_failed` | failed | "failed to launch <target>" |
| resolved path, process not running, attach mode | `not_running` | failed | "<exe> is not running (attach mode)" |
| resolved path, process running, no visible non-tool window > min_px | `no_eligible_window` | failed | "<exe> has no arrangeable window (minimized/hidden/0x0)" |
| window found and moved | `snapped` / `launched_and_snapped` | succeeded | existing "snapped ..." text |

No silent success: every zone yields exactly one outcome.

### Decision 8 — Tests

Rust:
- `apps.rs`: quoted-CSV PID parsing; PID→path resolution; `.lnk` target
  resolution; dedup prefers path-bearing entries; installed entries without a
  verified path are skipped.
- `engine.rs`: strict path matching (matches by full path; rejects exe-name-only
  and title-only); runtime resolution from running processes and from a
  launched PID; unassigned/unresolved/no-window outcomes.
- `api/commands.rs`: `apply_profile` intent vs outcomes; Apply vs LaunchArrange;
  unassigned/unresolved/no-window statuses; succeeded/failed/skipped counts;
  DTO serialization.

Frontend (bun test cannot import Svelte rune modules, so put the pure logic in a
new `src/lib/state/apply-result.ts` and test that):
- `summarizeApplyResult` maps outcomes to a toast tone + message: all snapped →
  success; partial → warning; all failed → error; unassigned-only → info;
  inactive → no success toast.
- Keep `doApply` using the helper; existing `workflow.test.ts` and
  `api-client.test.ts` stay green.

## Implementation order

1. `apps.rs` — PID/path + `.lnk` resolution + dedup rule (Decisions 1–2).
2. `engine.rs` / `engine_core.rs` — strict path matching + runtime resolution
   (Decisions 3–5).
3. `api/dto.rs` + `api/commands.rs` — intent/outcomes + edge cases (Decisions 6–7).
4. `frontend` — types, pure summary helper, `doApply`, tests (Decisions 6, 8).
5. Rust tests, full CI checks, real Windows testing, independent audit.

## Honest caveat

This will not make `Antigravity.exe` arrangeable — it has zero top-level windows
(only a 0×0 one after launch). Correct resolution reports `no_eligible_window`
accurately; there is no window to snap until Antigravity's UI is actually open.

## Known limitation — `.lnk` arguments and working directory

`resolve_lnk_target` uses `IShellLinkW::GetPath` only. Shortcut arguments
(`GetArguments`) and working directory (`GetWorkingDirectory`) are not read and
are not preserved. The resolved target exe path is stored in `AppEntry.path`
and launched via `launch_target` with **no arguments** and the **default working
directory**. Supporting them would require new `Zone` config fields plus UI and
a schema migration, which is explicitly out of scope; embedding them in the
stored path string would be a fragile hack. This is a launch-fidelity limitation,
not a matching change — window identity remains strict full-path comparison.

## Relevant files

- `src/apps.rs`, `src/engine.rs`, `src/engine_core.rs`
- `src/api/commands.rs`, `src/api/dto.rs`
- `frontend/src/lib/actions.ts`, `frontend/src/lib/api/client.ts`,
  `frontend/src/lib/state/apply-result.ts` (new), `frontend/tests/`
- `Cargo.toml` (features already sufficient — verify, do not add deps)
