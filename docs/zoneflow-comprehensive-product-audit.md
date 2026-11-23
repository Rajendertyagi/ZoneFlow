# ZoneFlow — Comprehensive Product Audit & Improvement Roadmap

## FINAL STATUS (2026-11-23) — all work packages complete

Every applicable finding below has been resolved, independently reviewed, and pushed to `origin/main`. This follow-up started at `138f126`; final HEAD: `99d5673`. Final checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (**246 unit + 3 integration**), `cargo build --release`, `bun run check`, `bun test` (**162**), `bun run build` — all passing.

| Package | Commit(s) | Findings closed |
|---|---|---|
| WP1A | `f93e45e` | U-01 (installed-app duplicate-key crash) |
| WP1B | `87396bb`, `15ff4be` | F2, F1 + two interaction edge cases |
| WP1 remainder | `ee12d8b` | F1b (undo/redo keyboard shortcuts) |
| WP2 | `d6ec625`, `dbfda78` | B-1, U-03, R-01, E-03 + race/absent-value follow-up |
| WP3 | `c577e79` | F4, B-2, F6, D-3, F3, F5 + latent D3 fallback |
| WP4 | `28db66c` | E-01, E-02, E-04, E-05, E-06, E-07, E-08, B-4, D-1, D-2 + close-action defect |
| WP5 | `d8299aa` | U-02, U-04, U-05, U-06, U-07, U-08, U-09, B-3/B-6 + EnumWindows fail-open |
| WP6 | `1a2ea7b` | F8 (library target + full round-trip), R-05 (`.bad` recovery), SSE backoff tests, path-isolated config write |
| WP7 | `7861087` | BAT/CMD launcher support via Job Objects (reliable attribution, no HWND snapshot heuristic) |
| WP7 follow-up | `99d5673` | WP7 ambiguity quiet-period reset + Job PID query error handling |

**Intentionally deferred / documented, not defects:**
- **R-02 (security):** the unauthenticated loopback API CSRF/DNS-rebinding risk remains an accepted trade-off. Per explicit user decision, no Origin/Host/CSRF/CORS/auth mechanism was introduced. Documented as a known limitation.
- **D-1 display-change re-tiling:** the dead `display_debounce_ms` control was removed; full display-change detection was not implemented (native-loop change beyond settings scope). Documented as a limitation.
- **B-4 startup auto-launch:** the dead toggle was removed; feature not implemented. Documented as a limitation.
- **BAT/CMD live Windows verification:** 11 manual acceptance scenarios are documented in `tests/README.md`; no live Windows BAT execution was performed (headless environment). Job Object attribution is implemented and unit-tested at the pure-logic level, but not exercised against a real desktop.
- **BAT/CMD ambiguity quiet period (documented limitation, not a regression):** the ambiguity quiet period is 400 ms measured from the most recent new window. A second application window appearing AFTER that quiet period has already elapsed (e.g. >400 ms after the first with no new windows in between) can still be missed because the poll loop breaks on settle. An absolute guarantee would require polling the full 8 s deadline on every single-app launch (a UX regression). This is a documented limitation of the prescribed mechanism, not a regression.

**Original audit baseline below (historical record).**

---

**Baseline:** clean `main` @ `8fbdf5e`, synced with origin. **Method:** five sequential read-only subagents. No code, config, UI, tests or docs were modified during the audit; nothing committed or pushed until this report was saved. **Verification limits:** no tests or builds were run. Findings are static-analysis based except Part 3, which did live visual verification against an already-running backend (screenshots at 1440×900 and ~639px). Where a subagent said "test exists," that means the test was *read*, not executed.

**Baseline gaps:** No `AGENTS.md` and no "project management protocol" document exist in the repo — reported rather than invented. Several docs claim features that have no consumer (`auto_launch_on_startup`, display-change re-tiling), and test counts conflict across README/CHANGELOG/architecture (126+43) vs implementation plan (128+36).

---

## A. Executive assessment

ZoneFlow is **architecturally strong but product-incomplete**. The hard engineering is genuinely good: a clean three-thread design with one authoritative state owner, strict path-based window matching, crash-safe config I/O, an ordered shutdown sequence, and a well-engineered SSE/CPU fix. D1–D6 are encoded in one exhaustive command layer and broadly tested.

