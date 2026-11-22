//! Plain serde DTOs for the HTTP API.
//!
//! Backend types that already derive `Serialize`/`Deserialize` (`AppConfig`,
//! `Settings`, `Profile`, `Zone`, the config enums) are mirrored here so the
//! wire format is owned by the API layer and can evolve independently. Types
//! that do **not** derive Serde (`engine_core::Rect`, `engine_core::KeyEvent`,
//! `system::Action`) are always mapped explicitly — never serialized directly.

use crate::config::{AppConfig, LaunchMode, Profile, Settings, TabBehavior, Zone};
use serde::{Deserialize, Serialize};

/* ------------------------------- geometry -------------------------------- */

/// Wire form of `engine_core::Rect` (screen pixels).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RectDto {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl From<&crate::engine_core::Rect> for RectDto {
    fn from(r: &crate::engine_core::Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}

/* --------------------------------- zone ---------------------------------- */

/// Wire form of a zone. Mirrors every persisted field so the inspector can
/// round-trip a zone without losing rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ZoneDto {
    pub id: i32,
    pub label: String,
    pub exe: String,
    /// Full path used for strict window matching. With a launcher set this is
    /// only an identification override — never a launch target.
    pub path: String,
    /// Optional `.bat`/`.cmd` script executed instead of the executable.
    #[serde(default)]
    pub launcher_path: String,
    /// Whether the script's console is visible (default true).
    #[serde(default = "crate::config::default_launcher_show_console")]
    pub launcher_show_console: bool,
    pub launch_mode: LaunchMode,
    pub title_match: String,
    pub url: String,
    pub tab_behavior: TabBehavior,
    pub macro_enabled: bool,
    pub trigger_title: String,
    pub macro_text: String,
    pub notes: String,
    pub key_delay: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for ZoneDto {
    fn default() -> Self {
        Self::from(&Zone::default())
    }
}

impl From<&Zone> for ZoneDto {
    fn from(z: &Zone) -> Self {
        Self {
            id: z.id,
            label: z.label.clone(),
            exe: z.exe.clone(),
            path: z.path.clone(),
            launcher_path: z.launcher_path.clone(),
            launcher_show_console: z.launcher_show_console,
            launch_mode: z.launch_mode,
            title_match: z.title_match.clone(),
            url: z.url.clone(),
            tab_behavior: z.tab_behavior,
            macro_enabled: z.macro_enabled,
            trigger_title: z.trigger_title.clone(),
            macro_text: z.macro_text.clone(),
            notes: z.notes.clone(),
            key_delay: z.key_delay,
            x: z.x,
            y: z.y,
            w: z.w,
            h: z.h,
        }
    }
}

impl From<ZoneDto> for Zone {
    fn from(d: ZoneDto) -> Self {
        Self {
            id: d.id,
            label: d.label,
            exe: d.exe,
            path: d.path,
            launcher_path: d.launcher_path,
            launcher_show_console: d.launcher_show_console,
            launch_mode: d.launch_mode,
            title_match: d.title_match,
            url: d.url,
            tab_behavior: d.tab_behavior,
            macro_enabled: d.macro_enabled,
            trigger_title: d.trigger_title,
            macro_text: d.macro_text,
            notes: d.notes,
            key_delay: d.key_delay,
            x: d.x,
            y: d.y,
            w: d.w,
            h: d.h,
        }
    }
}

/* -------------------------------- profile -------------------------------- */

/// Compact profile row for the profile list / tray menu.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub name: String,
    pub zone_count: usize,
    pub applied: bool,
    pub selected: bool,
}

impl ProfileSummary {
    pub fn new(profile: &Profile, applied: &str, selected: &str) -> Self {
        Self {
            name: profile.name.clone(),
            zone_count: profile.zones.len(),
            applied: profile.name == applied,
            selected: profile.name == selected,
        }
    }
}

/* --------------------------------- apps ---------------------------------- */

/// A discovered application (runtime enumeration, never persisted).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEntryDto {
    pub id: String,
    pub label: String,
    pub exe: String,
    pub path: String,
}

/* -------------------------------- settings ------------------------------- */

