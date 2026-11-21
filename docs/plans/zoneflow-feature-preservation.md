# ZoneFlow Studio — Feature Preservation Matrix

**Status:** Planning document. No implementation changes are authorized by this file.
**Date:** 2026-11-21
**Basis:** Direct source verification of all 20 `src/**/*.rs` files, `Cargo.toml`, docs, and the original `E:\Apps\Hotkeys\ScreenLayout.ahk` (416 lines). Line numbers are from the files as read.

**Rule:** No existing functionality is silently dropped. If something looks redundant, it is documented as a future consideration, not removed from the design.

---

## 1. Complete feature inventory

Legend: ✅ implemented · ⚠️ partial/degraded · ❌ absent

| # | Feature | Status | Evidence |
|---|---|---|---|
| 1 | Screen/work-area detection | ⚠️ primary only | `engine.rs:38-67` `SystemParametersInfoW(SPI_GETWORKAREA)`; fallback 1920×1040 at `:59-64`. AHK also monitor 1 only (`ScreenLayout.ahk:40`). Multi-monitor explicitly out of scope. |
| 2 | Zone geometry (percent rects, gap, px) | ✅ | `config.rs:84-109`; `engine_core.rs:33-74` (`zone_pixels`, `compute_zones`), `:143-154` snapping, `:228-292` boundaries. 15 unit tests. |
| 3 | Application library (Start Menu/registry/running) | ✅ | `rail.rs:485-516`, `:412-447` (.lnk), `:449-482` (registry), `:519-555` (tasklist), `:566-569` cache. |
| 4 | Custom applications (Browse/.exe) | ✅ | `dialogs.rs:323-367`, `rfd` at `:342-347`; entry `rail.rs:599-608`. |
| 5 | Process matching (exe/path) | ✅ | `engine.rs:244-246` normalize, `:269-297` enum_proc, `:315-334` find_window_by_path_min, `:161-191` process_path. |
| 6 | Browser/URL rules | ⚠️ logic present, text broken | Fields `config.rs:92-93`; execution `engine.rs:413-425`; `send_text` broken (`engine.rs:459-465`). |
| 7 | Macros/automation | ⚠️ minimal, text broken | `config.rs:94-103`; `engine.rs:427-520`; only `^t`,`^l`,`{ENTER}`,`{F5}`,`^X` recognized; ASCII-as-VK bug. Not in AHK original — Rust-only addition. |
| 8 | Hotkeys (17 + Ctrl+drag + re-registration) | ✅ | `config.rs:319-333`; `system.rs:99-150`, `:177-220`; `engine.rs:652-661`; `mod.rs:289-304`, `:400-444`. |
| 9 | Drag-to-snap overlay | ⚠️ works but grabs child HWND | `engine.rs:675-687` `WindowFromPoint` (no GetAncestor); `overlay.rs:85-175`; `mod.rs:400-444`. Missing AHK guards. |
| 10 | Window snapping/positioning (DWM) | ✅ | `engine.rs:69-159` (invisible borders, corner pref, SetWindowPos retries); `engine_core.rs:304-323`. Good AHK parity. |
| 11 | Application launching | ⚠️ path ignored | `engine.rs:387-403`; `Command::new(&zone.exe)` at `:392` — `zone.path` used only for matching. AHK launches full path (`ScreenLayout.ahk:154`). |
| 12 | Save/load + config migration | ⚠️ migration yes, atomicity no | `config.rs:8`, `:347-353`, `:355-402`; `.json.bad` recovery `:379-385`; non-atomic write admitted `:396-397`. |
| 13 | Undo/redo | ✅ | `history.rs:12-69`; `state.rs:120-124`; depth `config.rs:223` (60). |
| 14 | System tray | ⚠️ no profile switching | `system.rs:56-72` (Settings/Re-tile/Reload/Exit), `:161-169`; close-to-tray `dialogs.rs:588-674`. |
| 15 | Startup/autostart | ✅ (auto-launch opt-in) | `autostart.rs:7-62`; `system.rs:78`; `views.rs:71-81`; auto-launch `config.rs:229` default false, worker `mod.rs:1115-1181`. |
| 16 | Console/execution logs | ✅ (simulation) | `mod.rs:74-76`, `:276-284`, `:446-465`, `:777-889`; file log `main.rs:12-38`. `run_sequence` is simulation only. |
| 17 | Settings | ⚠️ most config-only | `config.rs:179-213` (21 fields); UI exposes only auto_start, auto_launch, close_action, gap, grid cols/rows, key_delay, hotkeys. |
| 18 | Views/tabs | ✅ | `state.rs:97-117`, `:210-215`; `mod.rs:947-960`; `rail.rs:578-590`; `inspector.rs:120-135`; `dialogs.rs:255-272`. |
| 19 | Profiles (CRUD/switch) | ✅ | `config.rs:150-156`; `dialogs.rs:545-578`; `mod.rs:962-997`; `rail.rs:736-758`. |
| 20 | Apply / disable layout | ⚠️ apply yes, disable ❌ | Save & Apply `action_bar.rs:46-61` → `mod.rs:312-327`. No disable concept anywhere (grep: only `autostart.rs:14` comment). |