The weakness is at the **user-facing seam**. The audits found a hard blocker (installed-app lists crash on a typical Windows machine), several false-success paths, a data-integrity bug in undo of profile operations, and a cluster of Settings controls that do nothing. The recurring theme: **the backend often does the right thing, but a UI consumer either ignores the result, mislabels it, or offers a control with no implementation behind it.** That is exactly the class of problem that needed to be found systematically rather than by trial and error.

Honest readiness: **not yet functionally complete or polished enough for a proper release.** The good news is that most findings are small, localized, and independent — no architectural rewrite is warranted.

---

## B. Feature completeness matrix

| Feature / workflow | Status | Evidence | Missing work |
|---|---|---|---|
| Zone create/draw/move/resize/split/merge | Working | `ZoneCanvas.svelte:138-412`; `commands.rs:543-699`; `geometry.ts` | — |
| Delete last zone / empty profile | Partially Working | Backend fixed `8fbdf5e`; empty-state UI present | Context menu still blocks it (F3) |
| Undo/redo (data) | Working | `state.rs:109-162`; batch single-step `commands.rs:575-613` | — |
| Undo/redo (selection) | Broken | F1 | Wire selection into snapshots/UI |
| Undo/redo (keyboard) | Missing | F1b | Add global shortcuts |
| Undo/redo across profile ops | Broken | F2 (P1) | Capture all profile dimensions |
| Save / revert / dirty (D1/D2) | Working | `actions.ts:128-157`; `commands.rs:204-214` | — |
| Apply / Launch / Preview (D1/D5) | Working | `commands.rs:795-841` | — |
| Per-zone outcome reporting | Partially Working | `ApplyResultDto`, `ApplyResults.svelte` | Home per-profile Apply bypasses it (B-1) |
| D3 replacement confirmation | Partially Working | `assignAppGuarded` | Inspector bypass (F4); same-app wipe (B-2) |
| D4 disable/enable | Partially Working | `commands.rs:847-877,981-994` | Tray enable doesn't re-apply (F6); overlay shown when disabled (D-3) |
| D6 profile switching | Working | `commands.rs:482-530` | — |
| Profile CRUD | Working | `commands.rs:368-530` | Backend CRUD not dirty-guarded (DR-1) |
| Installed app discovery | Working (backend) / **Broken (UI)** | `apps.rs:317-508`; U-01 | Duplicate-key crash in both lists |
| Running app discovery | Working | `apps.rs:510-579` | Noisy with system processes (U-06) |
| Browse native picker | Partially Working | `apps.rs:602-621` | No file filter; tab bugs (B-3, B-6) |
| Path resolution / dedup / ambiguity | Working | `engine.rs:255-320,409-464`; `apps.rs:25-32` | — |
| Evolving snapshot / 8s poll / duplicate-launch fix | Working | `engine.rs:564-651,705-715` | — |
| BAT/CMD launcher support | **Planned** | Plan doc `37a1ced`, "not implemented" | Full implementation; plan has a fatal gap (R1) |
| Tray menu (7 entries, Tools submenu) | Working | `system.rs:63-153,229-257` | — |
| Global hotkeys | Working | `system.rs:155-227` | Wrong labels (U-04); false success (U-03) |
| Window detection / positioning | Working | `find_window_by_paths_min` | Multi-window limitation undocumented (DR) |
| Overlay | Partially Working | `native_loop.rs:626-660,742-755` | Stale on display change (D-1); shown when disabled (D-3) |
| Startup / shutdown / SSE close | Working | `main.rs:65-229`; `events.rs:17-44` | — |
| Port conflict dialog / clipboard / autostart | Working | `main.rs:139-178`; `clipboard.rs`; `autostart.rs` | Autostart errors swallowed (R-01) |
| Settings — working tunables | Working | snap/grid/min/tolerance/undo/alpha/retries/min-px/cap/TTL/gap traced | — |
| Settings — dead controls | **Broken** | E-01, E-02, B-4, D-1, D-2 | Remove or implement |
| Settings — persistence model | Confusing | E-03 | Save button doesn't persist |
| Settings — validation | Missing | E-04 | Backend accepts anything |
| CPU/SSE fix (`1456d63`) | **Working — confirmed intact** | All three legs verified by two independent audits | — |
| Config persistence / migration / recovery | Working | `config.rs:356-419` | `.bad` recovery untested (R-05) |
| Config round-trip test | Partially Working | `tests/config_file_roundtrip.rs` | Incomplete schema mirror (F8) |
| Frontend component/E2E tests | Missing | 6 pure-helper test files only | No DOM/E2E framework |

