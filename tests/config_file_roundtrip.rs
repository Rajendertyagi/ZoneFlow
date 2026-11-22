//! Integration tests for the on-disk config JSON contract (§10.1).
//!
//! These tests import the REAL `screenlayout::config` types through the library
//! target (`src/lib.rs`). Unlike the old hand-written mirror, every persisted
//! field is asserted explicitly, so dropping a field from the real schema (or
//! changing its serde name/default) fails here instead of being silently
//! ignored by serde's "unknown fields are skipped" behaviour. See
//! `tests/README.md` for the rationale.

use screenlayout::config::{
    AppConfig, CloseAction, LaunchMode, Profile, ResolutionPreset, Settings, TabBehavior, Zone,
    SCHEMA_VERSION,
};
use std::collections::BTreeMap;

/* ---- Field-by-field equality helpers (AppConfig has no PartialEq derive) ---- */

fn assert_resolution_eq(a: &ResolutionPreset, b: &ResolutionPreset) {
    assert_eq!(a.label, b.label, "ResolutionPreset.label");
    assert_eq!(a.width, b.width, "ResolutionPreset.width");
    assert_eq!(a.work_height, b.work_height, "ResolutionPreset.work_height");
}

fn assert_zone_eq(a: &Zone, b: &Zone) {
    assert_eq!(a.id, b.id, "Zone.id");
    assert_eq!(a.label, b.label, "Zone.label");
    assert_eq!(a.exe, b.exe, "Zone.exe");
    assert_eq!(a.path, b.path, "Zone.path");
    assert_eq!(a.launcher_path, b.launcher_path, "Zone.launcher_path");
    assert_eq!(
        a.launcher_show_console, b.launcher_show_console,
        "Zone.launcher_show_console"
    );
    assert_eq!(a.launch_mode, b.launch_mode, "Zone.launch_mode");
    assert_eq!(a.title_match, b.title_match, "Zone.title_match");
    assert_eq!(a.url, b.url, "Zone.url");
    assert_eq!(a.tab_behavior, b.tab_behavior, "Zone.tab_behavior");
    assert_eq!(a.macro_enabled, b.macro_enabled, "Zone.macro_enabled");
    assert_eq!(a.trigger_title, b.trigger_title, "Zone.trigger_title");
    assert_eq!(a.macro_text, b.macro_text, "Zone.macro_text");
    assert_eq!(a.notes, b.notes, "Zone.notes");
    assert_eq!(a.key_delay, b.key_delay, "Zone.key_delay");
    assert_eq!(a.x, b.x, "Zone.x");
    assert_eq!(a.y, b.y, "Zone.y");
    assert_eq!(a.w, b.w, "Zone.w");
    assert_eq!(a.h, b.h, "Zone.h");
}

fn assert_profile_eq(a: &Profile, b: &Profile) {
    assert_eq!(a.name, b.name, "Profile.name");
    assert_eq!(a.hotkey, b.hotkey, "Profile.hotkey");
    assert_eq!(a.zones.len(), b.zones.len(), "Profile.zones length");
    for (za, zb) in a.zones.iter().zip(b.zones.iter()) {
        assert_zone_eq(za, zb);
    }
}