---

## 1a. Data ownership — profile-specific assignments (verified)

The requirement "application assignments belong to individual profiles, not the global library" is **already satisfied by the current data model**. Verified against source:

| Data | Owner | Evidence |
|---|---|---|
| Zone geometry + app binding + all rules | `Zone` inside `Profile.zones` | `config.rs:84-109`, `:150-156` |
| Profiles | `AppConfig.profiles: BTreeMap<String, Profile>` | `config.rs:272` |
| Assignment write path | `attach_app` → `active_mut().zones` only | `mod.rs:585-596` |
| Global library | Runtime enumeration, **not persisted** | `rail.rs:485-569` |

Consequences:
- Same app can be in different zones in different profiles — each profile owns independent zones.
- Editing one profile cannot modify another — separate `Profile` structs in the map.
- Browser rules, macros, launch settings are zone-scoped → profile-scoped.
- Removing an assignment clears zone fields; the library entry is untouched (library is not persisted).
- **No data-model rewrite required.** Remaining work is UI clarity (show assignments per profile) and the workflow rules in product workflow §2a/§3a.

---

## 2. Feature preservation matrix

| Existing feature | Current implementation | Proposed location/workflow | Required changes |
|---|---|---|---|
| Work-area detection | `engine.rs:38-67`, primary only | Home shows work-area info; engine unchanged | None (out of scope: multi-monitor) |
| Zone geometry | `engine_core.rs` pure math + canvas painter | Layout Editor canvas | Shell hands exact rect to canvas; remove `stage_lower_h` guessing |
| App library | `rail.rs` enumeration + cleanup | Editor left rail (Apps tab) | Keep; polish heuristics only |
| Custom apps | `dialogs.rs` Browse + `rfd` | Picker Browse tab | Keep; add path validation (future) |
| Process matching | `engine.rs` exe/path | App rules (contextual inspector) | Keep; fix launch to use path (bug) |
| Browser/URL rules | `engine.rs:413-425` | App rules (Browser tab) | **Fix SendInput text injection** (bug) |
| Macros | `engine.rs:427-520` | App rules (Automation tab) | **Fix SendInput**; expand parser (bug) |
| Hotkeys | `system.rs` + `config.rs` | Settings view + tray + global | Keep; fix `LaunchApp(i)` to use zone id not list index |
| Drag-to-snap | `overlay.rs` + `mod.rs:400-444` | Global gesture | **Fix HWND selection** (GetAncestor) + add AHK guards |
| Window snapping | `engine.rs:69-159` | Apply/retile/hotkeys | Keep; wire config retries into drag path |
| App launching | `engine.rs:387-403` | Apply / hotkeys / auto-launch | **Fix path-based launch** (bug); add CREATE_NO_WINDOW |
| Save/load/migration | `config.rs` | Save verb (editor) | Keep; fix docs ("atomic" overstated); consider true atomic write |
| Undo/redo | `history.rs` | Editor toolbar + shortcuts | Keep; cache dirty flag |
| System tray | `system.rs:56-72` | Tray-first profile switching | **Add profile submenu + Disable toggle + Open editor** |
| Startup/autostart | `autostart.rs` | Settings + startup | Keep; remove dead `is_auto_start()` or wire it |
| Console/logs | `mod.rs` + `main.rs` | Collapsible console in editor | Keep; feed real execution results, not only simulation |
| Settings | `views.rs` | Settings view | Keep; decide which config-only tunables to expose |
| Views/tabs | `state.rs` + modules | Home + Editor + Hotkeys + Settings | Add Home view; make inspector contextual |
| Profiles CRUD | `mod.rs`/`rail.rs`/`dialogs.rs` | Home + rail + tray | Keep; separate rail (draft) from tray/Home (apply) |
| Apply/disable | Save & Apply only | Apply verb + Disable verb | **Split Save and Apply; add Disable** (new) |
| Profile-specific assignments | Already in model (`Profile.zones` → `Zone.exe/path/rules`) | Same model; UI shows assignments per profile | No data change; add assignment visibility (§1a) |
| Global app library | Runtime enumeration, no assignments stored | Reusable entries only | No change; keep separate from assignments |
| Apply mode (launch vs arrange) | Current Apply = arrange only (`save_apply` retiles, no launch) | **(D1)** Apply (arrange running, default) + Launch & Arrange (explicit) | Add Launch & Arrange action; keep Apply as-is |
| Launch button | Console simulation (`run_sequence`) | **(D5)** real execution + separate Preview | Replace simulation with real execution; add Preview action |
| Disable layout | Absent | **(D4)** runtime-only flag, suspends all automation | New feature; resets to enabled on restart |
| Same app in multiple zones (one profile) | Backend returns single HWND per exe/path (`engine.rs:315-334`); `retile` moves same window twice (`engine.rs:522-534`) | Warn in editor; no reliable multi-window support | Document limitation (product §3b); proper multi-window matching deferred |