/// Wire form of `config::Settings`. All fields are mirrored so a PUT can
/// replace the whole settings object atomically.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsDto {
    pub snap_points: Vec<f32>,
    pub grid_cols: u32,
    pub grid_rows: u32,
    pub min_zone_pct: f32,
    pub resize_step_pct: f32,
    pub boundary_tolerance_pct: f32,
    pub undo_depth: usize,
    pub display_debounce_ms: u64,
    pub overlay_inactive_alpha: u8,
    pub overlay_active_alpha: u8,
    pub dwm_snap_retries: u32,
    pub min_window_px: i32,
    pub zone_tints: Vec<String>,
    pub auto_launch_on_startup: bool,
    pub resolutions: Vec<crate::config::ResolutionPreset>,
    pub app_list_cap: usize,
    pub running_apps_ttl_secs: u64,
    pub taskbar_h: f32,
    pub stage_lower_h: f32,
}

impl Default for SettingsDto {
    fn default() -> Self {
        Self::from(&Settings::default())
    }
}

impl From<&Settings> for SettingsDto {
    fn from(s: &Settings) -> Self {
        Self {
            snap_points: s.snap_points.clone(),
            grid_cols: s.grid_cols,
            grid_rows: s.grid_rows,
            min_zone_pct: s.min_zone_pct,
            resize_step_pct: s.resize_step_pct,
            boundary_tolerance_pct: s.boundary_tolerance_pct,
            undo_depth: s.undo_depth,
            display_debounce_ms: s.display_debounce_ms,
            overlay_inactive_alpha: s.overlay_inactive_alpha,
            overlay_active_alpha: s.overlay_active_alpha,
            dwm_snap_retries: s.dwm_snap_retries,
            min_window_px: s.min_window_px,
            zone_tints: s.zone_tints.clone(),
            auto_launch_on_startup: s.auto_launch_on_startup,
            resolutions: s.resolutions.clone(),
            app_list_cap: s.app_list_cap,
            running_apps_ttl_secs: s.running_apps_ttl_secs,
            taskbar_h: s.taskbar_h,
            stage_lower_h: s.stage_lower_h,
        }
    }
}

impl From<SettingsDto> for Settings {
    fn from(d: SettingsDto) -> Self {
        Self {
            snap_points: d.snap_points,
            grid_cols: d.grid_cols,
            grid_rows: d.grid_rows,
            min_zone_pct: d.min_zone_pct,
            resize_step_pct: d.resize_step_pct,
            boundary_tolerance_pct: d.boundary_tolerance_pct,
            undo_depth: d.undo_depth,
            display_debounce_ms: d.display_debounce_ms,
            overlay_inactive_alpha: d.overlay_inactive_alpha,
            overlay_active_alpha: d.overlay_active_alpha,
            dwm_snap_retries: d.dwm_snap_retries,
            min_window_px: d.min_window_px,
            zone_tints: d.zone_tints,
            auto_launch_on_startup: d.auto_launch_on_startup,
            resolutions: d.resolutions,
            app_list_cap: d.app_list_cap,
            running_apps_ttl_secs: d.running_apps_ttl_secs,
            taskbar_h: d.taskbar_h,
            stage_lower_h: d.stage_lower_h,
        }
    }
}

/// App-level options stored on `AppConfig` (not `Settings`): Windows autostart
/// registration and the editor close behavior. The web UI needs to edit both,
/// but `SettingsDto` only mirrors `Settings` — hence this small DTO (§5.6 gap).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppOptionsDto {
    pub auto_start: bool,
    pub close_action: crate::config::CloseAction,
    /// Gap between zones (percent of the work area), also on `AppConfig`.
    pub gap: i32,
}

impl Default for AppOptionsDto {
    fn default() -> Self {
        Self {
            auto_start: false,
            close_action: crate::config::CloseAction::Ask,
            gap: 8,
        }
    }
}

/* -------------------------------- hotkeys -------------------------------- */

/// One global hotkey binding (action name -> AHK-style binding string).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyDto {
    pub name: String,
    pub binding: String,
}

/* --------------------------------- logs ---------------------------------- */

