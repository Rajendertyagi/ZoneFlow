//! Integration-style tests for the on-disk config JSON contract (§10.1).
//!
//! `screenlayout` is a binary crate with no `src/lib.rs`, so a `tests/*.rs`
//! file cannot `use screenlayout::config::...`. Instead these tests validate
//! the serialized JSON *contract* directly with `serde` / `serde_json`, using
//! a small local mirror of the schema defined in `src/config.rs`. See
//! `tests/README.md` for the rationale.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Current on-disk schema version (mirrors `config::SCHEMA_VERSION`).
const SCHEMA_VERSION: u32 = 1;

/// Field-level default for `schema_version`: a missing value means "v0", which
/// is how `migrate()` detects configs that need upgrading (mirrors `zero()`).
fn zero() -> u32 {
    0
}

fn default_snap_points() -> Vec<f32> {
    vec![25.0, 50.0, 75.0]
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

fn default_key_delay() -> u32 {
    80
}

fn default_full_pct() -> f32 {
    100.0
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

/* ---- Local mirror of the config JSON schema ---- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum CloseAction {
    #[default]
    Ask,
    Minimize,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolutionPreset {
    pub label: String,
    pub width: u32,
    pub work_height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Zone {
    id: i32,
    label: String,
    exe: String,
    /// Newer field: absent in old configs, must default to empty.
    notes: String,
    macro_text: String,
    #[serde(default = "default_key_delay")]
    key_delay: u32,
    x: f32,
    y: f32,
    #[serde(default = "default_full_pct")]
    w: f32,
    #[serde(default = "default_full_pct")]
    h: f32,
}

impl Default for Zone {
    fn default() -> Self {
        Self {
            id: 0,
            label: String::new(),
            exe: String::new(),
            notes: String::new(),
            macro_text: String::new(),
            key_delay: 80,
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
struct Profile {
    name: String,
    hotkey: String,
    zones: Vec<Zone>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    #[serde(default = "default_snap_points")]
    snap_points: Vec<f32>,
    grid_cols: u32,
    grid_rows: u32,
    /// Newer tunables: absent in old configs, must fall back to these values.
    #[serde(default = "default_app_list_cap")]
    app_list_cap: usize,
    #[serde(default = "default_running_apps_ttl_secs")]
    running_apps_ttl_secs: u64,
    #[serde(default = "default_taskbar_h")]
    taskbar_h: f32,
    #[serde(default = "default_stage_lower_h")]
    stage_lower_h: f32,
    auto_launch_on_startup: bool,
    #[serde(default = "default_resolutions")]
    resolutions: Vec<ResolutionPreset>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            snap_points: default_snap_points(),
            grid_cols: 12,
            grid_rows: 8,
            app_list_cap: default_app_list_cap(),
            running_apps_ttl_secs: default_running_apps_ttl_secs(),
            taskbar_h: default_taskbar_h(),
            stage_lower_h: default_stage_lower_h(),
            auto_launch_on_startup: false,
            resolutions: default_resolutions(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct AppConfig {
    /// Missing in v0 configs — defaults to 0 so migration can detect them.
    #[serde(default = "zero")]
    schema_version: u32,
    profiles: BTreeMap<String, Profile>,
    active_profile: String,
    gap: i32,
    close_action: CloseAction,
    auto_start: bool,
    hotkeys: BTreeMap<String, String>,
    settings: Settings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            profiles: BTreeMap::new(),
            active_profile: String::new(),
            gap: 8,
            close_action: CloseAction::default(),
            auto_start: false,
            hotkeys: BTreeMap::new(),
            settings: Settings::default(),
        }
    }
}

/* ---- Tests ---- */

#[test]
fn v0_config_missing_new_fields_uses_defaults() {
    // A minimal v0-style config: no `schema_version`, no `settings`, and no
    // newer tunables. Built with `json!` to exercise the raw JSON contract.
    let raw = serde_json::json!({
        "profiles": {},
        "active_profile": "",
        "gap": 5,
    });

    let cfg: AppConfig = serde_json::from_value(raw).expect("v0 config must deserialize");

    // Missing schema_version is detected as v0 (field-level default = 0),
    // while the struct-level default would have produced 1.
    assert_eq!(cfg.schema_version, 0);
    assert_eq!(cfg.gap, 5);

    // The whole `settings` object was absent, so every documented default
    // must be filled in — this is what makes forward migration lossless.
    assert_eq!(cfg.settings.grid_cols, 12);
    assert_eq!(cfg.settings.grid_rows, 8);
    assert_eq!(cfg.settings.snap_points, vec![25.0, 50.0, 75.0]);
    assert_eq!(cfg.settings.app_list_cap, 200);
    assert_eq!(cfg.settings.running_apps_ttl_secs, 2);
    assert_eq!(cfg.settings.taskbar_h, 16.0);
    assert_eq!(cfg.settings.stage_lower_h, 200.0);
    assert!(!cfg.settings.auto_launch_on_startup);
    assert_eq!(cfg.settings.resolutions.len(), 3);
    assert_eq!(cfg.settings.resolutions[2].width, 1920);
    assert_eq!(cfg.settings.resolutions[2].work_height, 1032);
}

#[test]
fn v0_zone_missing_notes_defaults_to_empty() {
    // Old zones have no `notes` field; it must default to empty rather than
    // failing to deserialize. Geometry defaults (w/h = 100) apply too.
    let raw = serde_json::json!({
        "id": 1,
        "label": "Browser",
        "exe": "chrome.exe",
        "macro_text": "{LEFT}",
    });

    let zone: Zone = serde_json::from_value(raw).expect("old zone must deserialize");
    assert_eq!(zone.notes, "");
    assert_eq!(zone.macro_text, "{LEFT}");
    assert_eq!(zone.key_delay, 80);
    assert_eq!(zone.w, 100.0);
    assert_eq!(zone.h, 100.0);
}

#[test]
fn v1_config_round_trips() {
    // Build a fully populated v1 config, including the newer fields.
    let mut cfg = AppConfig {
        schema_version: SCHEMA_VERSION,
        gap: 12,
        close_action: CloseAction::Minimize,
        auto_start: true,
        ..AppConfig::default()
    };

    let mut zone = Zone {
        id: 7,
        label: "Terminal".into(),
        exe: "wt.exe".into(),
        notes: "merged app: old [old.exe]".into(),
        x: 0.0,
        y: 0.0,
        w: 50.0,
        h: 100.0,
        ..Zone::default()
    };
    zone.key_delay = 120;

    let profile = Profile {
        name: "Coding".into(),
        hotkey: "Ctrl+Shift+2".into(),
        zones: vec![zone],
    };
    cfg.profiles.insert("Coding".into(), profile);
    cfg.active_profile = "Coding".into();
    cfg.hotkeys.insert("reload".into(), "Ctrl+Alt+R".into());
    cfg.settings.auto_launch_on_startup = true;
    cfg.settings.app_list_cap = 250;

    // Serialize → deserialize must reproduce the exact same value.
    let json = serde_json::to_string_pretty(&cfg).expect("v1 config must serialize");
    let back: AppConfig = serde_json::from_str(&json).expect("v1 config must deserialize");
    assert_eq!(back, cfg);

    // The serialized document must carry the version and the newer fields —
    // i.e. the on-disk contract is stable and self-describing.
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(doc["schema_version"], SCHEMA_VERSION);
    assert_eq!(doc["settings"]["auto_launch_on_startup"], true);
    assert_eq!(
        doc["profiles"]["Coding"]["zones"][0]["notes"],
        "merged app: old [old.exe]"
    );
}