---

## C. Prioritized findings

Severity uses the final cross-check's calibration. Each finding: what / where / evidence / impact / why / solution / acceptance / risk.

### P0 — Critical
**None found.** No data-loss, security-breach, or totally-broken-core finding rose to P0. The cross-check explicitly found no missed P0/P1 issues.

### P1 — Critical to the product experience

**P1-1 — Installed-app lists crash with `each_key_duplicate` (U-01)**
- **What/where:** `AppLibrary.svelte:117` and `PickerDialog.svelte:148` key list rows on `app.exe + app.path`. Backend dedup is keyed on `(label, path)` (`apps.rs:31-32,248-300`), so two entries with the same exe+path but different labels (e.g. Start-Menu "chrome" vs registry "Google Chrome") both survive.
- **Evidence:** Live visual verification: console showed `Uncaught Error: each_key_duplicate` (3×); Installed tab stuck at "Loading…" indefinitely; Running tab rendered 69 rows correctly. `/api/apps?source=installed` returned duplicate exe+path pairs (AgnesCode, AutoHotkeyUX, chrome, cline-app, msedge).
- **Impact:** The primary way to find and assign an installed app is completely broken on a typical machine. Users can only fall back to Browse or the Running tab.
- **Why:** Core workflow hard blocker, not cosmetic.
- **Solution:** Deduplicate in backend `list_apps` by normalized path (preferred — fixes both consumers and `app_list_cap` consistently), or use a guaranteed-unique key including label/index.
- **Acceptance:** With the current machine's installed list, both panels render all entries; no `each_key_duplicate` in console; regression test with a duplicate exe+path fixture.
- **Risk:** Dedup changes list length and `app_list_cap` behaviour. Low.

**P1-2 — Unauthenticated loopback API: CSRF / DNS rebinding (R-02)**
- **What/where:** `routes.rs:428-430` explicitly "no auth guard, no bearer/cookie validation and no exact-Host check"; `main.rs:105` binds `127.0.0.1:4000`. No-body POSTs (`/api/shutdown`, `/api/undo`, `/api/redo`, `/api/save`, `/api/apply`, `/api/launch-arrange`, `/api/preview`, `/api/revert`, `/api/disable`, `/api/enable`, `/api/apps/browse`, `/api/profiles/{name}/load|select|apply`) take only `State` and execute cross-origin without preflight. GETs expose `/api/state`, `/api/logs`, `/api/settings`, `/api/hotkeys`.
- **Evidence:** A cross-origin `<form method=POST action="http://127.0.0.1:4000/api/shutdown">` sends a CORS-safelisted request with no preflight. DNS rebinding makes these same-origin (full read/write). JSON-body endpoints are incidentally shielded by axum's content-type requirement.
- **Impact:** A malicious page the user visits can shut down ZoneFlow, toggle automation, overwrite/revert config, or delete profiles.
- **Why:** Loopback-only is a meaningful boundary but not complete against a hostile web origin.
- **⚠️ Decision conflict:** You previously explicitly forbade reintroducing Host validation, Origin checks, CSRF, CORS allowlists, or any auth layer. I am **not** recommending implementation unilaterally — this needs your decision. Options: (a) accept and document the risk as an intentional trade-off; (b) allow a minimal same-origin check despite the earlier decision; (c) mitigate by binding/process-level means that don't touch HTTP headers.
- **Acceptance (if mitigated):** Cross-origin mutating requests rejected; same-origin SPA and tray flows still work with no credentials.
- **Risk:** Must not break the tray's open-editor flow or SPA fetches.

**P1-3 — Undo/redo desyncs profile dimensions (F2)**
- **What/where:** `Snapshot` (`state.rs:19-23`) captures only `cfg_json` + `selected_zone`; `apply_snapshot` (`:125-132`) restores only those. `ProfileRename`/`Delete` (`commands.rs:417-454`) mutate `selected_profile`/`applied_profile` then `push_history`.
- **Evidence:** After undo of a rename, `selected_profile` points to a name no longer in the draft → `drag_zones()` (`state.rs:221-226`) falls back to an empty profile and `Apply` (`commands.rs:795-805`) fails with `not_found`. No test covers undo of a profile op.
- **Impact:** Home/tray show a selected profile absent from the list; overlay shows no zones; Apply fails.
- **Why:** D6 requires Selected/Applied/Active to stay distinct but coherent; undo silently breaks that.
- **Solution:** Include `selected_profile` and `applied_profile` in `Snapshot` and restore them; add regression tests for undo after create/load/rename/delete.
- **Acceptance:** Undo/redo after any profile CRUD leaves all three dimensions pointing to existing profiles.
- **Risk:** History format changes, but in-memory histories are ephemeral — no migration needed. Medium.

