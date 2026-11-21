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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub cfg_json: String,
    pub selected_zone: i32,
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

    /// Current history snapshot (without pushing).
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            cfg_json: serde_json::to_string(&self.draft).unwrap_or_default(),
            selected_zone: self.selected_zone,
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

    /// Restore a history snapshot into the draft.
    pub fn apply_snapshot(&mut self, snap: &Snapshot) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&snap.cfg_json) {
            self.draft = cfg;
        } else {
            tracing::warn!("undo snapshot failed to deserialize; draft left unchanged");
        }
        self.selected_zone = snap.selected_zone;
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
}
