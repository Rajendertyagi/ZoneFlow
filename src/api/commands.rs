//! Command payloads and their D1–D6 behavior.
//!
//! Every mutating operation is executed here, serialized on the command worker
//! thread. HTTP handlers never touch authoritative state directly — they send a
//! `Command` and await a oneshot reply.

use super::dto::{
    ApplyResultDto, AssignDto, MergeDto, PreviewDto, RectDto, SplitDto, ZoneDto,
    ZoneGeometryUpdateDto,
};
use super::state::Inner;
use super::ApiError;
use crate::apps::{self, AppEntry};
use crate::config::{Profile, Zone};
use crate::native_loop::{NativeNotifier, ShutdownHandle};
use crate::system::Action;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

/// Every operation the worker can perform. Covers all §5 endpoints plus the
/// native-side `Action` / `MoveWindow` inputs and shutdown.
#[derive(Debug)]
pub enum CommandPayload {
    // --- session / state ---
    StateGet,
    StatusGet,
    Shutdown,

    // --- profiles (D6) ---
    ProfilesList,
    ProfileCreate {
        name: String,
    },
    ProfileRename {
        name: String,
        new_name: String,
    },
    ProfileDelete {
        name: String,
    },
    ProfileDuplicate {
        name: String,
        new_name: String,
    },
    ProfileLoad {
        name: String,
        discard_dirty: bool,
    },
    ProfileSelect {
        name: String,
        discard_dirty: bool,
    },
    ProfileApply {
        name: String,
        discard_dirty: bool,
    },

    // --- zones ---
    ZonesList,
    ZoneCreate {
        zone: ZoneDto,
    },
    ZoneUpdate {
        id: i32,
        zone: ZoneDto,
    },
    ZonesBatchUpdate {
        updates: Vec<ZoneGeometryUpdateDto>,
    },
    ZoneDelete {
        id: i32,
    },
    ZoneSplit {
        id: i32,
        spec: SplitDto,
    },
    ZoneMerge {
        spec: MergeDto,
    },
    ZoneAssign {
        id: i32,
        spec: AssignDto,
    },
    ZoneUnassign {
        id: i32,
    },
    Undo,
    Redo,

    // --- applications ---
    AppsList {
        source: String,
    },
    AppsBrowse,

    // --- actions (D1–D5) ---
    Save,
    Apply,
    LaunchArrange,
    Preview,
    Revert,
    Disable,
    Enable,

    // --- settings / hotkeys / logs ---
    SettingsGet,
    SettingsSet {
        settings: super::dto::SettingsDto,
    },
    /// AppConfig-level options (autostart + close behavior) — no other endpoint
    /// exposes these, so the web Settings view needs its own command.
    AppOptionsSet {
        options: super::dto::AppOptionsDto,
    },
    HotkeysGet,
    HotkeysSet {
        hotkeys: BTreeMap<String, String>,
    },
    LogsGet {
        since: u64,
    },

    // --- native inputs ---
    Action(Action),
    MoveWindow {
        hwnd: isize,
        zone_id: i32,
    },
}

/// Non-authoritative context passed to the handler: side-effect channels and
/// short-lived caches that do not belong in the persisted draft.
pub struct CommandContext<'a> {
    pub notifier: Arc<NativeNotifier>,
    pub shutdown: ShutdownHandle,
    pub editor_url: Arc<String>,
    /// 2-second TTL cache for running-app enumeration (§5.4).
    pub running_cache: &'a mut Option<(Instant, Vec<AppEntry>)>,
}

/// D2 unified dirty contract. Clean → proceed. Dirty + discard → revert the
/// draft, then proceed. Dirty + no discard → 409 with **no state change**.
fn guard_dirty(inner: &mut Inner, discard_dirty: bool) -> Result<(), ApiError> {
    if !inner.is_dirty() {
        return Ok(());
    }
    if discard_dirty {
        inner.revert_draft();
        Ok(())
    } else {
        Err(ApiError::DirtyState)
    }
}

fn ok() -> Result<Value, ApiError> {
    Ok(json!({ "ok": true }))
}

fn not_found(what: &str) -> ApiError {
    ApiError::NotFound(what.to_string())
}

fn bad_request(msg: impl Into<String>) -> ApiError {
    ApiError::BadRequest(msg.into())
}

/// Apply a profile's layout without mutating the draft (D1 "Apply does not
/// change the draft"). `launch=true` is Launch & Arrange; `launch=false` is
/// Apply (arrange running windows only).
fn apply_profile(inner: &mut Inner, name: &str, launch: bool) -> Result<ApplyResultDto, ApiError> {
    if !inner.draft.profiles.contains_key(name) {
        return Err(not_found("profile"));
    }
    let mut cfg = inner.draft.clone();
    cfg.active_profile = name.to_string();
    let area = crate::engine::work_area();

    let mut messages = Vec::new();
    if launch {
        // D5: real execution, never simulated success.
        let zones = crate::engine_core::compute_zones(cfg.active(), area, cfg.gap);
        for z in &cfg.active().zones {
            if let Some(rect) = crate::engine_core::zone_by_id(&zones, z.id) {
                match crate::engine::execute_zone_with_config(z, rect, &cfg) {
                    Ok(m) => messages.push(m),
                    Err(e) => {
                        tracing::warn!(profile = %name, zone = z.id, error = %e, "launch/arrange failed");
                        messages.push(format!("{}: {}", z.exe, e));
                    }
                }
            }
        }
    } else {
        crate::engine::retile(&cfg);
        messages.push(format!("arranged running apps for {name}"));
    }

    inner.applied_profile = name.to_string();
    inner.selected_profile = name.to_string();
    inner.log("info", format!("Applied profile {name}"));
    Ok(ApplyResultDto {
        profile: name.to_string(),
        message: messages.join("; "),
        launched: launch,
    })
}

