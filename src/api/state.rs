//! Authoritative in-memory state owned exclusively by the command worker.
//!
//! `Inner` is never shared: every mutation happens on the worker thread in
//! response to one serialized `Command`. HTTP handlers only observe snapshots
//! returned over oneshot replies.

use super::dto::{
    AppEntryDto, EngineStatusDto, HotkeyDto, LogLineDto, ProfileSummary, RectDto, StateSnapshot,
    ZoneDto,
};
use crate::config::AppConfig;
use std::collections::VecDeque;

/// Maximum number of retained log lines (oldest are dropped first).
const LOG_CAP: usize = 500;

/// One undo/redo entry. The draft config is stored as JSON so snapshots are
/// cheap to clone and compare (the same representation used for dirty checks).
///
/// The two profile dimensions are captured alongside the draft because profile
/// commands (rename/delete/create/load) mutate `selected_profile` and
/// `applied_profile` in the same atomic step as the draft. Restoring only the
/// draft would leave those names pointing at profiles that no longer exist in
/// the restored config (e.g. after undoing a rename), breaking `Apply` and the
/// drag overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub cfg_json: String,
    pub selected_zone: i32,
    pub selected_profile: String,
    pub applied_profile: String,
}

/// One console log line with a monotonic sequence number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    pub seq: u64,
    pub level: String,
    pub message: String,
}

impl From<&LogLine> for LogLineDto {
    fn from(l: &LogLine) -> Self {
        Self {
            seq: l.seq,
            level: l.level.clone(),
            message: l.message.clone(),
        }
    }
}

/// The authoritative draft state. All fields are private to the worker module
/// except through the accessor methods below.
pub struct Inner {
    /// The editable configuration (may differ from disk).
    pub draft: AppConfig,
    /// Serialized config at the last successful save/load — the dirty baseline.
    pub last_saved_json: String,
    /// Profile whose layout is currently applied to Windows (runtime only).
    pub applied_profile: String,
    /// Profile currently selected in Home / tray (runtime only, distinct from
    /// both the applied profile and the draft's `active_profile`).
    pub selected_profile: String,
    /// D4: runtime-only automation suspension. Resets to `true` on restart.
    pub layout_enabled: bool,
    /// Zone currently selected in the editor canvas (-1 = none).
    pub selected_zone: i32,
    /// Undo history.
    pub history: Vec<Snapshot>,
    /// Index of the current history entry.
    pub hindex: usize,
    /// Bounded console log.
    pub logs: VecDeque<LogLine>,
    /// Next log sequence number.
    next_seq: u64,
}

impl Inner {
    /// Initialize from a freshly loaded config. The draft starts clean, the
    /// selected and applied profiles both start at the config's active profile,
    /// and layout automation starts enabled (D4 runtime-only default).
    pub fn new(cfg: AppConfig) -> Self {
        let last_saved_json = serde_json::to_string(&cfg).unwrap_or_default();
        let active = cfg.active_profile.clone();
        let history = vec![Snapshot {
            cfg_json: last_saved_json.clone(),
            selected_zone: -1,
            selected_profile: active.clone(),
            applied_profile: active.clone(),
        }];
        Self {
            draft: cfg,
            last_saved_json,
            applied_profile: active.clone(),
            selected_profile: active,
            layout_enabled: true,
            selected_zone: -1,
            history,
            hindex: 0,
            logs: VecDeque::new(),
            next_seq: 1,
        }
    }

    /// D2: the draft is dirty when its serialization differs from the last
    /// saved/loaded baseline. `AppConfig` uses `BTreeMap`, so serialization is
    /// deterministic and this comparison is stable.
    pub fn is_dirty(&self) -> bool {
        serde_json::to_string(&self.draft).unwrap_or_default() != self.last_saved_json
    }