/// One console log line. `seq` is a monotonically increasing id used by
/// `GET /api/logs?since=N` for incremental polling.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLineDto {
    pub seq: u64,
    pub level: String,
    pub message: String,
}

/* --------------------------------- state --------------------------------- */

/// Lightweight engine status for `GET /api/status` and the state snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineStatusDto {
    pub work_area: RectDto,
    pub zone_count: usize,
    pub applied: String,
    pub enabled: bool,
}

/// Full authoritative snapshot returned by `GET /api/state`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub draft: AppConfig,
    pub dirty: bool,
    pub applied: String,
    pub selected: String,
    pub enabled: bool,
    pub selected_zone: i32,
    pub profiles: Vec<ProfileSummary>,
    pub zones: Vec<ZoneDto>,
    pub settings: SettingsDto,
    pub hotkeys: Vec<HotkeyDto>,
    pub logs: Vec<LogLineDto>,
    pub status: EngineStatusDto,
    /// Hotkey bindings that failed syntax validation or native registration
    /// (Finding U-03). Mirrored from the native thread through a shared slot.
    pub hotkey_failures: Vec<String>,
    /// Actual Windows autostart registration state, read from the registry.
    /// The Settings toggle is seeded from this; `draft.auto_start` remains the
    /// authoritative target that `set_auto_start` applies on save.
    pub auto_start_registered: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}

/* ------------------------------ command I/O ------------------------------ */

/// One geometry-only update in a `PUT /api/zones/batch` request.
///
/// Divider drags move two zones at once. Sending the whole batch in one request
/// lets the command worker record a single undo point for the whole gesture.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ZoneGeometryUpdateDto {
    pub id: i32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Payload for `POST /api/zones/{id}/split`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SplitDto {
    /// `"x"` or `"y"` (horizontal / vertical divider).
    pub axis: String,
    /// Fraction of the original extent kept by the first zone (0..1).
    pub ratio: f32,
}

/// Payload for `POST /api/zones/merge`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MergeDto {
    pub source_id: i32,
    pub target_id: i32,
}

/// Payload for `POST /api/zones/{id}/assign` (D3 replacement rules).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssignDto {
    pub label: String,
    pub exe: String,
    /// Optional window-identification override (never launched when a launcher is set).
    #[serde(default)]
    pub path: String,
    /// Optional `.bat`/`.cmd` script executed instead of the executable.
    #[serde(default)]
    pub launcher_path: String,
    /// Whether the script's console is visible (default true).
    #[serde(default = "crate::config::default_launcher_show_console")]
    pub launcher_show_console: bool,
}

/// Result of `POST /api/preview` (D5 — inspect steps, never execute).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewDto {
    pub profile: String,
    pub steps: Vec<String>,
    pub executed: bool,
}

/// What was asked for. Kept separate from [`ZoneOutcomeDto`] so an operation's
/// intent is never mistaken for its outcome (Decision 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyIntent {
    Apply,
    LaunchArrange,
}

/// What actually happened to one zone. Every zone in the profile yields
/// exactly one outcome — no silent success (Decision 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneOutcomeStatus {
    Snapped,
    LaunchedAndSnapped,
    Unassigned,
    Unresolved,
    /// A pathless legacy assignment matched multiple distinct installations.
    /// Always a failure — no arbitrary installation is selected.
    AmbiguousPath,
    /// A launcher script produced more than one eligible application window.
    /// No window is guessed; the user can add an EXE override to disambiguate.
    AmbiguousWindow,
    NotRunning,
    NoEligibleWindow,
    LaunchFailed,
    /// The launcher script path, command interpreter or job setup was invalid.
    /// Distinct from `LaunchFailed` (a spawn was attempted and failed).
    LauncherFailed,
    Error,
}

