# ZoneFlow — Audit follow-up findings

Recorded 2026-11-21 after commit `3f5a578` (WS-A backend correctness). An
independent review of the commit raised four items. Each was verified against
the current source and the AHK reference before a decision was made. This file
keeps the decisions and the deliberately-deferred items visible.

## Resolved in follow-up commit

| Item | Finding | Resolution |
|---|---|---|
| B | Duplicate detection missed mixed assignments (one zone with a full path, another with only the exe name for the same app). | `duplicate_app_assignments` groups by exe name but excludes distinct non-empty full paths (different installations resolve to different windows). Regression tests added for the mixed case, distinct installations, and identical paths. |
| C | Generic `^X` parser accepted punctuation despite its doc saying letter/digit; `^!` mapped to VK 0x21 (VK_PRIOR). | Changed `is_ascii()` to `is_ascii_alphanumeric()`; regression test added. |
| D (clone) | `resolutions()` cloned the entire `Settings` each render frame. | Now clones only the `resolutions` vec. |
| D (precision) | Baseline reported startup to tenths of a millisecond despite 50 ms polling. | Per-run values rounded to ~10 ms; note added that detection granularity is ±50 ms. |

## Deliberately not changed

### A — Path matching falls back to exe name (not a defect)

`find_window_by_path_min` (`src/engine.rs`) returns `ctx.found.or(ctx.exe_match)`:
when no exact full-path match exists it falls back to the first window with the
same exe name. `execute_zone_inner` searches for an existing window before
launching, so a different installation of the same app can be selected and the
configured path never launched.

This is **intentional AHK parity**, not a regression. The reference script's
`FindWindowForApp` (`ScreenLayout.ahk:124–129`) does exactly this:

```
criteria := "ahk_exe " . (app.matchType = "path" ? app.path : app.exe)
hwnd := WinExist(criteria)
if (!hwnd && app.matchType = "path")
    hwnd := WinExist("ahk_exe " . app.exe)
```

The feature-preservation requirement is AHK parity (§2c / T3). Making matching
strict would change approved behavior. **If strict path-only matching is
wanted, that is a product decision and should be made explicitly** — it is not
a bug fix.

### D — Fixed config temp filename (low priority)

`write_config_to` (`src/config.rs`) uses a single fixed `ScreenLayout.json.tmp`.
Two concurrently running instances saving at the same time could race on that
file. This is a pre-existing pattern (not introduced by the WS-A work) and the
app is not designed for concurrent instances. A unique temp name (e.g. PID +
counter) would remove the race if multi-instance use ever becomes supported.

### D — Private bytes vs private working set (metric choice, not an error)

The baseline labels `PrivateMemorySize64` as "private bytes", which is correct.
It is committed private memory, not private working set. If a future baseline
wants resident private memory, use the corresponding working-set counter and
label it accordingly.

## Verification note

The 39 tests, Clippy run and Windows release build were executed locally and
are agent-reported. No CI status checks are attached to the commit, so GitHub
does not independently verify them. Adding CI is a separate workstream.
