use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

/// Current on-disk schema version. Bump when the structure changes incompatibly.
pub const SCHEMA_VERSION: u32 = 1;

fn zero() -> u32 {
    0
}

/* ---- Defaults for the domain tunables (§2.2) ---- */

fn default_snap_points() -> Vec<f32> {
    vec![25.0, 50.0, 75.0]
}

fn default_resolutions() -> Vec<ResolutionPreset> {
    vec![
        ResolutionPreset {
            label: "3840 x 2160 - 4K".into(),
            width: 3840,
            work_height: 2112,
        },
        ResolutionPreset {
            label: "2560 x 1440 - QHD".into(),
            width: 2560,
            work_height: 1392,
        },
        ResolutionPreset {
            label: "1920 x 1080 - FHD".into(),
            width: 1920,
            work_height: 1032,
        },
    ]
}

fn default_app_list_cap() -> usize {
    200
}

fn default_running_apps_ttl_secs() -> u64 {
    2
}

fn default_taskbar_h() -> f32 {
    16.0
}

fn default_stage_lower_h() -> f32 {
    200.0
}

/// Launcher consoles are visible by default so the user can see the BAT run.
/// Old configs without the field get `true` via serde's default attribute.
pub(crate) fn default_launcher_show_console() -> bool {
    true
}

/* ============================ Enums ============================ */

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    #[default]
    Launch,
    Attach,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabBehavior {
    #[default]
    FocusOrNew,
    ReuseBlank,
    Ignore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloseAction {
    #[default]
    Ask,
    Minimize,
    Exit,
}

/* ============================ Zone ============================ */

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Zone {
    pub id: i32,
    pub label: String,
    pub exe: String,
    /// Full path to the executable (for matchType: path parity with the AHK script).
    /// When `launcher_path` is set this is ONLY a window-identification override —
    /// it is never launched instead of the script.
    pub path: String,
    /// Optional `.bat`/`.cmd` script executed via `cmd.exe /c` instead of the
    /// executable. Empty means the legacy EXE launch behavior is unchanged.
    #[serde(default)]
    pub launcher_path: String,
    /// Whether the console stays visible while the launcher script runs.
    /// Defaults to `true`; has no effect on EXE launches.
    #[serde(default = "default_launcher_show_console")]
    pub launcher_show_console: bool,
    pub launch_mode: LaunchMode,
    pub title_match: String,
    pub url: String,
    pub tab_behavior: TabBehavior,
    pub macro_enabled: bool,
    pub trigger_title: String,
    /// Keystrokes sent when the zone is triggered. Never store human-readable
    /// annotations here — they would be typed as keys. Use [`Zone::notes`].
    pub macro_text: String,
    /// Free-form, non-executable notes (e.g. merged-app bookkeeping). Unlike
    /// `macro_text`, this is never interpreted as keystrokes.
    #[serde(default)]
    pub notes: String,
    pub key_delay: u32,
    /// Normalized rectangle in percent (0..100) of the work area.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Default for Zone {
    fn default() -> Self {
        Self {
            id: 0,
            label: String::new(),
            exe: String::new(),
            path: String::new(),
            launcher_path: String::new(),
            launcher_show_console: default_launcher_show_console(),
            launch_mode: LaunchMode::default(),
            title_match: String::new(),
            url: String::new(),
            tab_behavior: TabBehavior::default(),
            macro_enabled: false,
            trigger_title: String::new(),
            macro_text: String::new(),
            notes: String::new(),
            key_delay: 80,
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        }
    }
}
impl Zone {
    #[allow(dead_code)] // used by unit tests
    pub fn new(id: i32, label: &str, exe: &str, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id,
            label: label.into(),
            exe: exe.into(),
            x,
            y,
            w,
            h,
            ..Self::default()
        }
    }
}

/* ============================ Profile ============================ */

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub hotkey: String,
    pub zones: Vec<Zone>,
}

/* ============================ Resolution presets ============================ */

/// A resolution preset retained for config compatibility. There is currently
/// no resolution selector in the UI, so these values are not consumed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolutionPreset {
    /// Label for the preset (retained for config compatibility).
    pub label: String,
    /// Screen width in pixels (retained for config compatibility).
    pub width: u32,
    /// Work-area height in pixels (retained for config compatibility).
    pub work_height: u32,
}