impl ZoneOutcomeStatus {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Snapped | Self::LaunchedAndSnapped)
    }

    pub fn is_skipped(self) -> bool {
        matches!(self, Self::Unassigned)
    }

    pub fn is_failure(self) -> bool {
        !self.is_success() && !self.is_skipped()
    }

    fn label(self) -> &'static str {
        match self {
            Self::Snapped => "snapped",
            Self::LaunchedAndSnapped => "launched & snapped",
            Self::Unassigned => "unassigned",
            Self::Unresolved => "unresolved",
            Self::AmbiguousPath => "ambiguous path",
            Self::AmbiguousWindow => "ambiguous window",
            Self::NotRunning => "not running",
            Self::NoEligibleWindow => "no eligible window",
            Self::LaunchFailed => "launch failed",
            Self::LauncherFailed => "launcher failed",
            Self::Error => "error",
        }
    }
}

/// One zone's outcome. `message` is the human-readable reason for the status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneOutcomeDto {
    pub zone_id: i32,
    pub label: String,
    pub exe: String,
    pub status: ZoneOutcomeStatus,
    pub message: String,
    /// This zone actually spawned a process in this call — not the operation
    /// intent. True even when the launch produced no eligible window.
    pub launched: bool,
}

/// Result of an apply / launch-arrange operation. `intent` records the verb;
/// `outcomes` record what happened; `message` is a summary derived from the
/// outcomes, never a blanket success claim (Decision 6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyResultDto {
    pub profile: String,
    pub intent: ApplyIntent,
    pub outcomes: Vec<ZoneOutcomeDto>,
    pub message: String,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
}

impl ApplyResultDto {
    /// Build a result and derive the counts and summary message from outcomes.
    pub fn new(profile: String, intent: ApplyIntent, outcomes: Vec<ZoneOutcomeDto>) -> Self {
        let succeeded = outcomes.iter().filter(|o| o.status.is_success()).count();
        let skipped = outcomes.iter().filter(|o| o.status.is_skipped()).count();
        let failed = outcomes.iter().filter(|o| o.status.is_failure()).count();
        let message = Self::summarize(&outcomes);
        Self {
            profile,
            intent,
            outcomes,
            message,
            succeeded,
            failed,
            skipped,
        }
    }