fn inactive_result(inner: &Inner) -> Value {
    json!({
        "ok": true,
        "applied": false,
        "inactive": true,
        "reason": "layout_disabled",
        "profile": inner.selected_profile,
    })
}

/// Execute one command against authoritative state.
pub fn handle(
    payload: CommandPayload,
    inner: &mut Inner,
    ctx: &mut CommandContext<'_>,
) -> Result<Value, ApiError> {
    match payload {
        CommandPayload::StateGet => {
            let area = RectDto::from(&crate::engine::work_area());
            Ok(serde_json::to_value(inner.state_snapshot(area))
                .map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::StatusGet => {
            let area = RectDto::from(&crate::engine::work_area());
            let status = inner.state_snapshot(area).status;
            Ok(serde_json::to_value(status).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::Shutdown => {
            ctx.shutdown.trigger();
            ok()
        }

        // ------------------------------ profiles ---------------------------
        CommandPayload::ProfilesList => {
            let rows: Vec<_> = inner
                .draft
                .profiles
                .values()
                .map(|p| {
                    super::dto::ProfileSummary::new(
                        p,
                        &inner.applied_profile,
                        &inner.selected_profile,
                    )
                })
                .collect();
            Ok(serde_json::to_value(rows).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::ProfileCreate { name } => {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(bad_request("profile name is empty"));
            }
            if inner.draft.profiles.contains_key(&name) {
                return Err(bad_request("profile already exists"));
            }
            inner.draft.profiles.insert(
                name.clone(),
                Profile {
                    name: name.clone(),
                    hotkey: String::new(),
                    zones: Vec::new(),
                },
            );
            inner.push_history();
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", format!("Created profile {name}"));
            Ok(json!({ "name": name }))
        }
        CommandPayload::ProfileRename { name, new_name } => {
            let new_name = new_name.trim().to_string();
            if new_name.is_empty() {
                return Err(bad_request("new profile name is empty"));
            }
            if !inner.draft.profiles.contains_key(&name) {
                return Err(not_found("profile"));
            }
            if name != new_name && inner.draft.profiles.contains_key(&new_name) {
                return Err(bad_request("profile already exists"));
            }
            let mut profile = inner.draft.profiles.remove(&name).unwrap();
            profile.name = new_name.clone();
            inner.draft.profiles.insert(new_name.clone(), profile);
            if inner.applied_profile == name {
                inner.applied_profile = new_name.clone();
            }
            if inner.selected_profile == name {
                inner.selected_profile = new_name.clone();
            }
            if inner.draft.active_profile == name {
                inner.draft.active_profile = new_name.clone();
            }
            inner.push_history();
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", format!("Renamed profile {name} to {new_name}"));
            ok()
        }
        CommandPayload::ProfileDelete { name } => {
            if !inner.draft.profiles.contains_key(&name) {
                return Err(not_found("profile"));
            }
            if inner.draft.profiles.len() == 1 {
                return Err(bad_request("cannot delete the last profile"));
            }
            inner.draft.profiles.remove(&name);
            let fallback = inner
                .draft
                .profiles
                .keys()
                .next()
                .cloned()
                .unwrap_or_default();
            if inner.applied_profile == name {
                inner.applied_profile.clear(); // D: clear Applied dimension
            }
            if inner.selected_profile == name {
                inner.selected_profile = fallback.clone();
            }
            if inner.draft.active_profile == name {
                inner.draft.active_profile = fallback;
            }
            inner.draft.validate();
            inner.push_history();
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", format!("Deleted profile {name}"));
            ok()
        }
        CommandPayload::ProfileDuplicate { name, new_name } => {
            let new_name = new_name.trim().to_string();
            if new_name.is_empty() {
                return Err(bad_request("new profile name is empty"));
            }
            let mut profile = inner
                .draft
                .profiles
                .get(&name)
                .cloned()
                .ok_or_else(|| not_found("profile"))?;
            if inner.draft.profiles.contains_key(&new_name) {
                return Err(bad_request("profile already exists"));
            }
            profile.name = new_name.clone();
            inner.draft.profiles.insert(new_name.clone(), profile);
            inner.push_history();
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", format!("Duplicated profile {name} as {new_name}"));
            ok()
        }
        CommandPayload::ProfileLoad {
            name,
            discard_dirty,
        } => {
            guard_dirty(inner, discard_dirty)?;
            if !inner.draft.profiles.contains_key(&name) {
                return Err(not_found("profile"));
            }
            // Rail: load the target profile's zones into the draft for editing.
            inner.draft.active_profile = name.clone();
            inner.selected_profile = name.clone();
            inner.push_history();
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", format!("Loaded profile {name} into draft"));
            ok()
        }
        CommandPayload::ProfileSelect {
            name,
            discard_dirty,
        } => {
            guard_dirty(inner, discard_dirty)?;
            if !inner.draft.profiles.contains_key(&name) {
                return Err(not_found("profile"));
            }
            // Home: selection only — no draft change, no apply (D6).
            inner.selected_profile = name.clone();
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", format!("Selected profile {name}"));
            ok()
        }
        CommandPayload::ProfileApply {
            name,
            discard_dirty,
        } => {
            guard_dirty(inner, discard_dirty)?;
            if !inner.draft.profiles.contains_key(&name) {
                return Err(not_found("profile"));
            }
            inner.selected_profile = name.clone();
            if !inner.layout_enabled {
                // D6: tray select-only while disabled (D4 suspends automation).
                ctx.notifier.rebuild_menu(inner);
                inner.log("info", format!("Selected profile {name} (layout disabled)"));
                return Ok(inactive_result(inner));
            }
            let result = apply_profile(inner, &name, false)?;
            ctx.notifier.rebuild_menu(inner);
            Ok(serde_json::to_value(result).map_err(|e| ApiError::Internal(e.to_string()))?)
        }

        // ------------------------------- zones -----------------------------
        CommandPayload::ZonesList => {
            let zones: Vec<ZoneDto> = inner
                .draft
                .active()
                .zones
                .iter()
                .map(ZoneDto::from)
                .collect();
            Ok(serde_json::to_value(zones).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::ZoneCreate { zone } => {
            let mut z: Zone = zone.into();
            let next = inner
                .draft
                .active()
                .zones
                .iter()
                .map(|z| z.id)
                .max()
                .unwrap_or(0)
                + 1;
            z.id = next;
            inner.draft.active_mut().zones.push(z.clone());
            inner.selected_zone = next;
            inner.push_history();
            inner.log("info", format!("Created zone {next}"));
            Ok(serde_json::to_value(ZoneDto::from(&z))
                .map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::ZoneUpdate { id, zone } => {
            let mut z: Zone = zone.into();
            z.id = id;
            let zones = &mut inner.draft.active_mut().zones;
            let slot = zones
                .iter_mut()
                .find(|z| z.id == id)
                .ok_or_else(|| not_found("zone"))?;
            *slot = z.clone();
            inner.push_history();
            Ok(serde_json::to_value(ZoneDto::from(&z))
                .map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::ZonesBatchUpdate { updates } => {
            {
                let zones = &mut inner.draft.active_mut().zones;
                // Validate every id before touching anything: a single unknown
                // id must leave the draft completely unchanged (no partial
                // mutation) and must not create an undo point.
                for u in &updates {
                    if !zones.iter().any(|z| z.id == u.id) {
                        return Err(not_found("zone"));
                    }
                }
                for u in &updates {
                    if let Some(z) = zones.iter_mut().find(|z| z.id == u.id) {
                        z.x = u.x;
                        z.y = u.y;
                        z.w = u.w;
                        z.h = u.h;
                    }
                }
            }
            // One history entry for the entire batch, regardless of how many
            // zones were affected. `push_history` is a no-op when nothing
            // actually changed, matching every other mutating command.
            inner.push_history();
            let updated: Vec<ZoneDto> = updates
                .iter()
                .map(|u| {
                    inner
                        .draft
                        .active()
                        .zones
                        .iter()
                        .find(|z| z.id == u.id)
                        .map(ZoneDto::from)
                        .unwrap()
                })
                .collect();
            Ok(serde_json::to_value(updated).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::ZoneDelete { id } => {
            let zones = &mut inner.draft.active_mut().zones;
            if zones.len() == 1 {
                return Err(bad_request("cannot delete the last zone"));
            }
            let pos = zones
                .iter()
                .position(|z| z.id == id)
                .ok_or_else(|| not_found("zone"))?;
            zones.remove(pos);
            if inner.selected_zone == id {
                inner.selected_zone = -1;
            }
            inner.push_history();
            inner.log("info", format!("Deleted zone {id}"));
            ok()
        }
        CommandPayload::ZoneSplit { id, spec } => {
            let ratio = spec.ratio.clamp(0.05, 0.95);
            let zones = &mut inner.draft.active_mut().zones;
            let pos = zones
                .iter()
                .position(|z| z.id == id)
                .ok_or_else(|| not_found("zone"))?;
            let original = zones[pos].clone();
            let new_id = zones.iter().map(|z| z.id).max().unwrap_or(0) + 1;
            let mut first = original.clone();
            let mut second = original.clone();
            second.id = new_id;
            if spec.axis.eq_ignore_ascii_case("x") {
                let w1 = original.w * ratio;
                first.w = w1;
                second.x = original.x + w1;
                second.w = original.w - w1;
            } else {
                let h1 = original.h * ratio;
                first.h = h1;
                second.y = original.y + h1;
                second.h = original.h - h1;
            }
            zones[pos] = first.clone();
            zones.insert(pos + 1, second.clone());
            inner.selected_zone = new_id;
            inner.push_history();
            inner.log("info", format!("Split zone {id} into {id} and {new_id}"));
            Ok(json!({
                "first": ZoneDto::from(&first),
                "second": ZoneDto::from(&second),
            }))
        }
        CommandPayload::ZoneMerge { spec } => {
            if spec.source_id == spec.target_id {
                return Err(bad_request("cannot merge a zone into itself"));
            }
            let zones = &mut inner.draft.active_mut().zones;
            let src_pos = zones
                .iter()
                .position(|z| z.id == spec.source_id)
                .ok_or_else(|| not_found("source zone"))?;
            let tgt_pos = zones
                .iter()
                .position(|z| z.id == spec.target_id)
                .ok_or_else(|| not_found("target zone"))?;
            let source = zones[src_pos].clone();
            let mut target = zones[tgt_pos].clone();
            let x = source.x.min(target.x);
            let y = source.y.min(target.y);
            let r = (source.x + source.w).max(target.x + target.w);
            let b = (source.y + source.h).max(target.y + target.h);
            target.x = x;
            target.y = y;
            target.w = r - x;
            target.h = b - y;
            // Remove the higher index first so the other position stays valid.
            let remove_first = src_pos.max(tgt_pos);
            zones.remove(remove_first);
            let keep_pos = src_pos.min(tgt_pos);
            zones[keep_pos] = target.clone();
            if inner.selected_zone == spec.source_id {
                inner.selected_zone = spec.target_id;
            }
            inner.push_history();
            inner.log(
                "info",
                format!("Merged zone {} into {}", spec.source_id, spec.target_id),
            );
            Ok(serde_json::to_value(ZoneDto::from(&target))
                .map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::ZoneAssign { id, spec } => {
            let zones = &mut inner.draft.active_mut().zones;
            let z = zones
                .iter_mut()
                .find(|z| z.id == id)
                .ok_or_else(|| not_found("zone"))?;
            // D3: preserve geometry, id and position; reset *all* automation
            // rules to their `Zone::default()` values — including launch mode
            // and the macro key delay, which are application-specific rules too.
            // The application library entry is never touched.
            z.label = spec.label;
            z.exe = spec.exe;
            z.path = spec.path;
            z.launch_mode = crate::config::LaunchMode::default();
            z.url.clear();
            z.title_match.clear();
            z.tab_behavior = crate::config::TabBehavior::default();
            z.macro_enabled = false;
            z.trigger_title.clear();
            z.macro_text.clear();
            z.notes.clear();
            z.key_delay = crate::config::Zone::default().key_delay;
            let dto = ZoneDto::from(&*z);
            inner.push_history();
            inner.log("info", format!("Assigned {} to zone {id}", dto.exe));
            Ok(serde_json::to_value(dto).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::ZoneUnassign { id } => {
            let zones = &mut inner.draft.active_mut().zones;
            let z = zones
                .iter_mut()
                .find(|z| z.id == id)
                .ok_or_else(|| not_found("zone"))?;
            z.label.clear();
            z.exe.clear();
            z.path.clear();
            z.url.clear();
            z.title_match.clear();
            z.macro_enabled = false;
            z.trigger_title.clear();
            z.macro_text.clear();
            z.notes.clear();
            inner.push_history();
            inner.log("info", format!("Cleared assignment on zone {id}"));
            ok()
        }
        CommandPayload::Undo => {
            if !inner.undo() {
                return Err(bad_request("nothing to undo"));
            }
            inner.log("info", "Undo");
            ok()
        }
        CommandPayload::Redo => {
            if !inner.redo() {
                return Err(bad_request("nothing to redo"));
            }
            inner.log("info", "Redo");
            ok()
        }

        // ------------------------------ apps -------------------------------
        CommandPayload::AppsList { source } => {
            let ttl = std::time::Duration::from_secs(inner.draft.settings.running_apps_ttl_secs);
            let list = if source.eq_ignore_ascii_case("running") {
                let fresh = ctx
                    .running_cache
                    .as_ref()
                    .is_some_and(|(at, _)| at.elapsed() < ttl);
                if !fresh {
                    let found = apps::enumerate_running_apps();
                    *ctx.running_cache = Some((Instant::now(), found));
                }
                ctx.running_cache
                    .as_ref()
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            } else {
                apps::enumerate_installed_apps()
            };
            let dtos = Inner::app_dtos(&list);
            Ok(serde_json::to_value(dtos).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::AppsBrowse => {
            let picked = apps::browse();
            let dtos = Inner::app_dtos(&picked);
            Ok(serde_json::to_value(dtos).map_err(|e| ApiError::Internal(e.to_string()))?)
        }

        // ----------------------------- actions -----------------------------
        CommandPayload::Save => {
            inner.save().map_err(ApiError::Internal)?;
            inner.log("info", "Saved configuration");
            ok()
        }
        CommandPayload::Apply => {
            guard_dirty(inner, false)?;
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            let name = inner.selected_profile.clone();
            let result = apply_profile(inner, &name, false)?;
            Ok(serde_json::to_value(result).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::LaunchArrange => {
            guard_dirty(inner, false)?;
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            let name = inner.selected_profile.clone();
            let result = apply_profile(inner, &name, true)?;
            Ok(serde_json::to_value(result).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::Preview => {
            // D5: inspect the steps without executing anything.
            let name = inner.selected_profile.clone();
            let mut cfg = inner.draft.clone();
            cfg.active_profile = name.clone();
            let area = crate::engine::work_area();
            let zones = crate::engine_core::compute_zones(cfg.active(), area, cfg.gap);
            let steps: Vec<String> = cfg
                .active()
                .zones
                .iter()
                .map(|z| {
                    let r = crate::engine_core::zone_by_id(&zones, z.id)
                        .map(|r| format!("{},{} {}x{}", r.x, r.y, r.w, r.h))
                        .unwrap_or_else(|| "?".into());
                    format!("launch/attach {} -> {r}", z.exe)
                })
                .collect();
            let dto = PreviewDto {
                profile: name,
                steps,
                executed: false,
            };
            Ok(serde_json::to_value(dto).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::Revert => {
            inner.revert_draft();
            inner.log("info", "Reverted draft to saved state");
            ok()
        }
        CommandPayload::Disable => {
            inner.layout_enabled = false;
            ctx.notifier.set_toggle_label(false);
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", "Layout disabled (runtime only)");
            ok()
        }
        CommandPayload::Enable => {
            inner.layout_enabled = true;
            ctx.notifier.set_toggle_label(true);
            ctx.notifier.rebuild_menu(inner);
            inner.log("info", "Layout enabled");
            // Restore automation: re-apply the selected profile with the normal
            // Apply behavior, but never apply a dirty draft (D2).
            if !inner.is_dirty() {
                let name = inner.selected_profile.clone();
                if !name.is_empty() {
                    match apply_profile(inner, &name, false) {
                        Ok(_) => {}
                        Err(e) => tracing::warn!(error = ?e, "re-apply on enable failed"),
                    }
                }
            } else {
                tracing::warn!("enable skipped re-apply: draft has unsaved changes");
            }
            ok()
        }

        // ------------------------ settings / hotkeys -----------------------
        CommandPayload::SettingsGet => {
            let dto = super::dto::SettingsDto::from(&inner.draft.settings);
            Ok(serde_json::to_value(dto).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::SettingsSet { settings } => {
            inner.draft.settings = settings.into();
            crate::autostart::set_auto_start(inner.draft.auto_start);
            inner.push_history();
            inner.log("info", "Settings updated");
            ok()
        }
        CommandPayload::AppOptionsSet { options } => {
            // Only touch the registry when the value actually changes — avoids a
            // redundant HKCU write on every Settings save (and keeps tests inert
            // when the value is unchanged).
            if inner.draft.auto_start != options.auto_start {
                crate::autostart::set_auto_start(options.auto_start);
            }
            inner.draft.auto_start = options.auto_start;
            inner.draft.close_action = options.close_action;
            inner.draft.gap = options.gap;
            inner.push_history();
            inner.log("info", "Application options updated");
            ok()
        }
        CommandPayload::HotkeysGet => {
            let rows: Vec<super::dto::HotkeyDto> = inner
                .draft
                .hotkeys
                .iter()
                .map(|(name, binding)| super::dto::HotkeyDto {
                    name: name.clone(),
                    binding: binding.clone(),
                })
                .collect();
            Ok(serde_json::to_value(rows).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        CommandPayload::HotkeysSet { hotkeys } => {
            inner.draft.hotkeys = hotkeys.clone();
            inner.push_history();
            ctx.notifier.reregister_hotkeys(&hotkeys);
            inner.log("info", "Hotkeys updated");
            ok()
        }
        CommandPayload::LogsGet { since } => {
            let rows = inner.logs_since(since);
            Ok(serde_json::to_value(rows).map_err(|e| ApiError::Internal(e.to_string()))?)
        }

        // --------------------------- native inputs -------------------------
        CommandPayload::Action(action) => handle_action(action, inner, ctx),
        CommandPayload::MoveWindow { hwnd, zone_id } => {
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            // Use the same geometry the overlay displayed (Inner::drag_zones),
            // otherwise a Home select can make the drop snap to a different
            // profile's rectangles than the ones the user saw highlighted.
            let zones = inner.drag_zones();
            if let Some(rect) = crate::engine_core::zone_by_id(&zones, zone_id) {
                crate::engine::move_window_raw(hwnd, rect);
            } else {
                tracing::warn!(zone_id, "drag release referenced an unknown zone");
            }
            ok()
        }
    }
}

/// Handle an `Action` produced by `System::poll()` on the native thread.
fn handle_action(
    action: Action,
    inner: &mut Inner,
    ctx: &mut CommandContext<'_>,
) -> Result<Value, ApiError> {
    match action {
        Action::Exit => {
            ctx.shutdown.trigger();
            ok()
        }
        Action::ShowSettings | Action::OpenEditor => {
            let url = ctx.editor_url.to_string();
            crate::session::open_editor_url(&url);
            ok()
        }
        Action::ToggleEnabled => {
            inner.layout_enabled = !inner.layout_enabled;
            ctx.notifier.set_toggle_label(inner.layout_enabled);
            ctx.notifier.rebuild_menu(inner);
            inner.log(
                "info",
                if inner.layout_enabled {
                    "Layout enabled"
                } else {
                    "Layout disabled (runtime only)"
                },
            );
            ok()
        }
        Action::ProfileApply(name) => {
            if !inner.draft.profiles.contains_key(&name) {
                return Err(not_found("profile"));
            }
            inner.selected_profile = name.clone();
            if !inner.layout_enabled {
                ctx.notifier.rebuild_menu(inner);
                return Ok(inactive_result(inner));
            }
            let result = apply_profile(inner, &name, false)?;
            ctx.notifier.rebuild_menu(inner);
            Ok(serde_json::to_value(result).map_err(|e| ApiError::Internal(e.to_string()))?)
        }
        Action::Reload => {
            inner.reload_from_disk();
            inner.log("info", "Reloaded config from disk");
            ok()
        }
        Action::LaunchApp(i) => {
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            let mut cfg = inner.draft.clone();
            cfg.active_profile = inner.selected_profile.clone();
            if let Some(z) = cfg.active().zones.get(i).cloned() {
                let area = crate::engine::work_area();
                let rect = crate::engine_core::zone_pixels(&z, area, cfg.gap);
                match crate::engine::execute_zone_with_config(&z, rect, &cfg) {
                    Ok(m) => inner.log("info", m),
                    Err(e) => tracing::warn!(error = %e, "hotkey launch failed"),
                }
            }
            ok()
        }
        Action::Retile => {
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            let mut cfg = inner.draft.clone();
            cfg.active_profile = inner.selected_profile.clone();
            crate::engine::retile(&cfg);
            ok()
        }
        Action::CycleNext => {
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            let mut cfg = inner.draft.clone();
            cfg.active_profile = inner.selected_profile.clone();
            crate::engine::cycle_zone(&cfg, true);
            ok()
        }
        Action::CyclePrev => {
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            let mut cfg = inner.draft.clone();
            cfg.active_profile = inner.selected_profile.clone();
            crate::engine::cycle_zone(&cfg, false);
            ok()
        }
        Action::CycleAxis(dx, dy) => {
            if !inner.layout_enabled {
                return Ok(inactive_result(inner));
            }
            let mut cfg = inner.draft.clone();
            cfg.active_profile = inner.selected_profile.clone();
            crate::engine::cycle_zone_axis(&cfg, dx, dy);
            ok()
        }
        Action::Maximize => {
            if inner.layout_enabled {
                crate::engine::maximize_active();
            }
            ok()
        }
        Action::Restore => {
            if inner.layout_enabled {
                let mut cfg = inner.draft.clone();
                cfg.active_profile = inner.selected_profile.clone();
                crate::engine::restore_active(&cfg);
            }
            ok()
        }
    }
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;
    use crate::native_loop::{HwndSend, NativeQueue, SharedEvent};
    use std::sync::Arc;

    /// Build a context whose side effects are inert (no real window posts).
    fn test_context<'a>(
        queue: &Arc<NativeQueue>,
        shutdown: &ShutdownHandle,
        cache: &'a mut Option<(Instant, Vec<AppEntry>)>,
    ) -> CommandContext<'a> {
        let notifier = NativeNotifier::new(queue.clone(), Arc::new(std::sync::OnceLock::new()));
        CommandContext {
            notifier: Arc::new(notifier),
            shutdown: shutdown.clone(),
            editor_url: Arc::new("http://127.0.0.1:1/#token=x".into()),
            running_cache: cache,
        }
    }

    #[allow(clippy::type_complexity)]
    fn setup() -> (
        Inner,
        Arc<NativeQueue>,
        ShutdownHandle,
        Option<(Instant, Vec<AppEntry>)>,
    ) {
        let inner = Inner::new(default_config());
        let queue = Arc::new(NativeQueue::new());
        let event = SharedEvent::new_test();
        let hwnd = Arc::new(std::sync::OnceLock::<HwndSend>::new());
        let shutdown = ShutdownHandle::new(
            queue.clone(),
            event,
            hwnd,
            tokio::sync::watch::Sender::new(false),
            None,
        );
        (inner, queue, shutdown, None)
    }

    #[test]
    fn d2_dirty_guard_blocks_without_discard_and_leaves_state_unchanged() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        inner.draft.gap = 99; // make dirty
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        let err = handle(CommandPayload::Apply, &mut inner, &mut ctx).unwrap_err();
        assert!(matches!(err, ApiError::DirtyState));
        assert!(inner.is_dirty());
        assert_eq!(inner.draft.gap, 99);
    }

    #[test]
    fn d2_discard_reverts_then_executes() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        inner.draft.gap = 99;
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(
            CommandPayload::ProfileLoad {
                name: "Coding".into(),
                discard_dirty: true,
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        // The dirty gap edit was discarded; loading the profile then makes the
        // draft dirty again by design (active_profile changed).
        assert_eq!(inner.draft.gap, 8);
        assert_eq!(inner.draft.active_profile, "Coding");
    }

    #[test]
    fn d2_all_three_profile_ops_are_guarded() {
        for payload in [
            CommandPayload::ProfileLoad {
                name: "Coding".into(),
                discard_dirty: false,
            },
            CommandPayload::ProfileSelect {
                name: "Coding".into(),
                discard_dirty: false,
            },
            CommandPayload::ProfileApply {
                name: "Coding".into(),
                discard_dirty: false,
            },
            CommandPayload::Apply,
            CommandPayload::LaunchArrange,
        ] {
            let (mut inner, queue, shutdown, mut cache) = setup();
            inner.draft.gap = 42;
            let mut ctx = test_context(&queue, &shutdown, &mut cache);
            let err = handle(payload, &mut inner, &mut ctx).unwrap_err();
            assert!(
                matches!(err, ApiError::DirtyState),
                "payload was not guarded"
            );
        }
    }

    #[test]
    fn d4_disable_guards_apply_and_launch() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        inner.layout_enabled = false;
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        let v = handle(CommandPayload::Apply, &mut inner, &mut ctx).unwrap();
        assert_eq!(v["inactive"], true);
        assert_eq!(v["reason"], "layout_disabled");
        let v = handle(CommandPayload::LaunchArrange, &mut inner, &mut ctx).unwrap();
        assert_eq!(v["inactive"], true);
    }

    #[test]
    fn d6_select_only_does_not_change_applied_or_draft() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(
            CommandPayload::ProfileSelect {
                name: "Trading".into(),
                discard_dirty: false,
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert_eq!(inner.selected_profile, "Trading");
        assert_eq!(inner.applied_profile, "Website");
        assert_eq!(inner.draft.active_profile, "Website");
        assert!(!inner.is_dirty());
    }

    #[test]
    fn d6_load_changes_draft_but_not_applied() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(
            CommandPayload::ProfileLoad {
                name: "Coding".into(),
                discard_dirty: false,
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert_eq!(inner.draft.active_profile, "Coding");
        assert_eq!(inner.applied_profile, "Website");
        assert!(inner.is_dirty());
    }

    #[test]
    fn d3_assignment_preserves_geometry_and_resets_rules() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        {
            let z = &mut inner.draft.active_mut().zones;
            let mut zone = Zone::new(1, "Old", "old.exe", 10.0, 20.0, 30.0, 40.0);
            zone.launch_mode = crate::config::LaunchMode::Attach;
            zone.url = "https://old".into();
            zone.macro_enabled = true;
            zone.macro_text = "^t".into();
            zone.notes = "keep?".into();
            zone.key_delay = 999;
            z.push(zone);
        }
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(
            CommandPayload::ZoneAssign {
                id: 1,
                spec: AssignDto {
                    label: "New".into(),
                    exe: "new.exe".into(),
                    path: String::new(),
                },
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        let z = &inner.draft.active().zones[0];
        assert_eq!(z.id, 1);
        assert_eq!((z.x, z.y, z.w, z.h), (10.0, 20.0, 30.0, 40.0));
        assert_eq!(z.exe, "new.exe");
        assert_eq!(z.launch_mode, crate::config::LaunchMode::Launch);
        assert!(z.url.is_empty());
        assert!(!z.macro_enabled);
        assert!(z.macro_text.is_empty());
        assert!(z.notes.is_empty());
        assert_eq!(z.key_delay, crate::config::Zone::default().key_delay);
    }

    #[test]
    fn d5_preview_never_executes() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        inner
            .draft
            .active_mut()
            .zones
            .push(Zone::new(1, "A", "a.exe", 0.0, 0.0, 50.0, 100.0));
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        let v = handle(CommandPayload::Preview, &mut inner, &mut ctx).unwrap();
        let dto: PreviewDto = serde_json::from_value(v).unwrap();
        assert!(!dto.executed);
        assert_eq!(dto.steps.len(), 1);
    }

    #[test]
    fn zone_crud_split_merge_and_last_zone_guard() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(
            CommandPayload::ZoneCreate {
                zone: ZoneDto::from(&Zone::new(0, "A", "a.exe", 0.0, 0.0, 100.0, 100.0)),
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert_eq!(inner.draft.active().zones.len(), 1);
        // Refuse deleting the last zone.
        assert!(matches!(
            handle(CommandPayload::ZoneDelete { id: 1 }, &mut inner, &mut ctx),
            Err(ApiError::BadRequest(_))
        ));
        handle(
            CommandPayload::ZoneSplit {
                id: 1,
                spec: SplitDto {
                    axis: "x".into(),
                    ratio: 0.5,
                },
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert_eq!(inner.draft.active().zones.len(), 2);
        handle(
            CommandPayload::ZoneMerge {
                spec: MergeDto {
                    source_id: 2,
                    target_id: 1,
                },
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert_eq!(inner.draft.active().zones.len(), 1);
    }

    /// Seed the active profile with two adjacent zones (a vertical divider).
    /// The seeded state becomes the undo baseline for the batch tests.
    fn seed_two_zones(inner: &mut Inner) {
        let zones = &mut inner.draft.active_mut().zones;
        zones.push(Zone::new(1, "A", "a.exe", 0.0, 0.0, 50.0, 100.0));
        zones.push(Zone::new(2, "B", "b.exe", 50.0, 0.0, 50.0, 100.0));
        inner.push_history();
    }

    #[test]
    fn batch_zone_update_creates_exactly_one_history_entry() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        seed_two_zones(&mut inner);
        let before = inner.history.len();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        let v = handle(
            CommandPayload::ZonesBatchUpdate {
                updates: vec![
                    ZoneGeometryUpdateDto {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        w: 40.0,
                        h: 100.0,
                    },
                    ZoneGeometryUpdateDto {
                        id: 2,
                        x: 40.0,
                        y: 0.0,
                        w: 60.0,
                        h: 100.0,
                    },
                ],
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            inner.history.len(),
            before + 1,
            "a 2-zone divider drag must add exactly one undo point"
        );
        let updated: Vec<ZoneDto> = serde_json::from_value(v).unwrap();
        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0].w, 40.0);
        assert_eq!(updated[1].x, 40.0);
    }

    #[test]
    fn batch_zone_update_invalid_id_returns_error_without_partial_mutation() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        seed_two_zones(&mut inner);
        let before = inner.history.len();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        let err = handle(
            CommandPayload::ZonesBatchUpdate {
                updates: vec![
                    ZoneGeometryUpdateDto {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        w: 10.0,
                        h: 10.0,
                    },
                    ZoneGeometryUpdateDto {
                        id: 999,
                        x: 0.0,
                        y: 0.0,
                        w: 10.0,
                        h: 10.0,
                    },
                ],
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)));
        let z1 = inner
            .draft
            .active()
            .zones
            .iter()
            .find(|z| z.id == 1)
            .unwrap();
        assert_eq!(
            (z1.x, z1.y, z1.w, z1.h),
            (0.0, 0.0, 50.0, 100.0),
            "the valid zone must not be mutated when a later id is invalid"
        );
        assert_eq!(inner.history.len(), before, "no undo point on error");
    }

    #[test]
    fn undo_after_batch_restores_all_zones_atomically() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        seed_two_zones(&mut inner);
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(
            CommandPayload::ZonesBatchUpdate {
                updates: vec![
                    ZoneGeometryUpdateDto {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        w: 30.0,
                        h: 100.0,
                    },
                    ZoneGeometryUpdateDto {
                        id: 2,
                        x: 30.0,
                        y: 0.0,
                        w: 70.0,
                        h: 100.0,
                    },
                ],
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        handle(CommandPayload::Undo, &mut inner, &mut ctx).unwrap();
        let zones = &inner.draft.active().zones;
        let z1 = zones.iter().find(|z| z.id == 1).unwrap();
        let z2 = zones.iter().find(|z| z.id == 2).unwrap();
        assert_eq!((z1.x, z1.y, z1.w, z1.h), (0.0, 0.0, 50.0, 100.0));
        assert_eq!((z2.x, z2.y, z2.w, z2.h), (50.0, 0.0, 50.0, 100.0));
    }

    #[test]
    fn profile_crud_updates_all_three_name_references() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(
            CommandPayload::ProfileRename {
                name: "Website".into(),
                new_name: "Home".into(),
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert!(inner.draft.profiles.contains_key("Home"));
        assert_eq!(inner.applied_profile, "Home");
        assert_eq!(inner.selected_profile, "Home");
        assert_eq!(inner.draft.active_profile, "Home");
        handle(
            CommandPayload::ProfileDuplicate {
                name: "Home".into(),
                new_name: "Copy".into(),
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert!(inner.draft.profiles.contains_key("Copy"));
        handle(
            CommandPayload::ProfileDelete {
                name: "Copy".into(),
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert!(!inner.draft.profiles.contains_key("Copy"));
    }

    #[test]
    fn disable_then_enable_toggles_runtime_flag() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        handle(CommandPayload::Disable, &mut inner, &mut ctx).unwrap();
        assert!(!inner.layout_enabled);
        handle(CommandPayload::Enable, &mut inner, &mut ctx).unwrap();
        assert!(inner.layout_enabled);
    }

    #[test]
    fn app_options_update_autostart_and_close_action() {
        let (mut inner, queue, shutdown, mut cache) = setup();
        let mut ctx = test_context(&queue, &shutdown, &mut cache);
        // auto_start stays false (the default), so no registry write occurs.
        handle(
            CommandPayload::AppOptionsSet {
                options: super::super::dto::AppOptionsDto {
                    auto_start: false,
                    close_action: crate::config::CloseAction::Exit,
                    gap: 12,
                },
            },
            &mut inner,
            &mut ctx,
        )
        .unwrap();
        assert!(!inner.draft.auto_start);
        assert_eq!(inner.draft.close_action, crate::config::CloseAction::Exit);
        assert_eq!(inner.draft.gap, 12);
    }
}