/* ============================ Settings (all tunables) ============================ */

/// Every tunable from the plan (§2.2). All values are defaults stored in config,
/// never literals in code. Kept in a separate struct so the top-level fields that
/// the engine and API already use (gap, close_action, auto_start) stay compatible.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub snap_points: Vec<f32>,
    pub grid_cols: u32,
    pub grid_rows: u32,
    pub min_zone_pct: f32,
    /// Retained for config compatibility; currently unused (no UI control or
    /// consumer). Kept so older config files still load without data loss.
    pub resize_step_pct: f32,
    pub boundary_tolerance_pct: f32,
    pub undo_depth: usize,
    /// Retained for config compatibility; currently unused (no display-change
    /// detection in this package). Kept so older config files still load.
    pub display_debounce_ms: u64,
    pub overlay_inactive_alpha: u8,
    pub overlay_active_alpha: u8,
    pub dwm_snap_retries: u32,
    pub min_window_px: i32,
    pub zone_tints: Vec<String>,
    /// Retained for config compatibility; currently unused (no consumer). The
    /// Settings view no longer presents a toggle for it.
    #[serde(default)]
    pub auto_launch_on_startup: bool,
    /// Retained for config compatibility; currently unused (no resolution
    /// selector in the UI). Kept so older config files still load unchanged.
    #[serde(default = "default_resolutions")]
    pub resolutions: Vec<ResolutionPreset>,
    /// Maximum number of installed apps shown in the library / picker.
    #[serde(default = "default_app_list_cap")]
    pub app_list_cap: usize,
    /// Seconds a running-app enumeration stays fresh before it is re-run.
    #[serde(default = "default_running_apps_ttl_secs")]
    pub running_apps_ttl_secs: u64,
    /// Retained for config compatibility; currently unused (no consumer).
    #[serde(default = "default_taskbar_h")]
    pub taskbar_h: f32,
    /// Retained for config compatibility; currently unused (no consumer).
    #[serde(default = "default_stage_lower_h")]
    pub stage_lower_h: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            snap_points: default_snap_points(),
            grid_cols: 12,
            grid_rows: 8,
            min_zone_pct: 15.0,
            resize_step_pct: 1.0,
            boundary_tolerance_pct: 1.5,
            undo_depth: 60,
            display_debounce_ms: 200,
            overlay_inactive_alpha: 200,
            overlay_active_alpha: 255,
            dwm_snap_retries: 2,
            min_window_px: 150,
            auto_launch_on_startup: false,
            resolutions: default_resolutions(),
            app_list_cap: default_app_list_cap(),
            running_apps_ttl_secs: default_running_apps_ttl_secs(),
            taskbar_h: default_taskbar_h(),
            stage_lower_h: default_stage_lower_h(),
            zone_tints: vec![
                "#F5B544".into(),
                "#6EA8FE".into(),
                "#5BD6A0".into(),
                "#C792EA".into(),
                "#FF8A65".into(),
                "#4DD0E1".into(),
            ],
        }
    }
}

/// Clamp a float to `lo..=hi`, substituting `default` for NaN/infinity so a
/// corrupt value can never poison the editor. `f32::clamp` panics on a NaN
/// input, so the finite check must come first.
fn clamp_f32(v: f32, lo: f32, hi: f32, default: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        default
    }
}