    /// Current history snapshot (without pushing). Captures all four dimensions
    /// that profile/zone commands can mutate atomically with the draft.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            cfg_json: serde_json::to_string(&self.draft).unwrap_or_default(),
            selected_zone: self.selected_zone,
            selected_profile: self.selected_profile.clone(),
            applied_profile: self.applied_profile.clone(),
        }
    }

    /// Record the current state as a new undo point. No-op when unchanged.
    pub fn push_history(&mut self) {
        let snap = self.snapshot();
        if self.history.get(self.hindex) == Some(&snap) {
            return;
        }
        self.history.truncate(self.hindex + 1);
        self.history.push(snap);
        let depth = self.draft.settings.undo_depth.max(1);
        if self.history.len() > depth {
            self.history.remove(0);
        }
        self.hindex = self.history.len() - 1;
    }

    /// Restore a history snapshot into the draft and the two runtime profile
    /// dimensions. All-or-nothing: an invalid `cfg_json` leaves every dimension
    /// untouched. Profile names are restored only when they exist in the
    /// restored draft, so no undo/redo can leave a dangling name behind.
    pub fn apply_snapshot(&mut self, snap: &Snapshot) {
        let Ok(cfg) = serde_json::from_str::<AppConfig>(&snap.cfg_json) else {
            tracing::warn!("undo snapshot failed to deserialize; state left unchanged");
            return;
        };
        self.draft = cfg;
        // Guarantee the restored config has a valid active profile before any
        // fallback reads `active_profile` below.
        self.draft.validate();
        self.selected_zone = snap.selected_zone;

        // Selected: restore when valid, otherwise fall back to the restored
        // draft's active profile (always present after `validate()`).
        if self.draft.profiles.contains_key(&snap.selected_profile) {
            self.selected_profile = snap.selected_profile.clone();
        } else {
            self.selected_profile = self.draft.active_profile.clone();
        }

        // Applied: restore when valid, otherwise clear it — the same D-rule
        // `ProfileDelete` uses when the applied profile disappears.
        if self.draft.profiles.contains_key(&snap.applied_profile) {
            self.applied_profile = snap.applied_profile.clone();
        } else {
            self.applied_profile.clear();
        }
    }

    /// Move one step back in history. Returns `false` when already at the start.
    pub fn undo(&mut self) -> bool {
        if self.hindex == 0 {
            return false;
        }
        self.hindex -= 1;
        let snap = self.history[self.hindex].clone();
        self.apply_snapshot(&snap);
        true
    }

    /// Move one step forward in history. Returns `false` at the end.
    pub fn redo(&mut self) -> bool {
        if self.hindex + 1 >= self.history.len() {
            return false;
        }
        self.hindex += 1;
        let snap = self.history[self.hindex].clone();
        self.apply_snapshot(&snap);
        true
    }

    pub fn can_undo(&self) -> bool {
        self.hindex > 0
    }

    pub fn can_redo(&self) -> bool {
        self.hindex + 1 < self.history.len()
    }

    /// Append a log line, evicting the oldest when the cap is reached.
    pub fn log(&mut self, level: &str, message: impl Into<String>) {
        let line = LogLine {
            seq: self.next_seq,
            level: level.to_string(),
            message: message.into(),
        };
        self.next_seq = self.next_seq.wrapping_add(1);
        if self.logs.len() == LOG_CAP {
            self.logs.pop_front();
        }
        self.logs.push_back(line);
    }

    /// Log lines with `seq > since`, oldest first.
    pub fn logs_since(&self, since: u64) -> Vec<LogLineDto> {
        self.logs
            .iter()
            .filter(|l| l.seq > since)
            .map(LogLineDto::from)
            .collect()
    }

    /// Discard the draft and reload it from the last saved baseline (D2
    /// `discard_dirty=true` path). Does not change applied/selected profiles.
    pub fn revert_draft(&mut self) {
        match serde_json::from_str::<AppConfig>(&self.last_saved_json) {
            Ok(cfg) => self.draft = cfg,
            Err(e) => tracing::warn!(error = %e, "revert failed: saved baseline is invalid"),
        }
    }

    /// Re-read the authoritative config from disk (legacy "Reload config"
    /// hotkey / tray item). Unlike [`Self::revert_draft`], this picks up external
    /// edits and refreshes the dirty baseline, matching the pre-headless
    /// behaviour. Applied/selected names are repaired if the reloaded config no
    /// longer contains them.
    pub fn reload_from_disk(&mut self) {
        let cfg = crate::config::load_config();
        if !cfg.profiles.contains_key(&self.applied_profile) {
            self.applied_profile = cfg.active_profile.clone();
        }
        if !cfg.profiles.contains_key(&self.selected_profile) {
            self.selected_profile = cfg.active_profile.clone();
        }
        self.draft = cfg.clone();
        self.last_saved_json = serde_json::to_string(&cfg).unwrap_or_default();
    }

    /// Zone rectangles used for the drag overlay and drag-release hit testing.
    ///
    /// Both the overlay cache (worker → native `SetZones`) and the `MoveWindow`
    /// command MUST derive their geometry from this single source. They key off
    /// `selected_profile` (the operational profile used by every other action —
    /// apply, launch, hotkeys), so a Home `select` that leaves the draft's
    /// `active_profile` unchanged cannot make the overlay show one profile's
    /// zones while the drop snaps to another's.
    pub fn drag_zones(&self) -> Vec<(i32, crate::engine_core::Rect)> {
        let mut cfg = self.draft.clone();
        cfg.active_profile = self.selected_profile.clone();
        let area = crate::engine::work_area();
        crate::engine_core::compute_zones(cfg.active(), area, cfg.gap)
    }

    /// Persist the current draft: write to disk, then update the dirty baseline.
    pub fn save(&mut self) -> Result<(), String> {
        crate::config::save_config(&self.draft).map_err(|e| e.to_string())?;
        self.last_saved_json = serde_json::to_string(&self.draft).unwrap_or_default();
        Ok(())
    }

    /// Build the full `GET /api/state` snapshot. `work_area` is supplied by the
    /// caller (the worker reads it from `engine::work_area()`) so this method
    /// stays pure and unit-testable.
    pub fn state_snapshot(&self, work_area: RectDto) -> StateSnapshot {
        let profiles: Vec<ProfileSummary> = self
            .draft
            .profiles
            .values()
            .map(|p| ProfileSummary::new(p, &self.applied_profile, &self.selected_profile))
            .collect();
        let zones: Vec<ZoneDto> = self
            .draft
            .active()
            .zones
            .iter()
            .map(ZoneDto::from)
            .collect();
        let hotkeys: Vec<HotkeyDto> = self
            .draft
            .hotkeys
            .iter()
            .map(|(name, binding)| HotkeyDto {
                name: name.clone(),
                binding: binding.clone(),
            })
            .collect();
        let logs: Vec<LogLineDto> = self.logs.iter().map(LogLineDto::from).collect();
        StateSnapshot {
            draft: self.draft.clone(),
            dirty: self.is_dirty(),
            applied: self.applied_profile.clone(),
            selected: self.selected_profile.clone(),
            enabled: self.layout_enabled,
            selected_zone: self.selected_zone,
            profiles,
            zones,
            settings: super::dto::SettingsDto::from(&self.draft.settings),
            hotkeys,
            logs,
            status: EngineStatusDto {
                work_area,
                zone_count: self.draft.active().zones.len(),
                applied: self.applied_profile.clone(),
                enabled: self.layout_enabled,
            },
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
        }
    }

    /// Build a lightweight app list DTO from discovered entries.
    pub fn app_dtos(apps: &[crate::apps::AppEntry]) -> Vec<AppEntryDto> {
        apps.iter()
            .map(|a| AppEntryDto {
                id: a.id.clone(),
                label: a.label.clone(),
                exe: a.exe.clone(),
                path: a.path.clone(),
            })
            .collect()
    }
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{default_config, Profile, Zone};

    fn cfg() -> AppConfig {
        default_config()
    }

    #[test]
    fn new_state_starts_clean_and_enabled() {
        let s = Inner::new(cfg());
        assert!(!s.is_dirty());
        assert!(s.layout_enabled);
        assert_eq!(s.applied_profile, "Website");
        assert_eq!(s.selected_profile, "Website");
        assert_eq!(s.selected_zone, -1);
        assert!(!s.can_undo());
        assert!(!s.can_redo());
    }

    #[test]
    fn dirty_check_detects_any_draft_change() {
        let mut s = Inner::new(cfg());
        assert!(!s.is_dirty());
        s.draft.gap = 99;
        assert!(s.is_dirty());
        s.revert_draft();
        assert!(!s.is_dirty());
        assert_eq!(s.draft.gap, 8);
    }

    #[test]
    fn save_clears_dirty_baseline() {
        let mut s = Inner::new(cfg());
        s.draft.gap = 12;
        assert!(s.is_dirty());
        // Avoid touching the real config file: update the baseline directly the
        // same way `save` does after a successful disk write.
        s.last_saved_json = serde_json::to_string(&s.draft).unwrap();
        assert!(!s.is_dirty());
    }

    #[test]
    fn history_undo_redo_round_trip() {
        let mut s = Inner::new(cfg());
        s.draft.gap = 10;
        s.push_history();
        s.draft.gap = 20;
        s.push_history();
        assert_eq!(s.draft.gap, 20);
        assert!(s.undo());
        assert_eq!(s.draft.gap, 10);
        assert!(s.undo());
        assert_eq!(s.draft.gap, 8);
        assert!(!s.undo());
        assert!(s.redo());
        assert_eq!(s.draft.gap, 10);
        assert!(s.redo());
        assert_eq!(s.draft.gap, 20);
        assert!(!s.redo());
    }

    #[test]
    fn push_history_ignores_unchanged_state() {
        let mut s = Inner::new(cfg());
        s.push_history();
        assert_eq!(s.history.len(), 1);
        s.draft.gap = 5;
        s.push_history();
        s.push_history(); // no change since last push
        assert_eq!(s.history.len(), 2);
    }

    #[test]
    fn logs_are_bounded_and_since_filtered() {
        let mut s = Inner::new(cfg());
        for i in 0..(LOG_CAP + 10) {
            s.log("info", format!("line {i}"));
        }
        assert_eq!(s.logs.len(), LOG_CAP);
        let all = s.logs_since(0);
        assert_eq!(all.len(), LOG_CAP);
        let first_seq = all[0].seq;
        let tail = s.logs_since(first_seq);
        assert_eq!(tail.len(), LOG_CAP - 1);
    }

    #[test]
    fn state_snapshot_reports_independent_dimensions() {
        let mut s = Inner::new(cfg());
        s.applied_profile = "Coding".into();
        s.selected_profile = "Trading".into();
        s.layout_enabled = false;
        s.draft.gap = 3;
        let snap = s.state_snapshot(RectDto {
            x: 0,
            y: 0,
            w: 1920,
            h: 1040,
        });
        assert!(snap.dirty);
        assert_eq!(snap.applied, "Coding");
        assert_eq!(snap.selected, "Trading");
        assert!(!snap.enabled);
        assert_eq!(snap.profiles.len(), 3);
        assert!(snap
            .profiles
            .iter()
            .any(|p| p.name == "Coding" && p.applied));
        assert!(snap
            .profiles
            .iter()
            .any(|p| p.name == "Trading" && p.selected));
    }

    #[test]
    fn state_snapshot_zones_come_from_active_profile() {
        let mut s = Inner::new(cfg());
        s.draft
            .active_mut()
            .zones
            .push(Zone::new(1, "A", "a.exe", 0.0, 0.0, 50.0, 100.0));
        let snap = s.state_snapshot(RectDto {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        });
        assert_eq!(snap.zones.len(), 1);
        assert_eq!(snap.zones[0].exe, "a.exe");
        assert_eq!(snap.status.zone_count, 1);
    }

    #[test]
    fn profile_summary_helper_handles_empty_profiles() {
        let p = Profile {
            name: "Empty".into(),
            hotkey: String::new(),
            zones: Vec::new(),
        };
        let s = ProfileSummary::new(&p, "Empty", "Other");
        assert_eq!(s.zone_count, 0);
        assert!(s.applied);
        assert!(!s.selected);
    }

    #[test]
    fn drag_zones_follow_selected_profile_not_draft() {
        let mut s = Inner::new(cfg());
        // The draft's active profile and the selected profile have different
        // zone ids. A Home `select` changes only `selected_profile`, so the
        // overlay cache and the drag-release snap must both follow the selected
        // profile — otherwise the drop lands in a rectangle the user never saw.
        s.draft
            .profiles
            .get_mut("Website")
            .unwrap()
            .zones
            .push(Zone::new(1, "Web", "web.exe", 0.0, 0.0, 100.0, 100.0));
        s.draft
            .profiles
            .get_mut("Coding")
            .unwrap()
            .zones
            .push(Zone::new(2, "Code", "code.exe", 0.0, 0.0, 50.0, 50.0));
        s.selected_profile = "Coding".into();
        assert_eq!(s.draft.active_profile, "Website");
        let zones = s.drag_zones();
        assert_eq!(zones.len(), 1);
        assert_eq!(
            zones[0].0, 2,
            "drag zones must come from the selected profile"
        );
    }

    #[test]
    fn snapshot_captures_all_four_dimensions() {
        let mut s = Inner::new(cfg());
        s.selected_zone = 4;
        s.selected_profile = "Trading".into();
        s.applied_profile = "Coding".into();
        s.draft.gap = 12;
        let snap = s.snapshot();
        assert_eq!(snap.selected_zone, 4);
        assert_eq!(snap.selected_profile, "Trading");
        assert_eq!(snap.applied_profile, "Coding");
        assert!(snap.cfg_json.contains("\"gap\":12"));
    }

    #[test]
    fn apply_snapshot_repairs_stale_profile_names() {
        let mut s = Inner::new(cfg());
        // Both names are absent from the restored draft: selected falls back to
        // the draft's active profile, applied is cleared (ProfileDelete D-rule).
        let snap = Snapshot {
            cfg_json: serde_json::to_string(&cfg()).unwrap(),
            selected_zone: 7,
            selected_profile: "Ghost".into(),
            applied_profile: "Phantom".into(),
        };
        s.apply_snapshot(&snap);
        assert_eq!(s.selected_zone, 7);
        assert_eq!(s.selected_profile, "Website");
        assert_eq!(s.applied_profile, "");
        assert!(s.draft.profiles.contains_key(&s.selected_profile));
    }

    #[test]
    fn apply_snapshot_restores_valid_names_and_repairs_only_stale_ones() {
        let mut s = Inner::new(cfg());
        // Selected is valid in the restored draft, applied is stale.
        let snap = Snapshot {
            cfg_json: serde_json::to_string(&cfg()).unwrap(),
            selected_zone: 2,
            selected_profile: "Coding".into(),
            applied_profile: "Phantom".into(),
        };
        s.apply_snapshot(&snap);
        assert_eq!(s.selected_profile, "Coding");
        assert_eq!(s.applied_profile, "");
        assert!(s.draft.profiles.contains_key(&s.selected_profile));
    }

    #[test]
    fn apply_snapshot_invalid_json_leaves_all_dimensions_unchanged() {
        let mut s = Inner::new(cfg());
        s.selected_zone = 3;
        s.selected_profile = "Coding".into();
        s.applied_profile = "Trading".into();
        s.draft.gap = 99;
        let snap = Snapshot {
            cfg_json: "not valid json".into(),
            selected_zone: 99,
            selected_profile: "Ghost".into(),
            applied_profile: "Phantom".into(),
        };
        s.apply_snapshot(&snap);
        assert_eq!(s.selected_zone, 3);
        assert_eq!(s.selected_profile, "Coding");
        assert_eq!(s.applied_profile, "Trading");
        assert_eq!(s.draft.gap, 99);
    }

    #[test]
    fn undo_of_zone_create_restores_prior_selected_zone() {
        let mut s = Inner::new(cfg());
        // Baseline snapshot has selected_zone = -1.
        s.draft
            .active_mut()
            .zones
            .push(Zone::new(1, "A", "a.exe", 0.0, 0.0, 100.0, 100.0));
        s.selected_zone = 1;
        s.push_history();
        assert_eq!(s.selected_zone, 1);
        assert!(s.undo());
        assert_eq!(s.selected_zone, -1);
        assert!(s.draft.active().zones.is_empty());
        assert!(s.redo());
        assert_eq!(s.selected_zone, 1);
        assert_eq!(s.draft.active().zones.len(), 1);
    }
}
