use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Current on-disk schema version. Bump when the structure changes incompatibly.
pub const SCHEMA_VERSION: u32 = 1;

fn zero() -> u32 {
    0
}

/* ---- Defaults for the domain tunables moved out of editor code (§2.2) ---- */

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
    pub path: String,
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

/// A user-editable resolution preset for the editor's resolution selector
/// (§2d item 11). Previously a hardcoded tuple table in `editor/state.rs`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolutionPreset {
    /// Label shown in the resolution selector.
    pub label: String,
    /// Screen width in pixels (used to convert zone percentages to pixels).
    pub width: u32,
    /// Work-area height in pixels (screen height minus the taskbar).
    pub work_height: u32,
}

/* ============================ Settings (all tunables) ============================ */

/// Every tunable from the plan (§2.2). All values are defaults stored in config,
/// never literals in code. Kept in a separate struct so the top-level fields that
/// engine.rs/editor.rs already use (gap, close_action, auto_start) stay compatible.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
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
    /// Launch and snap every bound app on startup (AHK `AutoLaunchAndSnapAll`
    /// parity, §2c). Defaults to `false` so development runs never surprise-launch
    /// apps; enable it here (or in the Settings view) for normal use.
    #[serde(default)]
    pub auto_launch_on_startup: bool,
    /// Resolution presets shown in the editor's resolution selector (§2d item 11).
    #[serde(default = "default_resolutions")]
    pub resolutions: Vec<ResolutionPreset>,
    /// Maximum number of installed apps shown in the library / picker.
    #[serde(default = "default_app_list_cap")]
    pub app_list_cap: usize,
    /// Seconds a running-app enumeration stays fresh before it is re-run.
    #[serde(default = "default_running_apps_ttl_secs")]
    pub running_apps_ttl_secs: u64,
    /// Height of the fake taskbar strip drawn at the bottom of the canvas.
    #[serde(default = "default_taskbar_h")]
    pub taskbar_h: f32,
    /// Vertical space reserved below the canvas for hint/action/apps bars.
    #[serde(default = "default_stage_lower_h")]
    pub stage_lower_h: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            snap_points: vec![25.0, 50.0, 75.0],
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

/* ---- Settings snapshot for free functions without an Editor ---------- */

/// Snapshot of the settings loaded at startup. Editor-free helpers (the
/// resolution selector and app enumeration) read their tunables from here so
/// the values live in config rather than as literals in those modules.
static LOADED_SETTINGS: OnceLock<Settings> = OnceLock::new();

/// The loaded settings, or defaults if no config has been loaded yet (tests).
pub fn loaded_settings() -> &'static Settings {
    LOADED_SETTINGS.get_or_init(Settings::default)
}

/// Resolution presets used by `editor::state::ResKey`.
pub fn resolutions() -> &'static [ResolutionPreset] {
    &loaded_settings().resolutions
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
    // Publish the loaded settings for editor-free helpers (resolution selector,
    // app enumeration) that have no `Editor` instance to read from.
    let _ = LOADED_SETTINGS.set(cfg.settings.clone());
    cfg
}

fn load_config_inner() -> AppConfig {
    let path = config_path();
    if !path.exists() {
        let cfg = default_config();
        if let Err(e) = save_config(&cfg) {
            tracing::warn!(error = %e, "Failed to write default config");
        }
        return cfg;
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return default_config(),
    };
    let cfg: AppConfig = match serde_json::from_str(&text) {
        Ok(c) => c,
        Err(_) => {
            // Preserve the unreadable file for recovery instead of silently wiping it.
            let bad = path.with_extension("json.bad");
            if let Err(e) = fs::rename(&path, &bad) {
                tracing::warn!(error = %e, "Failed to preserve unreadable config for recovery");
            }
            return default_config();
        }
    };
    migrate(cfg)
}

pub fn save_config(cfg: &AppConfig) -> std::io::Result<()> {
    let path = config_path();
    let text = serde_json::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text)?;
    // On Windows, rename fails if the destination exists. Remove first, then rename.
    // (Small crash window; the app keeps config in memory and rewrites on next save.)
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(&tmp, &path)
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
}