impl Settings {
    /// Clamp every numeric tunable to the range the Settings UI advertises.
    ///
    /// The loopback API is unauthenticated, so HTML `min`/`max` attributes are
    /// not a security boundary: a crafted PUT can carry any value. This is the
    /// authoritative repair point, applied on config load and on every settings
    /// write. Values already in range are left untouched (no data loss).
    pub fn validate(&mut self) {
        self.grid_cols = self.grid_cols.clamp(1, 24);
        self.grid_rows = self.grid_rows.clamp(1, 24);
        self.min_zone_pct = clamp_f32(self.min_zone_pct, 1.0, 50.0, 15.0);
        self.boundary_tolerance_pct = clamp_f32(self.boundary_tolerance_pct, 0.0, 10.0, 1.5);
        self.undo_depth = self.undo_depth.clamp(1, 200);
        // `u8` is inherently 0..255 — the UI's alpha range needs no clamping.
        self.display_debounce_ms = self.display_debounce_ms.clamp(0, 5000);
        self.dwm_snap_retries = self.dwm_snap_retries.clamp(0, 20);
        self.min_window_px = self.min_window_px.clamp(0, 500);
        self.app_list_cap = self.app_list_cap.clamp(10, 2000);
        self.running_apps_ttl_secs = self.running_apps_ttl_secs.clamp(0, 600);
        self.resize_step_pct = clamp_f32(self.resize_step_pct, 0.5, 25.0, 1.0);
        self.taskbar_h = clamp_f32(self.taskbar_h, 0.0, 200.0, 16.0);
        self.stage_lower_h = clamp_f32(self.stage_lower_h, 0.0, 200.0, 200.0);

        // Keep only finite snap points inside 0..100. An empty result would
        // leave the editor with no guides at all, so restore the sane default.
        self.snap_points
            .retain(|p| p.is_finite() && (0.0..=100.0).contains(p));
        if self.snap_points.is_empty() {
            self.snap_points = default_snap_points();
        }
        // `zone_tints` is a free-form list of CSS colors; keep it as-is.
    }
}

/* ---- Settings snapshot for free functions without a UI instance ------ */

/// Snapshot of the settings for free helpers (app enumeration). Wrapped in an
/// RwLock so it can be refreshed on load/save;
/// the outer OnceLock only guards one-time allocation of the lock itself.
static LOADED_SETTINGS: OnceLock<RwLock<Settings>> = OnceLock::new();

fn settings_cell() -> &'static RwLock<Settings> {
    LOADED_SETTINGS.get_or_init(|| RwLock::new(Settings::default()))
}

/// Refresh the snapshot from a config's settings. Called after load and save
/// so runtime setting edits propagate (fixes bug B7).
pub fn refresh_loaded_settings(settings: &Settings) {
    match settings_cell().write() {
        Ok(mut guard) => *guard = settings.clone(),
        Err(e) => tracing::warn!(error = %e, "settings snapshot lock poisoned; refresh skipped"),
    }
}

/// Current settings snapshot (owned clone).
pub fn loaded_settings() -> Settings {
    settings_cell()
        .read()
        .map(|g| g.clone())
        .unwrap_or_else(|_| Settings::default())
}

/* ============================ AppConfig ============================ */

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Missing in v0 configs — defaults to 0 so `migrate()` can detect and upgrade them.
    #[serde(default = "zero")]
    pub schema_version: u32,
    pub profiles: BTreeMap<String, Profile>,
    pub active_profile: String,
    pub gap: i32,
    pub close_action: CloseAction,
    pub auto_start: bool,
    /// Global hotkey bindings (action name -> binding string). Defaults mirror the AHK script (§2c).
    pub hotkeys: BTreeMap<String, String>,
    pub settings: Settings,
    /// Panic-free fallback for `active()` when the invariant is temporarily broken.
    /// Never serialized; `validate()` repairs the real map on load.
    #[serde(skip)]
    fallback: Profile,
}
impl Default for AppConfig {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        for name in ["Website", "Coding", "Trading"] {
            profiles.insert(
                name.into(),
                Profile {
                    name: name.into(),
                    hotkey: String::new(),
                    zones: Vec::new(),
                },
            );
        }
        Self {
            schema_version: SCHEMA_VERSION,
            profiles,
            active_profile: "Website".into(),
            gap: 8,
            close_action: CloseAction::Ask,
            auto_start: false,
            hotkeys: default_hotkeys(),
            settings: Settings::default(),
            fallback: Profile::default(),
        }
    }
}

/// New installs start with empty starter profiles — no hardcoded apps, URLs or titles.
pub fn default_config() -> AppConfig {
    AppConfig::default()
}

/// Default global hotkey bindings, mirroring the original AHK script (§2c).
/// These are DEFAULTS stored in config — editable in the Hotkeys view, never hardcoded in system.rs.
pub fn default_hotkeys() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for i in 1..=9 {
        m.insert(format!("profile_{}", i), format!("Ctrl+Shift+{}", i));
    }
    m.insert("clear_layout".into(), "Ctrl+Shift+0".into());
    m.insert("move_left".into(), "Ctrl+Alt+Left".into());
    m.insert("move_right".into(), "Ctrl+Alt+Right".into());
    m.insert("move_up".into(), "Ctrl+Alt+Up".into());
    m.insert("move_down".into(), "Ctrl+Alt+Down".into());
    m.insert("maximize".into(), "Win+Up".into());
    m.insert("restore".into(), "Win+Down".into());
    m.insert("reload".into(), "Ctrl+Alt+R".into());
    m
}