### P2 — High

**P2-1 — Per-profile Apply shows unconditional success (B-1):** `HomeView.svelte:83-90` → `guardedProfileOp` (`actions.ts:41-52`) toasts success for any non-inactive result and never calls `summarizeApplyResult`/`showApplyResults`, unlike `doApply`. A profile whose zones all failed shows "Applied X." with no explanation. **Solution:** detect `ApplyResultDto` via `isApplyResult`, use the same summary/panel as `doApply`. **Acceptance:** failing zones produce warning/error tone and a populated panel. Low risk; preserve D2 guard.

**P2-2 — "Save settings" doesn't persist (E-03):** `SettingsSet`/`AppOptionsSet` (`commands.rs:884-907`) mutate the draft only; no `inner.save()`. The toast does disclose "Save the profile to persist draft changes," but the button label is misleading. **Solution:** either persist immediately in `SettingsSet`, or rename to "Apply settings (draft)" with an inline persistent warning. **Acceptance:** restart retains values, or UI unambiguously states the extra step. Option (b) lower risk.

**P2-3 — No display-change detection (D-1):** `native_loop.rs:626` computes `work_area()` once; no `WM_DISPLAYCHANGE` handling or polling; `display_debounce_ms` has no consumer. Docs (`CHANGELOG.md:43`, `zoneflow-master-plan.md:619`) falsely claim it works. **Impact:** after resolution/taskbar change, overlay is misaligned and drops snap to wrong rectangles. **Solution:** detect display changes (real message window or work-area polling at `display_debounce_ms`), recompute overlay work/zones, push `SetZones`, re-tile. **Acceptance:** overlay tracks work area after display change; setting honored or removed. Medium risk — preserve §3.6 shutdown ordering.

**P2-4 — `auto_launch_on_startup` toggle has no consumer (B-4):** only in config/DTO/UI/tests/docs; no startup code reads it. Docs falsely claim implemented. **Solution:** implement startup auto-launch or remove/disable the toggle. **Acceptance:** feature works end-to-end or UI doesn't present it as functional.

**P2-5 — Inspector direct exe/path edit bypasses D3 and retains stale rules (F4):** `ZoneInspector.svelte:28-40,107,117` calls `api.updateZone` → `ZoneUpdate` (`commands.rs:562-574`), no rule reset/confirmation. Worse, old app's URL/title/macro rules are **retained** for the new app. **Solution:** route identity changes through the D3 guard, or make exe/path read-only in inspector. **Acceptance:** any app-identity change from any surface triggers confirmation and resets rules. Low risk.

**P2-6 — Same-app reassignment wipes rules without confirmation (B-2):** `workflow.ts:71-74` requires `zone.exe !== next.exe` for confirmation, but `ZoneAssign` (`commands.rs:700-725`) resets all rules unconditionally. Re-picking the same app (e.g. to add a path) silently destroys launch mode/URL/tab/macro/notes. **Solution:** prompt whenever `hasNonDefaultRules(zone)` is true and rules will be reset. **Acceptance:** any rule-resetting assignment confirms first. Low risk.

**P2-7 — Hotkey save reports success even when registration fails (U-03):** `HotkeysView.svelte:44-52` unconditional success toast; `reregister_hotkeys` is fire-and-forget (`native_loop.rs:277-281`); `system.rs:48,203,221` records failures in a vector that is **never read**. **Impact:** typo/conflicting binding silently does nothing. **Solution:** return registration failures, surface in API/state, show warning toast naming failed bindings. **Acceptance:** invalid binding produces visible warning. Medium risk (signature change, multiple callers).

**P2-8 — Undo/redo doesn't restore selection (F1):** backend stores/restores `selected_zone` but frontend never reads it; no `SelectZone` command exists. After undoing a creation, `selectedId` points at a deleted zone. **Solution:** add `SelectZone` command and reconcile local selection from snapshot, or drop the backend field and document selection as UI-local. **Acceptance:** previously selected zone reselected after undo/redo. Low risk; SSE timing race to watch.