---

## 3. Confirmed bugs that must be fixed (functional, not GUI)

| ID | Bug | Evidence | Impact |
|---|---|---|---|
| B1 | SendInput macros broken — ASCII codes used as VK codes | `engine.rs:459-465`, `:477` | URLs and macros produce wrong keys |
| B2 | Path-based launching broken — path used for matching, not launching | `engine.rs:392` vs `:397` | Apps not on PATH fail to launch |
| B3 | Drag-to-snap grabs child HWND via `WindowFromPoint` | `engine.rs:675-687` | Wrong window may be moved |
| B4 | Drag lacks AHK guards (title-bar, class, threshold) | `mod.rs:400-444` vs AHK `:328-350` | Triggers on taskbar/client clicks |
| B5 | Launch button is console simulation only | `mod.rs:777-889` | Misleading; must perform or be renamed |
| B6 | Hotkeys read disk config; auto-launch reads draft | `mod.rs:356-385` vs `:1118` | Inconsistent behavior with unsaved edits |
| B7 | `LOADED_SETTINGS` frozen at first load | `config.rs:252`, `:359` | Runtime setting edits never propagate |
| B8 | Profile switch doesn't apply layout | `rail.rs:748` | Active layout lags selection |
| B9 | `LaunchApp(i)` uses list index, not zone id | `mod.rs:357` | Hotkey targets wrong zone after edits |
| B10 | `restore_active` ignores maximized state | `engine.rs:547-570` vs AHK `:283-284` | Win+Down parity gap |
| B11 | Config writes not atomic (docs overstate) | `config.rs:396-401` | Crash window; docs wrong |
| B12 | `is_auto_start()` dead with false doc comment | `autostart.rs:64-92` | UI can misreport state |

---

## 4. Redundant / future-consideration items (NOT removed)

| Item | Why it looks redundant | Decision |
|---|---|---|
| `Profile.hotkey` field | Dead — global hotkeys in `AppConfig.hotkeys` | Keep field; wire to profile switching in a future iteration or document as reserved. Do not delete silently. |
| Compatibility wrappers in `engine.rs`/`engine_core.rs` | Thin wrappers around config-aware variants | Keep for now; remove only after all call sites migrated and tests confirm. |
| `run_sequence` simulation | Doesn't execute | **(D5 — APPROVED)** Launch performs real execution; keep a separate Preview action for inspecting steps. No simulated success. |
| Config-only tunables (~17) | Not editable in UI | Keep in config; expose gradually. Do not remove. |
| `clear_layout` name | Maps to Retile, not clear | Rename action internally to `retile_all` in a future cleanup; preserve hotkey behavior. |
| Unicode-free Lucide icons | Already correct | No change. |

---

## 4a. Same application in multiple zones — verified limitation

- **Different profiles:** same app in different zones is fully supported (each profile owns independent zones — §1a). Preserve this.
- **Within one profile:** `find_window_by_path_min` (`engine.rs:315-334`) returns a **single** HWND (first exe match or exact path match). `retile` (`engine.rs:522-534`) calls it per zone, so two zones with the same exe/path receive the **same HWND** — the window is moved twice, last zone wins. `execute_zone` behaves similarly.
- **Safest initial behavior:** detect duplicate exe/path within a profile, warn the user ("only one window can be placed per app; last zone will win"), do not claim multi-window support, do not silently move the same window without warning. Keep the model permissive (user may intend two instances the backend cannot distinguish).
- **Deferred capability:** proper multi-window matching (enumerate all windows for an exe/path, track placed HWNDs, assign distinct instances) — separate future work, not in scope.

---

## 5. Compatibility considerations

- **Config schema v1** must remain compatible; v0 migration preserved (`config.rs:347-353`).
- **Hotkey bindings** in existing configs must keep working (17 defaults, `Win`→`Meta` normalization).
- **Close-to-tray / remember choice** behavior preserved (`dialogs.rs:588-674`).
- **Original AHK script** remains untouched at `E:\Apps\Hotkeys\ScreenLayout.ahk`.
- **No new dependencies** for the planning scope; bug fixes use existing crates (`windows`, `tracing`, `rfd`).
- **Multi-monitor**: explicitly out of scope — no work planned or required.