/* ============================ Paths / IO ============================ */

pub fn config_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("screenlayout.exe"));
    p.pop();
    p.push("ScreenLayout.json");
    p
}

/// Migrate older schemas forward. v0 had no `schema_version`/`settings`; serde's
/// `#[serde(default)]` already filled those, so we only need to bump the version
/// and preserve the user's existing profiles/zones (never delete user data).
fn migrate(mut cfg: AppConfig) -> AppConfig {
    if cfg.schema_version == 0 {
        cfg.schema_version = SCHEMA_VERSION;
    }
    cfg.validate();
    cfg
}

pub fn load_config() -> AppConfig {
    let cfg = load_config_inner();
    // Publish the loaded settings for free helpers (app enumeration) that have
    // no UI instance to read from.
    refresh_loaded_settings(&cfg.settings);
    cfg
}

fn load_config_inner() -> AppConfig {
    load_config_from(&config_path())
}

/// Load a config from an explicit path. This is the body of `load_config_inner`
/// factored out so the unreadable-config recovery paths can be unit-tested with
/// a temp path instead of the real config location. Production behaviour is
/// unchanged: `load_config_inner` passes exactly `config_path()`.
fn load_config_from(path: &Path) -> AppConfig {
    if !path.exists() {
        let cfg = default_config();
        // Write defaults to the GIVEN path, not the global config_path(): this
        // helper is path-isolated so it can be exercised with a temp path.
        if let Err(e) = save_config_to(path, &cfg) {
            tracing::warn!(error = %e, "Failed to write default config");
        }
        return cfg;
    }
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            // Preserve the unreadable original for recovery instead of silently
            // falling back to defaults. Use a distinct suffix from the
            // JSON-parse branch (`.json.bad`) so the two recovery paths cannot
            // collide. If preservation fails, leave the original untouched —
            // never delete user data on a best-effort recovery path.
            let preserved = path.with_extension("json.bad.read");
            match fs::rename(path, &preserved) {
                Ok(()) => tracing::warn!(
                    error = %e,
                    preserved = %preserved.display(),
                    "Config file was unreadable; preserved for recovery and using defaults"
                ),
                Err(preserve_err) => tracing::warn!(
                    error = %e,
                    preserve_error = %preserve_err,
                    "Config file was unreadable and could not be preserved; using defaults"
                ),
            }
            return default_config();
        }
    };
    let cfg: AppConfig = match serde_json::from_str(&text) {
        Ok(c) => c,
        Err(_) => {
            // Preserve the unreadable file for recovery instead of silently wiping it.
            let bad = path.with_extension("json.bad");
            if let Err(e) = fs::rename(path, &bad) {
                tracing::warn!(error = %e, "Failed to preserve unreadable config for recovery");
            }
            return default_config();
        }
    };
    migrate(cfg)
}

/// Write `cfg` to an explicit `path` via temp file + atomic rename. On Windows
/// `std::fs::rename` uses MoveFileExW(MOVEFILE_REPLACE_EXISTING), which replaces
/// an existing destination atomically — no remove-first crash window (bug B11).
///
/// This is the single atomic-write implementation: `save_config` delegates here
/// with `config_path()`, and `load_config_from` uses it with its own path so the
/// loader stays fully path-isolated. It intentionally has no global side effects
/// (the settings snapshot is refreshed by the callers that own that state).
fn save_config_to(path: &Path, cfg: &AppConfig) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

pub fn save_config(cfg: &AppConfig) -> std::io::Result<()> {
    save_config_to(&config_path(), cfg)?;
    // Keep the settings snapshot in sync with what was just saved (bug B7).
    refresh_loaded_settings(&cfg.settings);
    Ok(())
}

/* ============================ AppConfig helpers ============================ */