**P2-9 — Undo/redo keyboard shortcuts absent (F1b):** README documents `Ctrl+Z`/`Ctrl+Shift+Z`/`Ctrl+Y`; no handler exists. **Solution:** global keydown in `App.svelte`, guarded against inputs/dialogs. **Acceptance:** shortcuts work in editor, don't fire while typing. Coordinate with F1.

**P2-10 — Duplicate-assignment warning logic mismatches backend (F5):** frontend `workflow.ts:118-130` groups by exe only; backend `engine_core.rs:395-429` is path-aware. UI falsely warns for two distinct installations that strict path matching actually places separately. **Solution:** port path-aware logic to frontend or expose backend helper. **Acceptance:** frontend/backend agree; add frontend test. Low risk.

**P2-11 — Tray "Enable layout" doesn't re-apply (F6):** `Action::ToggleEnabled` (`commands.rs:981-994`) only flips the flag; `CommandPayload::Enable` (`:854-877`) re-applies when clean. Same verb, different behaviour. **Solution:** make `ToggleEnabled` delegate to the enable path. **Acceptance:** tray and UI enable produce identical applied state. Low risk.

**P2-12 — No backend validation of settings (E-04):** `SettingsSet` accepts any DTO; all min/max constraints are HTML-only. Loopback API is unauthenticated, so any local process can corrupt config (`grid_cols: 0`, negative `min_window_px`, etc.). **Solution:** add `Settings::validate()` clamping to UI-advertised ranges; call on set and load. **Acceptance:** out-of-range PUT yields clamped values. Medium risk — keep UI/backend ranges in sync.

### P3 — Medium / Low

| ID | Finding | Evidence | Fix |
|---|---|---|---|
| E-01 | `resize_step_pct`, `taskbar_h`, `stage_lower_h` exposed but unconsumed | `SettingsView.svelte:163-165,266-274`; no consumer | Remove controls; keep fields for schema compat |
| E-02 | `resolutions` dead + false code comment claiming a UI selector | `config.rs:199-201` | Remove field/comment or implement selector |
| E-05 | Autostart toggle seeded from config, not registry; `is_auto_start()` dead | `autostart.rs:64-92`; `SettingsView.svelte:54-58` | Expose real registry state in snapshot |
| E-06 | Settings exposes Win32/API jargon with no help text | `SettingsView.svelte:120-284` | Group into Editor/Appearance/Advanced with plain-language copy |
| E-07 | `zone_tints` functional but no UI editor | `config.rs:193`; `ZoneCanvas.svelte:664` | Add compact color-swatch editor |
| E-08 | One "Save settings" click creates two undo points | `commands.rs:887,904` | Batch into one command |
| F3 | Context menu still disables removing last zone | `ZoneCanvas.svelte:569` vs `commands.rs:614-627` | Remove `zones.length <= 1` guard |
| F7 | Wizard template apply creates many undo points | `WizardDialog.svelte:37-69` | Backend "replace zones with template" command, one history entry |
| F8 | Config round-trip test mirrors only a subset of schema | `tests/config_file_roundtrip.rs:87-104,131-150` | Mirror all fields or use real `AppConfig` via lib target |
| B-3/B-6 | Browse result overwritten / stale list on tab switch | `AppLibrary.svelte:47-68`; `PickerDialog.svelte:38-63` | Don't mutate `source`; clear `apps` on Browse entry |
| D-2 | "Remember this choice" in CloseDialog has no effect | `CloseDialog.svelte:28-41`; no reader of `close_action` | Honor it or remove checkbox |
| D-3 | Overlay appears on Ctrl+drag when layout disabled; drop ignored | `native_loop.rs:856-868`; `commands.rs:934-937` | Track `enabled` in `NativeLoop`; skip overlay |
| U-02 | `Input` ignores caller sizing classes (`cn()` just joins) | `input.svelte:35-40`; `utils.ts:2-3` | Use `tailwind-merge` or remove base sizing |
| U-04 | `profile_N` hotkey labelled "Apply profile N" but launches zone N | `HotkeysView.svelte:24-25` vs `system.rs:344-350` | Relabel "Launch/focus zone N (current profile)" — do **not** change behaviour |
| U-05 | Hotkey inputs have no accessible label | `HotkeysView.svelte:83-90` | Add `id`/`aria-label` |
| U-06 | Running list dominated by non-window system processes | `apps.rs` running enumeration | Filter to visible-window processes or group background |
| U-07 | Responsive breakpoints hide rail/inspector with no alternative | `EditorView.svelte:204,231` | Compact profile selector at narrow widths |
| U-08 | Home rows have redundant Select/Edit controls; tooltip leaks "D6" | `HomeView.svelte:170-194` | One affordance per action; remove internal IDs from copy |
| U-09 | Wizard "Replace" deletes assigned zones without warning | `WizardDialog.svelte:30,42-54` | Confirm listing zones/assignments to be removed |
| R-01 | Autostart registry errors swallowed → false "Settings saved" | `autostart.rs:16-62`; `commands.rs:884-907` | Return `Result`, propagate as `ApiError` |
| R-04 | Every mutation does two full snapshot fetches; `set_zones` on read-only commands | `actions.ts` explicit refreshes + SSE; `worker.rs:63` | Pick one refresh source; gate `set_zones` on `emits_state_change` |