fn assert_settings_eq(a: &Settings, b: &Settings) {
    assert_eq!(a.snap_points, b.snap_points, "Settings.snap_points");
    assert_eq!(a.grid_cols, b.grid_cols, "Settings.grid_cols");
    assert_eq!(a.grid_rows, b.grid_rows, "Settings.grid_rows");
    assert_eq!(a.min_zone_pct, b.min_zone_pct, "Settings.min_zone_pct");
    assert_eq!(
        a.resize_step_pct, b.resize_step_pct,
        "Settings.resize_step_pct"
    );
    assert_eq!(
        a.boundary_tolerance_pct, b.boundary_tolerance_pct,
        "Settings.boundary_tolerance_pct"
    );
    assert_eq!(a.undo_depth, b.undo_depth, "Settings.undo_depth");
    assert_eq!(
        a.display_debounce_ms, b.display_debounce_ms,
        "Settings.display_debounce_ms"
    );
    assert_eq!(
        a.overlay_inactive_alpha, b.overlay_inactive_alpha,
        "Settings.overlay_inactive_alpha"
    );
    assert_eq!(
        a.overlay_active_alpha, b.overlay_active_alpha,
        "Settings.overlay_active_alpha"
    );
    assert_eq!(
        a.dwm_snap_retries, b.dwm_snap_retries,
        "Settings.dwm_snap_retries"
    );
    assert_eq!(a.min_window_px, b.min_window_px, "Settings.min_window_px");
    assert_eq!(a.zone_tints, b.zone_tints, "Settings.zone_tints");
    assert_eq!(
        a.auto_launch_on_startup, b.auto_launch_on_startup,
        "Settings.auto_launch_on_startup"
    );
    assert_eq!(
        a.resolutions.len(),
        b.resolutions.len(),
        "Settings.resolutions length"
    );
    for (ra, rb) in a.resolutions.iter().zip(b.resolutions.iter()) {
        assert_resolution_eq(ra, rb);
    }
    assert_eq!(a.app_list_cap, b.app_list_cap, "Settings.app_list_cap");
    assert_eq!(
        a.running_apps_ttl_secs, b.running_apps_ttl_secs,
        "Settings.running_apps_ttl_secs"
    );
    assert_eq!(a.taskbar_h, b.taskbar_h, "Settings.taskbar_h");
    assert_eq!(a.stage_lower_h, b.stage_lower_h, "Settings.stage_lower_h");
}

fn assert_config_eq(a: &AppConfig, b: &AppConfig) {
    assert_eq!(
        a.schema_version, b.schema_version,
        "AppConfig.schema_version"
    );
    assert_eq!(
        a.profiles.len(),
        b.profiles.len(),
        "AppConfig.profiles length"
    );
    for (key, pa) in &a.profiles {
        let pb = b
            .profiles
            .get(key)
            .unwrap_or_else(|| panic!("AppConfig.profiles missing key {key:?}"));
        assert_profile_eq(pa, pb);
    }
    assert_eq!(
        a.active_profile, b.active_profile,
        "AppConfig.active_profile"
    );
    assert_eq!(a.gap, b.gap, "AppConfig.gap");
    assert_eq!(a.close_action, b.close_action, "AppConfig.close_action");
    assert_eq!(a.auto_start, b.auto_start, "AppConfig.auto_start");
    assert_eq!(a.hotkeys, b.hotkeys, "AppConfig.hotkeys");
    assert_settings_eq(&a.settings, &b.settings);
}

/* ---- Tests ---- */

#[test]
fn v0_config_missing_new_fields_uses_defaults() {
    // A minimal v0-style config: no `schema_version`, no `settings`, and no
    // newer tunables. Built with `json!` to exercise the raw JSON contract
    // against the real deserializer.
    let raw = serde_json::json!({
        "profiles": {},
        "active_profile": "",
        "gap": 5,
    });

    let cfg: AppConfig = serde_json::from_value(raw).expect("v0 config must deserialize");

    // Missing schema_version is detected as v0 (field-level default = 0),
    // while the struct-level default would have produced SCHEMA_VERSION.
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
    assert_eq!(zone.launcher_path, "");
    assert!(zone.launcher_show_console);
    assert_eq!(zone.macro_text, "{LEFT}");
    assert_eq!(zone.key_delay, 80);
    assert_eq!(zone.w, 100.0);
    assert_eq!(zone.h, 100.0);
}