impl AppConfig {
    /// Repair invariants after load: at least one profile, valid active key.
    pub fn validate(&mut self) {
        if self.profiles.is_empty() {
            self.profiles.insert(
                "Website".into(),
                Profile {
                    name: "Website".into(),
                    hotkey: String::new(),
                    zones: Vec::new(),
                },
            );
        }
        if !self.profiles.contains_key(&self.active_profile) {
            self.active_profile = self
                .profiles
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "Website".into());
        }
        // Repair corrupt/out-of-range tunables on load (via `migrate`) as well
        // as on any other validation pass. Idempotent for already-valid values.
        self.gap = self.gap.clamp(0, 20);
        self.settings.validate();
    }

    /// Panic-free: falls back to an in-memory empty profile if the invariant is broken.
    pub fn active(&self) -> &Profile {
        self.profiles
            .get(&self.active_profile)
            .unwrap_or(&self.fallback)
    }

    /// Panic-free: self-heals by inserting a profile if the active key is missing.
    pub fn active_mut(&mut self) -> &mut Profile {
        let name = self.active_profile.clone();
        self.profiles.entry(name).or_insert_with(|| Profile {
            name: self.active_profile.clone(),
            hotkey: String::new(),
            zones: Vec::new(),
        })
    }
}

/* ============================ Tests ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_empty_starter_profiles() {
        let cfg = default_config();
        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
        assert_eq!(cfg.profiles.len(), 3);
        assert!(cfg.profiles.values().all(|p| p.zones.is_empty()));
        assert!(cfg.profiles.values().all(|p| p.hotkey.is_empty()));
        assert_eq!(cfg.active_profile, "Website");
    }

    #[test]
    fn round_trip_preserves_data() {
        let mut cfg = default_config();
        cfg.gap = 12;
        cfg.active_mut()
            .zones
            .push(Zone::new(1, "App", "app.exe", 0.0, 0.0, 50.0, 100.0));
        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.gap, 12);
        assert_eq!(back.active().zones.len(), 1);
        assert_eq!(back.active().zones[0].exe, "app.exe");
    }

    #[test]
    fn missing_fields_use_defaults() {
        // A minimal v0-style config with only a few fields.
        let json = r#"{"profiles":{},"active_profile":"","gap":5}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.schema_version, 0);
        assert_eq!(cfg.gap, 5);
        assert_eq!(cfg.settings.grid_cols, 12);
        assert_eq!(cfg.settings.snap_points, vec![25.0, 50.0, 75.0]);
    }

    #[test]
    fn v0_migrates_and_repairs() {
        let json = r#"{"profiles":{},"active_profile":"","gap":5}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        let migrated = migrate(cfg);
        assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        assert!(!migrated.profiles.is_empty());
        assert!(migrated.profiles.contains_key(&migrated.active_profile));
    }

    #[test]
    fn active_helpers_are_panic_free() {
        let mut cfg: AppConfig =
            serde_json::from_str(r#"{"profiles":{},"active_profile":"missing"}"#).unwrap();
        assert!(cfg.active().zones.is_empty()); // fallback, no panic
        cfg.active_mut().zones.push(Zone::default()); // self-heals, no panic
        assert_eq!(cfg.active().zones.len(), 1);
    }

    #[test]
    fn auto_launch_on_startup_defaults_to_false() {
        // New installs are safe by default: no surprise app launches.
        assert!(!default_config().settings.auto_launch_on_startup);
        // Older configs that lack the field also default to false.
        let old: AppConfig =
            serde_json::from_str(r#"{"profiles":{},"active_profile":""}"#).unwrap();
        assert!(!old.settings.auto_launch_on_startup);
        // An explicitly enabled value survives a round trip.
        let mut cfg = default_config();
        cfg.settings.auto_launch_on_startup = true;
        let json = serde_json::to_string(&cfg).unwrap();
        let back: AppConfig = serde_json::from_str(&json).unwrap();
        assert!(back.settings.auto_launch_on_startup);
    }

    #[test]
    fn zone_notes_default_to_empty_and_round_trip() {
        // Old configs have no `notes` field — it must default to empty.
        let old = r#"{"id":1,"label":"A","exe":"a.exe","macro_text":"{LEFT}"}"#;
        let z: Zone = serde_json::from_str(old).unwrap();
        assert_eq!(z.notes, "");
        assert_eq!(z.macro_text, "{LEFT}");

        // New field survives a serialize/deserialize round trip.
        let mut z = Zone::new(2, "B", "b.exe", 0.0, 0.0, 50.0, 100.0);
        z.notes = "merged app: C [c.exe]".into();
        let json = serde_json::to_string(&z).unwrap();
        let back: Zone = serde_json::from_str(&json).unwrap();
        assert_eq!(back.notes, "merged app: C [c.exe]");
        assert_eq!(back.macro_text, "");
    }

    #[test]
    fn launcher_fields_default_and_round_trip() {
        // Old zones have no launcher fields — they must load unchanged with an
        // empty path and a visible console (no schema version bump).
        let old = r#"{"id":1,"label":"A","exe":"a.exe","path":"C:\\A\\a.exe"}"#;
        let z: Zone = serde_json::from_str(old).unwrap();
        assert_eq!(z.launcher_path, "");
        assert!(z.launcher_show_console);
        assert_eq!(z.exe, "a.exe");
        assert_eq!(z.path, r"C:\A\a.exe");

        // New fields survive a serialize/deserialize round trip.
        let mut z = Zone::new(2, "B", "b.exe", 0.0, 0.0, 50.0, 100.0);
        z.launcher_path = r"C:\Scripts\run.bat".into();
        z.launcher_show_console = false;
        let json = serde_json::to_string(&z).unwrap();
        let back: Zone = serde_json::from_str(&json).unwrap();
        assert_eq!(back.launcher_path, r"C:\Scripts\run.bat");
        assert!(!back.launcher_show_console);
    }

    #[test]
    fn old_config_without_launcher_fields_loads_unchanged() {
        // A full v1 config predating launchers must deserialize with the
        // documented defaults and keep every existing value.
        let json = r#"{"schema_version":1,"profiles":{"P":{"name":"P","zones":[{"id":3,"label":"App","exe":"app.exe","path":"C:\\App\\app.exe"}]}},"active_profile":"P"}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        let z = &cfg.active().zones[0];
        assert_eq!(z.launcher_path, "");
        assert!(z.launcher_show_console);
        assert_eq!(z.exe, "app.exe");
        assert_eq!(z.path, r"C:\App\app.exe");
    }

    #[test]
    fn new_settings_defaults_match_legacy_constants() {
        let s = Settings::default();
        assert_eq!(s.resolutions.len(), 3);
        assert_eq!(s.resolutions[0].label, "3840 x 2160 - 4K");
        assert_eq!(s.resolutions[0].width, 3840);
        assert_eq!(s.resolutions[0].work_height, 2112);
        assert_eq!(s.resolutions[1].label, "2560 x 1440 - QHD");
        assert_eq!(s.resolutions[1].width, 2560);
        assert_eq!(s.resolutions[1].work_height, 1392);
        assert_eq!(s.resolutions[2].label, "1920 x 1080 - FHD");
        assert_eq!(s.resolutions[2].width, 1920);
        assert_eq!(s.resolutions[2].work_height, 1032);
        assert_eq!(s.app_list_cap, 200);
        assert_eq!(s.running_apps_ttl_secs, 2);
        assert_eq!(s.taskbar_h, 16.0);
        assert_eq!(s.stage_lower_h, 200.0);
    }

    #[test]
    fn old_config_gets_new_setting_defaults() {
        // `settings` is present but predates the new fields.
        let json = r#"{"profiles":{},"active_profile":"","settings":{"grid_cols":12}}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.settings.grid_cols, 12);
        assert_eq!(cfg.settings.app_list_cap, 200);
        assert_eq!(cfg.settings.running_apps_ttl_secs, 2);
        assert_eq!(cfg.settings.taskbar_h, 16.0);
        assert_eq!(cfg.settings.stage_lower_h, 200.0);
        assert_eq!(cfg.settings.resolutions.len(), 3);
        assert_eq!(cfg.settings.resolutions[2].width, 1920);
        assert_eq!(cfg.settings.resolutions[2].work_height, 1032);
    }

    #[test]
    fn refresh_loaded_settings_propagates_runtime_edits() {
        // Bug B7: the snapshot must reflect a changed setting after refresh.
        let s = Settings {
            app_list_cap: 999,
            grid_cols: 7,
            ..Default::default()
        };
        refresh_loaded_settings(&s);
        let snap = loaded_settings();
        assert_eq!(snap.app_list_cap, 999);
        assert_eq!(snap.grid_cols, 7);
        // Restore defaults so this test does not leak state into others.
        refresh_loaded_settings(&Settings::default());
    }

    #[test]
    fn save_config_to_replaces_existing_file_atomically() {
        // Bug B11: an existing destination must be replaced with new content
        // and no .tmp file left behind (no remove-first crash window).
        let dir = std::env::temp_dir().join(format!("zoneflow_atomic_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("ScreenLayout.json");
        let mut cfg = default_config();
        cfg.gap = 3;
        save_config_to(&path, &cfg).unwrap();
        // Second write over an existing destination must succeed (rename replaces).
        cfg.gap = 42;
        save_config_to(&path, &cfg).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let back: AppConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back.gap, 42);
        assert!(!path.with_extension("json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_config_from_creates_defaults_at_given_path_when_missing() {
        // The extracted loader must be fully path-isolated: for a nonexistent
        // path it writes the default config to THAT path, not to the global
        // config_path(). If it still called save_config(), the temp file below
        // would never appear and this test would fail. We deliberately do not
        // inspect the real config path; the temp file's existence is the signal.
        let dir = std::env::temp_dir().join(format!("zoneflow_missing_cfg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ScreenLayout.json");
        assert!(!path.exists(), "precondition: temp config must not exist");

        let cfg = load_config_from(&path);

        // Defaults are returned to the caller.
        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
        assert_eq!(cfg.gap, 8);
        assert_eq!(cfg.active_profile, "Website");

        // The defaults were written to the GIVEN path as valid JSON with the
        // same values, and the atomic write left no .tmp file behind.
        assert!(path.exists(), "defaults must be written to the given path");
        let text = fs::read_to_string(&path).unwrap();
        let written: AppConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(written.schema_version, SCHEMA_VERSION);
        assert_eq!(written.gap, 8);
        assert_eq!(written.active_profile, "Website");
        assert!(!path.with_extension("json.tmp").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_config_is_preserved_as_bad_and_defaults_returned() {
        // R-05: an unreadable config must be renamed to `.json.bad` for
        // recovery and the loader must fall back to defaults. Uses a temp path
        // so the real config location is never touched.
        let dir = std::env::temp_dir().join(format!("zoneflow_bad_cfg_a_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ScreenLayout.json");
        let bad = path.with_extension("json.bad");
        let invalid = "{ this is not valid json";
        fs::write(&path, invalid).unwrap();

        let cfg = load_config_from(&path);

        // Defaults are returned, not a partial/garbage config.
        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
        assert_eq!(cfg.gap, 8);
        assert_eq!(cfg.active_profile, "Website");
        // The unreadable original is gone and preserved verbatim as `.bad`.
        assert!(!path.exists(), "unreadable config must be moved away");
        assert!(bad.exists(), "unreadable config must be preserved as .bad");
        assert_eq!(fs::read_to_string(&bad).unwrap(), invalid);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_config_with_existing_bad_file_still_returns_defaults() {
        // R-05: when `.json.bad` already exists, the rename may fail (or replace
        // it, depending on platform). The loader must still return defaults
        // without panicking and without losing the unreadable config content.
        let dir = std::env::temp_dir().join(format!("zoneflow_bad_cfg_b_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ScreenLayout.json");
        let bad = path.with_extension("json.bad");
        let invalid = "{ still not json";
        fs::write(&path, invalid).unwrap();
        fs::write(&bad, "previous bad content").unwrap();

        let cfg = load_config_from(&path);

        // No panic, defaults returned.
        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
        assert_eq!(cfg.gap, 8);
        assert_eq!(cfg.active_profile, "Website");
        // The unreadable config must survive somewhere: at its original path
        // (rename failed) or in the `.bad` file (rename replaced it).
        let in_original = fs::read_to_string(&path).unwrap_or_default();
        let in_bad = fs::read_to_string(&bad).unwrap_or_default();
        assert!(
            in_original == invalid || in_bad == invalid,
            "unreadable config content must not be lost"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_config_read_error_is_preserved_and_defaults_returned() {
        // B2: a config that exists but cannot be read must be preserved under a
        // distinct `.json.bad.read` suffix (not silently dropped, and not
        // colliding with the JSON-parse branch's `.json.bad`). A directory at
        // the config path is a portable way to force `read_to_string` to fail.
        let dir =
            std::env::temp_dir().join(format!("zoneflow_unreadable_cfg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ScreenLayout.json");
        fs::create_dir(&path).unwrap();
        let preserved = path.with_extension("json.bad.read");

        let cfg = load_config_from(&path);

        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
        assert_eq!(cfg.gap, 8);
        assert_eq!(cfg.active_profile, "Website");
        assert!(!path.exists(), "unreadable config must be moved away");
        assert!(preserved.exists(), "unreadable config must be preserved");
        assert!(
            !path.with_extension("json.bad").exists(),
            "read-error recovery must not collide with the parse-error suffix"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_validate_clamps_every_out_of_range_field() {
        let mut s = Settings {
            grid_cols: 999,
            grid_rows: 0,
            min_zone_pct: -5.0,
            boundary_tolerance_pct: 99.0,
            undo_depth: 9999,
            display_debounce_ms: 99_999,
            dwm_snap_retries: 999,
            min_window_px: -50,
            app_list_cap: 99_999,
            running_apps_ttl_secs: 99_999,
            resize_step_pct: 999.0,
            taskbar_h: -1.0,
            stage_lower_h: 999.0,
            ..Default::default()
        };
        s.validate();
        assert_eq!(s.grid_cols, 24);
        assert_eq!(s.grid_rows, 1);
        assert_eq!(s.min_zone_pct, 1.0);
        assert_eq!(s.boundary_tolerance_pct, 10.0);
        assert_eq!(s.undo_depth, 200);
        assert_eq!(s.display_debounce_ms, 5000);
        assert_eq!(s.dwm_snap_retries, 20);
        assert_eq!(s.min_window_px, 0);
        assert_eq!(s.app_list_cap, 2000);
        assert_eq!(s.running_apps_ttl_secs, 600);
        assert_eq!(s.resize_step_pct, 25.0);
        assert_eq!(s.taskbar_h, 0.0);
        assert_eq!(s.stage_lower_h, 200.0);
    }

    #[test]
    fn settings_validate_filters_snap_points_and_repairs_empty() {
        let mut s = Settings {
            snap_points: vec![f32::NAN, f32::INFINITY, -1.0, 101.0, 25.0, 50.0],
            ..Default::default()
        };
        s.validate();
        assert_eq!(s.snap_points, vec![25.0, 50.0]);

        s.snap_points.clear();
        s.validate();
        assert_eq!(s.snap_points, vec![25.0, 50.0, 75.0]);
    }

    #[test]
    fn settings_validate_leaves_valid_values_unchanged() {
        let mut s = Settings {
            grid_cols: 9,
            grid_rows: 6,
            min_zone_pct: 12.5,
            boundary_tolerance_pct: 2.0,
            undo_depth: 42,
            display_debounce_ms: 150,
            dwm_snap_retries: 3,
            min_window_px: 120,
            app_list_cap: 500,
            running_apps_ttl_secs: 5,
            resize_step_pct: 2.5,
            taskbar_h: 32.0,
            stage_lower_h: 180.0,
            snap_points: vec![10.0, 33.3, 90.0],
            zone_tints: vec!["#123456".into()],
            ..Default::default()
        };
        let before = serde_json::to_value(&s).unwrap();
        s.validate();
        let after = serde_json::to_value(&s).unwrap();
        assert_eq!(before, after, "valid settings must not be modified");
    }

    #[test]
    fn migrate_repairs_corrupt_settings_on_load() {
        // A crafted/legacy config with out-of-range values must be repaired by
        // the same path `load_config` uses, without dropping any other data.
        let json = r#"{"profiles":{},"active_profile":"","gap":5,"settings":{"grid_cols":999,"min_zone_pct":-5,"snap_points":[],"app_list_cap":1}}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        let migrated = migrate(cfg);
        assert_eq!(migrated.gap, 5, "unrelated data must survive");
        assert_eq!(migrated.settings.grid_cols, 24);
        assert_eq!(migrated.settings.min_zone_pct, 1.0);
        assert_eq!(migrated.settings.app_list_cap, 10);
        assert_eq!(migrated.settings.snap_points, vec![25.0, 50.0, 75.0]);
    }

    #[test]
    fn migrate_clamps_out_of_range_gap_on_load() {
        let json = r#"{"profiles":{},"active_profile":"","gap":999}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(migrate(cfg).gap, 20);

        let json = r#"{"profiles":{},"active_profile":"","gap":-5}"#;
        let cfg: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(migrate(cfg).gap, 0);
    }
}