**Design recommendations (not defects):** DR-1 backend CRUD dirty guard is frontend-only; DR-2 `selected_zone` vestigial; DR-3 wizard keeps first zone's assignment; multi-window-per-app limitation undocumented; aggregate tone for unassigned zones inconsistent with per-zone badge; Browse accepts non-EXE files.

---

## D. Recommended implementation roadmap

Ordered by user impact and dependency, not ease.

**WP0 — Decision gate (before any code): R-02.** You must decide whether to accept the CSRF/rebinding risk as an intentional trade-off or permit a minimal same-origin check. This blocks the security work package only; it does not block other packages.

**WP1 — Core stability & data integrity (P1).** Fix U-01 (duplicate-key crash), F2 (undo profile dimensions), F1/F1b (selection + keyboard undo). *Why together:* all are core-workflow breakages in the editor/profile layer and share the state/snapshot code. *Files:* `apps.rs`, `AppLibrary.svelte`, `PickerDialog.svelte`, `state.rs`, `commands.rs`, `ZoneCanvas.svelte`, `App.svelte`, tests. *Testing:* duplicate-fixture regression, undo-after-profile-CRUD tests, selection restoration test. *Done when:* installed lists render on the audited machine; undo after rename/delete leaves coherent dimensions; Ctrl+Z works and restores selection.

**WP2 — Trustworthy feedback (P2).** Fix B-1, U-03, R-01, E-03. *Why together:* all are "reports success when it didn't" defects. *Files:* `actions.ts`, `HomeView.svelte`, `HotkeysView.svelte`, `system.rs`, `commands.rs`, `autostart.rs`, `SettingsView.svelte`. *Testing:* failing-zone apply toast, invalid-hotkey warning, forced registry failure. *Done when:* no operation claims success on failure; settings persistence is unambiguous.

**WP3 — Frozen-decision consistency (P2).** Fix F4, B-2, F6, D-3, F3, F5. *Why together:* all are D1–D6 behaviours that differ by surface or are bypassable. *Files:* `ZoneInspector.svelte`, `workflow.ts`, `actions.ts`, `commands.rs`, `native_loop.rs`, `ZoneCanvas.svelte`, `engine_core.rs`. *Testing:* D3 confirmation from every surface; tray/UI enable parity; overlay disabled behaviour. *Done when:* each frozen decision behaves identically everywhere.

**WP4 — Settings truthfulness (P2/P3).** Fix E-01, E-02, E-04, E-05, E-06, E-07, E-08, B-4, D-1, D-2. *Why together:* all concern controls that don't do what they say. *Files:* `SettingsView.svelte`, `config.rs`, `dto.rs`, `commands.rs`, `native_loop.rs`, `autostart.rs`, `CloseDialog.svelte`. *Dependencies:* D-1 touches native loop — coordinate with WP3's `NativeLoop` changes. *Testing:* every remaining control has a traced consumer; backend clamping test. *Done when:* no dead controls; docs match code; display changes handled or setting removed.

