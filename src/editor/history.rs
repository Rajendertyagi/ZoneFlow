//! Undo/redo history support.
//!
//! These are `impl Editor` methods that operate on the editor's history stack.
//! As a child module of `editor`, this module can access `Editor`'s private
//! fields and methods.

use crate::config::AppConfig;
use crate::editor::state::{LogClass, Snapshot};
use crate::editor::Editor;

impl Editor {
    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            cfg_json: serde_json::to_string(&self.draft).unwrap_or_default(),
            selected_zone: self.selected_zone,
            res: self.res,
        }
    }

    pub(super) fn push_history(&mut self) {
        let snap = self.snapshot();
        if self.history.get(self.hindex) == Some(&snap) {
            return;
        }
        self.history.truncate(self.hindex + 1);
        self.history.push(snap);
        let depth = self.cfg().settings.undo_depth.max(1);
        if self.history.len() > depth {
            self.history.remove(0);
        }
        self.hindex = self.history.len() - 1;
    }

    pub(super) fn apply_snapshot(&mut self, snap: &Snapshot) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&snap.cfg_json) {
            self.draft = cfg;
        }
        self.selected_zone = snap.selected_zone;
        self.res = snap.res;
        self.fields_zone = -1; // force inspector buffer resync
        self.merge_mode = false;
        self.boundary_drag = None;
        self.zone_drag = None;
        self.draw_start = None;
        self.draw_current = None;
        self.hud = None;
    }

    pub(super) fn undo(&mut self) {
        if self.hindex == 0 {
            self.toast("Nothing to undo");
            return;
        }
        self.hindex -= 1;
        let snap = self.history[self.hindex].clone();
        self.apply_snapshot(&snap);
        self.log("Undo", LogClass::Good);
    }

    pub(super) fn redo(&mut self) {
        if self.hindex + 1 >= self.history.len() {
            self.toast("Nothing to redo");
            return;
        }
        self.hindex += 1;
        let snap = self.history[self.hindex].clone();
        self.apply_snapshot(&snap);
        self.log("Redo", LogClass::Good);
    }
}