#[test]
fn v1_config_round_trips_every_persisted_field() {
    // Build a fully populated v1 config with NON-DEFAULT values in every
    // persisted field. If any field were dropped from the real schema, either
    // this test would not compile or the field-by-field assertion would fail.
    // `AppConfig` has a private `fallback` field, so it cannot be built with a
    // struct literal from this external crate. Start from the public default
    // and overwrite every persisted field with a non-default value.
    let mut cfg = AppConfig::default();
    cfg.schema_version = SCHEMA_VERSION;
    cfg.profiles = BTreeMap::new();
    cfg.active_profile = "Coding".into();
    cfg.gap = 12;
    cfg.close_action = CloseAction::Minimize;
    cfg.auto_start = true;
    cfg.hotkeys = BTreeMap::from([
        ("reload".to_string(), "Ctrl+Alt+R".to_string()),
        ("profile_1".to_string(), "Alt+1".to_string()),
    ]);
    cfg.settings = Settings {
        snap_points: vec![10.0, 33.3, 90.0],
        grid_cols: 9,
        grid_rows: 6,
        min_zone_pct: 12.5,
        resize_step_pct: 2.5,
        boundary_tolerance_pct: 2.0,
        undo_depth: 42,
        display_debounce_ms: 150,
        overlay_inactive_alpha: 180,
        overlay_active_alpha: 240,
        dwm_snap_retries: 3,
        min_window_px: 120,
        zone_tints: vec!["#123456".into(), "#ABCDEF".into()],
        auto_launch_on_startup: true,
        resolutions: vec![ResolutionPreset {
            label: "Custom".into(),
            width: 1234,
            work_height: 567,
        }],
        app_list_cap: 250,
        running_apps_ttl_secs: 5,
        taskbar_h: 32.0,
        stage_lower_h: 180.0,
    };

    let zone = Zone {
        id: 7,
        label: "Terminal".into(),
        exe: "wt.exe".into(),
        path: r"C:\Windows\System32\wt.exe".into(),
        launcher_path: r"C:\Scripts\launch-wt.cmd".into(),
        launcher_show_console: false,
        launch_mode: LaunchMode::Attach,
        title_match: "Windows Terminal".into(),
        url: "https://example.com".into(),
        tab_behavior: TabBehavior::ReuseBlank,
        macro_enabled: true,
        trigger_title: "Main".into(),
        macro_text: "^t".into(),
        notes: "merged app: old [old.exe]".into(),
        key_delay: 120,
        x: 1.0,
        y: 2.0,
        w: 50.0,
        h: 75.0,
    };

    cfg.profiles.insert(
        "Coding".into(),
        Profile {
            name: "Coding".into(),
            hotkey: "Ctrl+Shift+2".into(),
            zones: vec![zone],
        },
    );

    // Serialize → deserialize must reproduce the exact same value, field by field.
    let json = serde_json::to_string_pretty(&cfg).expect("v1 config must serialize");
    let back: AppConfig = serde_json::from_str(&json).expect("v1 config must deserialize");
    assert_config_eq(&back, &cfg);

    // The serialized document must carry the version and the newer fields —
    // i.e. the on-disk contract is stable and self-describing.
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(doc["schema_version"], SCHEMA_VERSION);
    assert_eq!(doc["settings"]["auto_launch_on_startup"], true);
    assert_eq!(doc["settings"]["app_list_cap"], 250);
    assert_eq!(
        doc["profiles"]["Coding"]["zones"][0]["notes"],
        "merged app: old [old.exe]"
    );
    assert_eq!(
        doc["profiles"]["Coding"]["zones"][0]["launcher_path"],
        r"C:\Scripts\launch-wt.cmd"
    );
    assert_eq!(
        doc["profiles"]["Coding"]["zones"][0]["launcher_show_console"],
        false
    );
    assert_eq!(
        doc["profiles"]["Coding"]["zones"][0]["launch_mode"],
        "attach"
    );
    assert_eq!(
        doc["profiles"]["Coding"]["zones"][0]["tab_behavior"],
        "reuse_blank"
    );
}