**WP5 — UI/UX polish (P3).** Fix U-02, U-04, U-05, U-06, U-07, U-08, U-09, B-3/B-6. *Why together:* independent presentation fixes with low regression risk. *Files:* `input.svelte`, `utils.ts`, `HotkeysView.svelte`, `AppLibrary.svelte`, `PickerDialog.svelte`, `EditorView.svelte`, `HomeView.svelte`, `WizardDialog.svelte`. *Testing:* visual re-verification at desktop and narrow widths. *Done when:* hotkey inputs sized correctly, labels accurate, all primary actions reachable ≥768px.

**WP6 — Test fidelity (P3).** Fix F8, R-05 (`.bad` recovery untested), frontend transport/component gaps. *Why together:* all improve confidence without changing behaviour. *Files:* `tests/config_file_roundtrip.rs`, new frontend tests. *Done when:* round-trip asserts every persisted field; `.bad` recovery covered; SSE transport tested.

**WP7 — BAT/CMD implementation (Planned).** Only after WP1–WP4. **The current plan has a fatal gap (R1):** with `launcher_show_console=true` (the default), the visible `cmd.exe` console is itself a new eligible window and would be arranged instead of the app. Before implementation the plan must: exclude new windows owned by `cmd.exe`/`conhost.exe`; force `needs_launch=true` when a launcher is set (R2 — current `zone_needs_launch` skips launch when a running EXE override matches); reject script paths containing `% ! & | ^ < >` rather than trying to escape them (R3); require `cmd.exe` basename for ComSpec (R4). The single-choke-point launch design (all paths funnel through `execute_zone_inner`) and the two-field model are sound — keep them. *Done when:* all 11 real-Windows BAT scenarios pass with sentinel proof, including console exclusion.

---

## E. Product readiness assessment

- **Functionally complete:** No. U-01 blocks app assignment; F2 breaks undo of profile operations; several Settings controls and two toggles are no-ops; BAT support is planned only. WP1–WP4 are required.
- **Reliable for everyday use:** Close. The architecture (single state owner, ordered shutdown, atomic config, SSE fix) is sound. Remaining reliability work is R-01 error propagation, D-1 display changes, and test fidelity (WP6).
- **Visually and interactively polished:** Not yet. The desktop editor looks good, but the Hotkeys screen is defective (U-02/U-03/U-04), narrow-width navigation is incomplete (U-07), and several dialogs lack confirmation (U-09). WP5.
- **Ready for a proper release:** After WP0–WP6, plus a decision on R-02 and a full CI + real-Windows verification pass. BAT support (WP7) can be a follow-up release if you want to ship the core first.

---

## F. What is already good — do not rewrite

- **Three-thread architecture** with one authoritative `Inner` on a serialized worker; HTTP handlers only observe oneshot snapshots — eliminates a class of races.
- **D1–D6 encoded in one exhaustive command layer**, with an exhaustive `emits_state_change` classification test and broad decision tests (~100 Rust tests + 6 frontend files).
- **The SSE/CPU fix (`1456d63`) is confirmed intact by two independent audits** — exhaustive read-only classification, deterministic barrier-based worker test, pure frontend policy, onOpen catch-up preserved. This is a model pattern; leave it alone.
- **Strict path-based window matching** — no `SearchPathW`, no exe-name/title fallback, ambiguity refused rather than guessed.
- **Duplicate-launch fix** via evolving process-path snapshot with pure testable helpers.
- **Honest apply outcomes** — intent separated from per-zone outcomes, `launched` flag on both success and failure, counts derived from outcomes. The defect is one UI consumer (B-1), not the model.
- **Config I/O** — atomic temp+rename, `.json.bad` preservation, v0 migration, deterministic serialization, self-healing `validate()`.
- **Ordered shutdown (§3.6)** — carefully sequenced with idempotent handle, guaranteed event wake-up, and join-before-destroy invariant.
- **Tray menu redesign** — all 7 entries and 3 Tools items wired, stable IDs, applied/selected markers.
- **Port-conflict dialog, clipboard owner fix, overlay rendering, hotkey fail-soft registration, native queue coalescing** — all sound.
- **Documentation is unusually thorough and self-critical** (architecture §13 records unmeasured workloads honestly). The gaps are drift/incompleteness, not fabrication.

---

**Bottom line:** one P1 hard blocker (U-01), one P1 security decision (R-02), one P1 data-integrity bug (F2), then a manageable set of P2/P3 fixes. No architectural rewrite is justified. Start with WP0 (your security decision) and WP1 (core stability), and work down the roadmap.