    /// e.g. "2 snapped, 1 no eligible window". Empty profiles say so plainly.
    fn summarize(outcomes: &[ZoneOutcomeDto]) -> String {
        if outcomes.is_empty() {
            return "no zones in profile".to_string();
        }
        let mut counts: Vec<(ZoneOutcomeStatus, usize)> = Vec::new();
        for outcome in outcomes {
            match counts
                .iter_mut()
                .find(|(status, _)| *status == outcome.status)
            {
                Some((_, n)) => *n += 1,
                None => counts.push((outcome.status, 1)),
            }
        }
        counts
            .iter()
            .map(|(status, n)| format!("{n} {}", status.label()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/* --------------------------------- errors -------------------------------- */

/// Error envelope: `{ "error": { "code": ..., "message": ... } }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
}

impl ApiErrorBody {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: ApiErrorDetail {
                code: code.into(),
                message: message.into(),
            },
        }
    }
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CloseAction;

    #[test]
    fn zone_dto_round_trips_every_field() {
        let mut z = Zone::new(7, "Editor", "code.exe", 10.0, 20.0, 30.0, 40.0);
        z.path = r"C:\Tools\code.exe".into();
        z.launcher_path = r"C:\Tools\run.cmd".into();
        z.launcher_show_console = false;
        z.launch_mode = LaunchMode::Attach;
        z.title_match = "Visual Studio".into();
        z.url = "https://example.com".into();
        z.tab_behavior = TabBehavior::ReuseBlank;
        z.macro_enabled = true;
        z.trigger_title = "Main".into();
        z.macro_text = "^t".into();
        z.notes = "merged".into();
        z.key_delay = 123;

        let dto = ZoneDto::from(&z);
        let back = Zone::from(dto);
        assert_eq!(back.id, 7);
        assert_eq!(back.label, "Editor");
        assert_eq!(back.exe, "code.exe");
        assert_eq!(back.path, r"C:\Tools\code.exe");
        assert_eq!(back.launcher_path, r"C:\Tools\run.cmd");
        assert!(!back.launcher_show_console);
        assert_eq!(back.launch_mode, LaunchMode::Attach);
        assert_eq!(back.title_match, "Visual Studio");
        assert_eq!(back.url, "https://example.com");
        assert_eq!(back.tab_behavior, TabBehavior::ReuseBlank);
        assert!(back.macro_enabled);
        assert_eq!(back.trigger_title, "Main");
        assert_eq!(back.macro_text, "^t");
        assert_eq!(back.notes, "merged");
        assert_eq!(back.key_delay, 123);
        assert_eq!((back.x, back.y, back.w, back.h), (10.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn assign_dto_launcher_fields_default_for_old_clients() {
        // An old client sends only label/exe/path. The launcher fields must
        // default so the legacy EXE behavior is preserved.
        let json = r#"{"label":"App","exe":"app.exe","path":"C:\\App\\app.exe"}"#;
        let dto: AssignDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.launcher_path, "");
        assert!(dto.launcher_show_console);
        assert_eq!(dto.path, r"C:\App\app.exe");
    }

    #[test]
    fn launcher_statuses_are_failures_and_summarized() {
        for status in [
            ZoneOutcomeStatus::LauncherFailed,
            ZoneOutcomeStatus::AmbiguousWindow,
        ] {
            assert!(status.is_failure());
            assert!(!status.is_success());
        }
        let result = ApplyResultDto::new(
            "P".into(),
            ApplyIntent::LaunchArrange,
            vec![
                outcome(1, ZoneOutcomeStatus::LauncherFailed),
                outcome(2, ZoneOutcomeStatus::AmbiguousWindow),
            ],
        );
        assert_eq!((result.succeeded, result.failed, result.skipped), (0, 2, 0));
        assert_eq!(result.message, "1 launcher failed, 1 ambiguous window");
    }

    #[test]
    fn zone_geometry_update_dto_round_trips() {
        let update = ZoneGeometryUpdateDto {
            id: 7,
            x: 12.5,
            y: 0.0,
            w: 37.5,
            h: 100.0,
        };
        let json = serde_json::to_value(update).unwrap();
        assert_eq!(json["id"], 7);
        assert_eq!(json["x"], 12.5);
        assert_eq!(json["y"], 0.0);
        assert_eq!(json["w"], 37.5);
        assert_eq!(json["h"], 100.0);
        let back: ZoneGeometryUpdateDto = serde_json::from_value(json).unwrap();
        assert_eq!(back, update);
    }

    #[test]
    fn settings_dto_round_trips() {
        let s = Settings {
            grid_cols: 9,
            overlay_active_alpha: 222,
            auto_launch_on_startup: true,
            ..Default::default()
        };
        let dto = SettingsDto::from(&s);
        let back = Settings::from(dto);
        assert_eq!(back.grid_cols, 9);
        assert_eq!(back.overlay_active_alpha, 222);
        assert!(back.auto_launch_on_startup);
        assert_eq!(back.resolutions.len(), 3);
    }

    #[test]
    fn rect_dto_maps_engine_rect() {
        let r = crate::engine_core::Rect {
            x: 1,
            y: 2,
            w: 3,
            h: 4,
        };
        let dto = RectDto::from(&r);
        assert_eq!((dto.x, dto.y, dto.w, dto.h), (1, 2, 3, 4));
    }

    #[test]
    fn profile_summary_marks_applied_and_selected_independently() {
        let p = Profile {
            name: "Coding".into(),
            hotkey: String::new(),
            zones: vec![Zone::default(), Zone::default()],
        };
        let s = ProfileSummary::new(&p, "Website", "Coding");
        assert_eq!(s.zone_count, 2);
        assert!(!s.applied);
        assert!(s.selected);
    }

    #[test]
    fn api_error_body_shape() {
        let body = ApiErrorBody::new("dirty_state", "save or discard changes first");
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["error"]["code"], "dirty_state");
        assert_eq!(json["error"]["message"], "save or discard changes first");
    }

    #[test]
    fn close_action_is_mapped_without_serializing_directly() {
        // CloseAction is a serde enum; confirm the DTO layer can carry it.
        let json = serde_json::to_string(&CloseAction::Minimize).unwrap();
        assert_eq!(json, "\"minimize\"");
    }

    fn outcome(id: i32, status: ZoneOutcomeStatus) -> ZoneOutcomeDto {
        ZoneOutcomeDto {
            zone_id: id,
            label: format!("Zone {id}"),
            exe: "app.exe".into(),
            status,
            message: format!("detail for zone {id}"),
            launched: status == ZoneOutcomeStatus::LaunchedAndSnapped,
        }
    }

    #[test]
    fn apply_result_derives_counts_and_summary_from_outcomes() {
        let result = ApplyResultDto::new(
            "Coding".into(),
            ApplyIntent::LaunchArrange,
            vec![
                outcome(1, ZoneOutcomeStatus::Snapped),
                outcome(2, ZoneOutcomeStatus::LaunchedAndSnapped),
                outcome(3, ZoneOutcomeStatus::NoEligibleWindow),
                outcome(4, ZoneOutcomeStatus::Unassigned),
            ],
        );
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(
            result.message,
            "1 snapped, 1 launched & snapped, 1 no eligible window, 1 unassigned"
        );
        assert_eq!(result.outcomes.len(), 4);
    }

    #[test]
    fn apply_result_serializes_intent_and_status_without_top_level_launched() {
        let result = ApplyResultDto::new(
            "Coding".into(),
            ApplyIntent::Apply,
            vec![outcome(1, ZoneOutcomeStatus::Unassigned)],
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["intent"], "apply");
        assert_eq!(json["outcomes"][0]["status"], "unassigned");
        assert_eq!(json["outcomes"][0]["zone_id"], 1);
        assert_eq!(json["outcomes"][0]["launched"], false);
        assert_eq!(json["succeeded"], 0);
        assert_eq!(json["failed"], 0);
        assert_eq!(json["skipped"], 1);
        assert!(
            json.get("launched").is_none(),
            "the old top-level intent field must stay removed"
        );
        let back: ApplyResultDto = serde_json::from_value(json).unwrap();
        assert_eq!(back, result);
    }

    #[test]
    fn failed_outcome_can_record_that_a_launch_happened() {
        let mut failed = outcome(2, ZoneOutcomeStatus::NoEligibleWindow);
        failed.launched = true;
        let result = ApplyResultDto::new(
            "Coding".into(),
            ApplyIntent::LaunchArrange,
            vec![failed.clone()],
        );
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["outcomes"][0]["status"], "no_eligible_window");
        assert_eq!(json["outcomes"][0]["launched"], true);
        let back: ApplyResultDto = serde_json::from_value(json).unwrap();
        assert_eq!(back.outcomes[0], failed);
        assert_eq!((back.succeeded, back.failed, back.skipped), (0, 1, 0));
    }

    #[test]
    fn intent_and_status_wire_values_are_snake_case() {
        assert_eq!(
            serde_json::to_string(&ApplyIntent::LaunchArrange).unwrap(),
            "\"launch_arrange\""
        );
        assert_eq!(
            serde_json::to_string(&ZoneOutcomeStatus::NoEligibleWindow).unwrap(),
            "\"no_eligible_window\""
        );
        assert_eq!(
            serde_json::to_string(&ZoneOutcomeStatus::LaunchedAndSnapped).unwrap(),
            "\"launched_and_snapped\""
        );
    }

    #[test]
    fn empty_profile_summary_is_explicit() {
        let result = ApplyResultDto::new("Empty".into(), ApplyIntent::Apply, vec![]);
        assert_eq!(result.message, "no zones in profile");
        assert_eq!((result.succeeded, result.failed, result.skipped), (0, 0, 0));
    }

    #[test]
    fn ambiguous_path_is_a_failure_never_a_success() {
        let status = ZoneOutcomeStatus::AmbiguousPath;
        assert!(!status.is_success());
        assert!(!status.is_skipped());
        assert!(status.is_failure());
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            "\"ambiguous_path\""
        );
    }

    #[test]
    fn ambiguous_path_counts_as_failure_and_is_summarized() {
        let result = ApplyResultDto::new(
            "Coding".into(),
            ApplyIntent::LaunchArrange,
            vec![outcome(1, ZoneOutcomeStatus::AmbiguousPath)],
        );
        assert_eq!((result.succeeded, result.failed, result.skipped), (0, 1, 0));
        assert_eq!(result.message, "1 ambiguous path");
    }
}
