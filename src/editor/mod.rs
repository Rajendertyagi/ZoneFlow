use crate::config::{self, AppConfig, CloseAction, Zone};
use crate::engine;
use crate::overlay::{self, SharedOverlay};
use crate::system::{Action, System};
use eframe::egui;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use state::{
    AppEntry, BoundaryDrag, DragApp, EditBuffers, LogClass, LogLine, PickerState, RectSpec, ResKey,
    Snapshot, StrField, TextDialogKind, TextDialogState, View, WizardState, ZoneDrag,
};
use theme::CanvasTheme;
use tokens::*;

mod action_bar;
mod canvas;
mod dialogs;
mod history;
mod inspector;
mod rail;
mod state;
mod theme;
mod tokens;
mod toolbar;
mod top_bar;
mod views;

/* ============================================================================
 * EDITOR
 * ========================================================================== */

pub struct Editor {
    draft: AppConfig,
    last_saved_json: String,
    view: View,
    selected_zone: i32,
    res: ResKey,

    history: Vec<Snapshot>,
    hindex: usize,

    boundary_drag: Option<BoundaryDrag>,
    zone_drag: Option<ZoneDrag>,
    draw_active: bool,
    draw_start: Option<egui::Vec2>,
    draw_current: Option<egui::Vec2>,
    snap_draw: bool,
    hud: Option<String>,
    snap_guide: Option<f32>,
    snap_guide_vertical: bool,
    drop_target: Option<i32>,

    merge_mode: bool,
    merge_source: i32,

    wizard_open: bool,
    wizard: WizardState,
    picker_open: bool,
    picker: PickerState,
    help_open: bool,
    reset_confirm_open: bool,
    text_dialog: Option<TextDialogState>,
    close_dialog: bool,
    /// Set when the user confirms exit (dialog or tray) so the close_requested
    /// handler does not re-open the Ask dialog and cancel the close (infinite loop).
    exiting: bool,

    edit_bufs: EditBuffers,
    fields_zone: i32,
    edit_session_active: bool,

    console: Vec<LogLine>,
    exec_queue: VecDeque<LogLine>,
    exec_next: Instant,

    lib_search: String,
    rail_tab: state::RailTab,
    inspector_tab: state::InspectorTab,
    toast: Option<(String, Instant)>,

    /// Short-TTL cache for the "Running now" picker tab. Unlike installed apps
    /// (which rarely change and stay in the process-wide OnceLock), the running
    /// set is re-enumerated on tab open / when this cache expires (N1).
    running_apps_cache: Option<(Instant, Vec<AppEntry>)>,

    system: Option<System>,
    overlay: SharedOverlay,
    drag_hwnd: Option<isize>,

    /// One-shot guard for AHK `AutoLaunchAndSnapAll` parity (§2c / T1).
    auto_launched: bool,
    /// Results from the worker thread that performs startup auto-launch, drained
    /// on the UI thread so `execute_zone`'s blocking waits never stall a frame.
    auto_launch_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,

    /// Baseline work area for display-change detection (§2c / T2). `None` until
    /// the first poll, which only records the baseline and never re-tiles.
    last_work_area: Option<engine::Rect>,
    last_work_area_check: Instant,
}

impl Default for Editor {
    fn default() -> Self {
        let draft = config::load_config();
        // Serializing an in-memory AppConfig to JSON is effectively infallible;
        // `unwrap_or_default` yields "" on the impossible failure, which simply
        // marks the editor as "always dirty" (safe, no behavior change). The same
        // reasoning applies to every `last_saved_json` assignment here and in
        // history.rs / dialogs.rs, so those are intentionally left as-is.
        let last_saved_json = serde_json::to_string(&draft).unwrap_or_default();
        let selected_zone = draft.active().zones.first().map(|z| z.id).unwrap_or(0);
        let history = vec![Snapshot {
            cfg_json: last_saved_json.clone(),
            selected_zone,
            res: ResKey::Fhd,
        }];
        Self {
            draft,
            last_saved_json,
            view: View::Canvas,
            selected_zone,
            res: ResKey::Fhd,
            history,
            hindex: 0,
            boundary_drag: None,
            zone_drag: None,
            draw_active: false,
            draw_start: None,
            draw_current: None,
            snap_draw: true,
            hud: None,
            snap_guide: None,
            snap_guide_vertical: true,
            drop_target: None,
            merge_mode: false,
            merge_source: 0,
            wizard_open: false,
            wizard: WizardState::default(),
            picker_open: false,
            picker: PickerState::default(),
            help_open: false,
            reset_confirm_open: false,
            text_dialog: None,
            close_dialog: false,
            exiting: false,
            edit_bufs: EditBuffers::default(),
            fields_zone: -1,
            edit_session_active: false,
            console: vec![
                LogLine {
                    class: LogClass::Good,
                    text: "ZoneFlow Studio ready.".into(),
                },
                LogLine {
                    class: LogClass::Accent,
                    text: "Engine ready - hotkeys active.".into(),
                },
            ],
            exec_queue: VecDeque::new(),
            exec_next: Instant::now(),
            lib_search: String::new(),
            rail_tab: state::RailTab::Apps,
            inspector_tab: state::InspectorTab::Process,
            toast: None,
            running_apps_cache: None,
            system: None,
            overlay: Arc::new(RwLock::new(overlay::OverlayState::default())),
            drag_hwnd: None,
            auto_launched: false,
            auto_launch_rx: None,
            last_work_area: None,
            last_work_area_check: Instant::now(),
        }
    }
}

/// Copy every app-binding / rule field from `src` into `dst` (geometry untouched).
fn copy_zone_binding(src: &Zone, dst: &mut Zone) {
    dst.label = src.label.clone();
    dst.exe = src.exe.clone();
    dst.path = src.path.clone();
    dst.launch_mode = src.launch_mode;
    dst.url = src.url.clone();
    dst.title_match = src.title_match.clone();
    dst.tab_behavior = src.tab_behavior;
    dst.macro_enabled = src.macro_enabled;
    dst.trigger_title = src.trigger_title.clone();
    dst.macro_text = src.macro_text.clone();
    dst.notes = src.notes.clone();
    dst.key_delay = src.key_delay;
}

/// Merge a removed zone's binding/rules into a surviving zone so nothing is
/// silently lost when a template has fewer zones than the current profile (§1).
/// A zone binds exactly one executable, so when `dst` is already bound we keep
/// the extra zone's data in the non-executable [`Zone::notes`] field rather than
/// dropping it or writing it into `macro_text` (which would be typed as keys).
/// When `dst` is still blank we adopt the whole binding.
fn merge_zone_binding(dst: &mut Zone, src: &Zone) {
    if src.exe.is_empty()
        && src.url.is_empty()
        && src.title_match.is_empty()
        && src.macro_text.is_empty()
        && src.notes.is_empty()
    {
        return;
    }
    if dst.exe.is_empty() {
        copy_zone_binding(src, dst);
        return;
    }
    let mut parts: Vec<String> = Vec::new();
    if !src.label.is_empty() || !src.exe.is_empty() {
        parts.push(format!("{} [{}]", src.label, src.exe));
    }
    if !src.url.is_empty() {
        parts.push(format!("url:{}", src.url));
    }
    if !src.title_match.is_empty() {
        parts.push(format!("title:{}", src.title_match));
    }
    if !src.macro_text.is_empty() {
        parts.push(format!("keys:{}", src.macro_text));
    }
    if !src.notes.is_empty() {
        parts.push(src.notes.trim().to_string());
    }
    if parts.is_empty() {
        return;
    }
    let note = format!("merged app: {}", parts.join(" | "));
    if !dst.notes.contains(&note) {
        if !dst.notes.is_empty() {
            dst.notes.push('\n');
        }
        dst.notes.push_str(&note);
    }
}

impl Editor {
    pub fn with_system(mut self) -> Self {
        match System::new() {
            Ok(s) => {
                if !s.failures.is_empty() {
                    // One concise amber line instead of dumping every binding.
                    self.log(
                        &format!("{} hotkeys in use — see log", s.failures.len()),
                        LogClass::Accent,
                    );
                }
                self.system = Some(s);
            }
            Err(e) => self.log(&format!("Tray failed: {}", e), LogClass::Bad),
        }
        self
    }

    fn cfg(&self) -> &AppConfig {
        &self.draft
    }
    fn cfg_mut(&mut self) -> &mut AppConfig {
        &mut self.draft
    }
    /// Token colors for the hand-painted monitor canvas. Reads the live
    /// shadcn theme; never writes to the egui Context.
    fn canvas_theme(&self, ctx: &egui::Context) -> CanvasTheme {
        CanvasTheme::new(ctx, self.cfg())
    }

    fn dirty(&self) -> bool {
        serde_json::to_string(&self.draft).unwrap_or_default() != self.last_saved_json
    }

    fn log(&mut self, msg: &str, class: LogClass) {
        self.console.push(LogLine {
            class,
            text: format!("[{}] {}", clock(), msg),
        });
        if self.console.len() > 200 {
            self.console.remove(0);
        }
    }

    /// Re-apply the in-memory hotkey bindings after a config change (§2c).
    /// `register_from_config` unregisters the old set first, so this is safe to
    /// call repeatedly. Any per-binding failures are surfaced in the console.
    fn reregister_hotkeys(&mut self, cfg: &AppConfig) {
        let Some(sys) = self.system.as_mut() else {
            return;
        };
        let result = sys.register_from_config(cfg);
        let failures = sys.failures.clone();
        if let Err(e) = result {
            self.log(
                &format!("Hotkey re-registration failed: {}", e),
                LogClass::Bad,
            );
        }
        for f in failures {
            self.log(&format!("Hotkey warning: {}", f), LogClass::Accent);
        }
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    /* ------------------------- persistence ------------------------- */

    fn save_apply(&mut self) {
        let cfg = self.cfg().clone();
        match config::save_config(&cfg) {
            Ok(()) => {
                self.last_saved_json = serde_json::to_string(&cfg).unwrap_or_default();
                engine::retile(&cfg);
                self.reregister_hotkeys(&cfg);
                self.log("Saved and applied.", LogClass::Good);
                self.toast("Saved and applied");
            }
            Err(e) => {
                self.log(&format!("Save failed: {}", e), LogClass::Bad);
                self.toast("Failed to save config");
            }
        }
    }

    fn revert(&mut self) {
        self.draft = config::load_config();
        self.last_saved_json = serde_json::to_string(&self.draft).unwrap_or_default();
        self.selected_zone = self.draft.active().zones.first().map(|z| z.id).unwrap_or(0);
        self.fields_zone = -1;
        self.merge_mode = false;
        self.push_history();
        let cfg = self.draft.clone();
        self.reregister_hotkeys(&cfg);
        self.log("Reverted to saved state.", LogClass::Good);
    }

    fn reset_demo(&mut self) {
        self.draft = config::default_config();
        self.selected_zone = 0;
        self.fields_zone = -1;
        self.merge_mode = false;
        self.push_history();
        self.log("Demo reset to empty starter profiles.", LogClass::Good);
        self.toast("Demo reset");
    }

    /* ------------------------- system actions ------------------------- */

    fn handle_action(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::LaunchApp(i) => {
                let cfg = config::load_config();
                if let Some(z) = cfg.active().zones.get(i) {
                    let r = engine::zone_pixels(z, engine::work_area(), cfg.gap);
                    match engine::execute_zone_with_config(z, r, &cfg) {
                        Ok(m) => self.log(&m, LogClass::Good),
                        Err(e) => self.log(&e, LogClass::Bad),
                    }
                }
            }
            Action::Retile => {
                let c = config::load_config();
                engine::retile(&c);
                self.log("Re-tiled.", LogClass::Good);
            }
            Action::CycleNext => {
                let c = config::load_config();
                engine::cycle_zone(&c, true);
            }
            Action::CyclePrev => {
                let c = config::load_config();
                engine::cycle_zone(&c, false);
            }
            Action::CycleAxis(dx, dy) => {
                let c = config::load_config();
                engine::cycle_zone_axis(&c, dx, dy);
            }
            Action::Maximize => engine::maximize_active(),
            Action::Restore => {
                let c = config::load_config();
                engine::restore_active(&c);
            }
            Action::Reload => self.revert(),
            Action::ShowSettings => {
                self.view = View::Settings;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            Action::Exit => {
                self.exiting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn poll_drag(&mut self, ctx: &egui::Context) {
        if engine::drag_keys_held() {
            if self.drag_hwnd.is_none() {
                let (x, y) = engine::cursor_pos();
                if let Some(h) = engine::window_at(x, y) {
                    let cfg = config::load_config();
                    let area = engine::work_area();
                    let zones = engine::compute_zones(cfg.active(), area, cfg.gap);
                    {
                        let mut st = self.overlay.write().unwrap_or_else(|e| e.into_inner());
                        st.active = true;
                        st.zones = zones;
                        st.work = area;
                        st.hover_zone = 0;
                    }
                    self.drag_hwnd = Some(h.0 as isize);
                    ctx.request_repaint();
                }
            }
            let (x, y) = engine::cursor_pos();
            let st = self.overlay.read().unwrap_or_else(|e| e.into_inner());
            let hover = engine::zone_at(&st.zones, x, y).unwrap_or(0);
            drop(st);
            self.overlay
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .hover_zone = hover;
            ctx.request_repaint();
        } else if let Some(raw) = self.drag_hwnd.take() {
            let st = self.overlay.read().unwrap_or_else(|e| e.into_inner());
            let hover = st.hover_zone;
            let zones = st.zones.clone();
            drop(st);
            self.overlay
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .active = false;
            if hover > 0 {
                if let Some((_, r)) = zones.iter().find(|(id, _)| *id == hover) {
                    engine::move_window_raw(raw, *r);
                }
            }
            ctx.request_repaint();
        }
    }

    fn advance_exec(&mut self, ctx: &egui::Context) {
        if self.exec_queue.is_empty() {
            return;
        }
        let now = Instant::now();
        if self.exec_next <= now {
            if let Some(line) = self.exec_queue.pop_front() {
                self.console.push(line);
                if self.console.len() > 200 {
                    self.console.remove(0);
                }
            }
            self.exec_next = now + Duration::from_millis(240);
        }
        let wait = self
            .exec_next
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(250));
        ctx.request_repaint_after(wait);
    }

    /* ------------------------- zone mutations ------------------------- */

    fn next_zone_id(&self) -> i32 {
        self.cfg()
            .active()
            .zones
            .iter()
            .map(|z| z.id)
            .max()
            .unwrap_or(0)
            + 1
    }

    fn blank_zone(&self, id: i32, x: f32, y: f32, w: f32, h: f32) -> Zone {
        Zone::new(id, &format!("Zone {}", id), "", x, y, w, h)
    }

    fn split_zone(&mut self, dir: bool, ratio: f32) {
        // dir = true -> vertical split (left/right); false -> horizontal (top/bottom)
        let id = self.selected_zone;
        let Some(z) = self
            .cfg()
            .active()
            .zones
            .iter()
            .find(|z| z.id == id)
            .cloned()
        else {
            self.toast("Select a zone first");
            return;
        };
        let (size, axis) = if dir { (z.w, "narrow") } else { (z.h, "short") };
        if size < 20.0 {
            self.toast(format!("Too {} to split", axis));
            return;
        }
        let new_id = self.next_zone_id();
        let first = size * ratio;
        let mut nz = self.blank_zone(new_id, z.x, z.y, z.w, z.h);
        if dir {
            if let Some(q) = self
                .cfg_mut()
                .active_mut()
                .zones
                .iter_mut()
                .find(|q| q.id == id)
            {
                q.w = first;
            }
            nz.x = z.x + first;
            nz.w = z.w - first;
        } else {
            if let Some(q) = self
                .cfg_mut()
                .active_mut()
                .zones
                .iter_mut()
                .find(|q| q.id == id)
            {
                q.h = first;
            }
            nz.y = z.y + first;
            nz.h = z.h - first;
        }
        self.cfg_mut().active_mut().zones.push(nz);
        self.selected_zone = new_id;
        self.push_history();
        self.log("Zone split", LogClass::Good);
    }

    fn delete_zone(&mut self) {
        let id = self.selected_zone;
        let zones = &mut self.cfg_mut().active_mut().zones;
        if zones.len() <= 1 {
            self.toast("Cannot delete the only zone");
            return;
        }
        let Some(i) = zones.iter().position(|z| z.id == id) else {
            return;
        };
        let removed = zones.remove(i);
        if let Some(n) = zones.iter_mut().min_by(|a, b| {
            let da = (a.x - (removed.x + removed.w)).abs() + (a.y - removed.y).abs();
            let db = (b.x - (removed.x + removed.w)).abs() + (b.y - removed.y).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            n.w = (n.w + removed.w).min(100.0 - n.x);
            n.h = (n.h + removed.h).min(100.0 - n.y);
        }
        self.selected_zone = zones.first().map(|z| z.id).unwrap_or(0);
        self.fields_zone = -1;
        self.push_history();
        self.log("Zone removed", LogClass::Good);
    }

    fn swap_zones(&mut self, a: i32, b: i32) {
        if a == b {
            return;
        }
        let zones = &mut self.cfg_mut().active_mut().zones;
        let ia = zones.iter().position(|z| z.id == a);
        let ib = zones.iter().position(|z| z.id == b);
        if let (Some(ia), Some(ib)) = (ia, ib) {
            let ga = (zones[ia].x, zones[ia].y, zones[ia].w, zones[ia].h);
            let gb = (zones[ib].x, zones[ib].y, zones[ib].w, zones[ib].h);
            zones[ia].x = gb.0;
            zones[ia].y = gb.1;
            zones[ia].w = gb.2;
            zones[ia].h = gb.3;
            zones[ib].x = ga.0;
            zones[ib].y = ga.1;
            zones[ib].w = ga.2;
            zones[ib].h = ga.3;
            self.push_history();
            self.log("Zones swapped", LogClass::Good);
        }
    }

    fn attach_app(&mut self, zone_id: i32, app: &DragApp) {
        if let Some(z) = self
            .cfg_mut()
            .active_mut()
            .zones
            .iter_mut()
            .find(|z| z.id == zone_id)
        {
            z.label = app.label.clone();
            z.exe = app.exe.clone();
            z.path = app.path.clone();
        }
        self.selected_zone = zone_id;
        self.fields_zone = -1;
        self.push_history();
        self.log(
            &format!("{} -> zone {}", app.label, zone_id),
            LogClass::Good,
        );
        self.toast(format!("{} attached", app.label));
    }

    fn attach_app_to_selected(&mut self, label: String, exe: String, path: String) {
        if self.cfg().active().zones.is_empty() {
            let id = self.next_zone_id();
            let z = self.blank_zone(id, 0.0, 0.0, 100.0, 100.0);
            self.cfg_mut().active_mut().zones.push(z);
            self.selected_zone = id;
        }
        let id = self.selected_zone;
        self.attach_app(id, &DragApp { label, exe, path });
    }

    fn cancel_merge(&mut self) {
        self.merge_mode = false;
        self.merge_source = 0;
    }

    fn begin_merge(&mut self) {
        if self.merge_mode {
            self.cancel_merge();
            self.log("Merge cancelled", LogClass::Accent);
            return;
        }
        if self.cfg().active().zones.len() <= 1 {
            self.toast("Need at least two zones to merge");
            return;
        }
        if self.selected_zone == 0 {
            self.toast("Select a source zone first");
            return;
        }
        self.merge_mode = true;
        self.merge_source = self.selected_zone;
        self.log("Merge mode — choose a target zone", LogClass::Accent);
    }

    fn apply_layout(&mut self, rects: &[RectSpec]) {
        let old = self.cfg().active().zones.clone();
        let mut used = vec![false; old.len()];
        let mut zones: Vec<Zone> = Vec::with_capacity(rects.len());

        for (i, (x, y, w, h)) in rects.iter().enumerate() {
            let new_id = (i + 1) as i32;
            let mut z = self.blank_zone(new_id, *x, *y, *w, *h);
            // Preserve by zone id where possible; fall back to original order
            // for profiles whose ids are not 1..n (e.g. after splits/deletes).
            let mut src = old
                .iter()
                .enumerate()
                .find(|(idx, o)| !used[*idx] && o.id == new_id)
                .map(|(idx, _)| idx);
            if src.is_none() {
                src = old
                    .iter()
                    .enumerate()
                    .find(|(idx, _)| !used[*idx])
                    .map(|(idx, _)| idx);
            }
            if let Some(idx) = src {
                copy_zone_binding(&old[idx], &mut z);
                used[idx] = true;
            }
            zones.push(z);
        }

        // Old zones with no new counterpart would have their bindings silently
        // dropped when a template has fewer zones. Merge them into the first
        // surviving zone so no app binding/rule is lost (§1).
        let leftovers: Vec<&Zone> = old
            .iter()
            .enumerate()
            .filter(|(idx, _)| !used[*idx])
            .map(|(_, o)| o)
            .collect();
        if let Some(first) = zones.first_mut() {
            for src in leftovers {
                merge_zone_binding(first, src);
            }
        } else if rects.is_empty() {
            // The "Empty" template has no surviving zone to merge leftovers into.
            // Rather than silently dropping every app binding/rule, warn loudly so
            // the user knows those bindings were cleared (they can be re-added or
            // restored from history with Ctrl+Z). Documented behaviour, not data
            // loss by accident.
            let bound = old
                .iter()
                .filter(|z| {
                    !z.exe.is_empty()
                        || !z.url.is_empty()
                        || !z.title_match.is_empty()
                        || !z.macro_text.is_empty()
                        || !z.notes.is_empty()
                })
                .count();
            if bound > 0 {
                self.log(
                    &format!(
                        "Empty layout cleared {} zone binding(s) — re-add apps or press Ctrl+Z to undo",
                        bound
                    ),
                    LogClass::Bad,
                );
            }
        }

        self.cfg_mut().active_mut().zones = zones;
        self.selected_zone = if rects.is_empty() { 0 } else { 1 };
        self.fields_zone = -1;
        self.push_history();
        self.log(
            &format!("Layout applied - {} zones", rects.len()),
            LogClass::Good,
        );
    }

    fn nudge_selected(&mut self, dx: f32, dy: f32) {
        let id = self.selected_zone;
        if let Some(z) = self
            .cfg_mut()
            .active_mut()
            .zones
            .iter_mut()
            .find(|z| z.id == id)
        {
            z.x = (z.x + dx).clamp(0.0, 100.0 - z.w);
            z.y = (z.y + dy).clamp(0.0, 100.0 - z.h);
        }
        self.push_history();
    }

    fn set_zone_str(&mut self, id: i32, field: StrField, value: String) {
        if let Some(z) = self
            .cfg_mut()
            .active_mut()
            .zones
            .iter_mut()
            .find(|z| z.id == id)
        {
            match field {
                StrField::Label => z.label = value,
                StrField::Exe => z.exe = value,
                StrField::Path => z.path = value,
                StrField::Url => z.url = value,
                StrField::Title => z.title_match = value,
                StrField::Trigger => z.trigger_title = value,
                StrField::Macro => z.macro_text = value,
            }
        }
    }

    fn sync_buffers(&mut self, zone: &Zone) {
        self.fields_zone = zone.id;
        self.edit_bufs.label = zone.label.clone();
        self.edit_bufs.exe = zone.exe.clone();
        self.edit_bufs.path = zone.path.clone();
        self.edit_bufs.url = zone.url.clone();
        self.edit_bufs.title = zone.title_match.clone();
        self.edit_bufs.trigger = zone.trigger_title.clone();
        self.edit_bufs.macro_text = zone.macro_text.clone();
        self.edit_bufs.delay = zone.key_delay as f32;
    }

    /* ------------------------- execution simulation ------------------------- */

    fn queue_line(&mut self, text: impl Into<String>, class: LogClass) {
        self.exec_queue.push_back(LogLine {
            class,
            text: format!("[{}] {}", clock(), text.into()),
        });
    }

    fn run_sequence(&mut self, kind: &str) {
        if !self.exec_queue.is_empty() {
            return;
        }
        let res = self.res;
        let gap = self.cfg().gap;
        let zones = self.cfg().active().zones.clone();
        let profile = self.cfg().active_profile.clone();
        if kind == "launch" {
            self.queue_line(
                format!("Launching workspace '{}'", profile),
                LogClass::Accent,
            );
            self.queue_line(
                format!("OK Profile loaded - {} zones", zones.len()),
                LogClass::Good,
            );
            for z in &zones {
                self.queue_line(
                    format!(
                        "-> Checking window: {}",
                        if z.exe.is_empty() {
                            "(unassigned)"
                        } else {
                            &z.exe
                        }
                    ),
                    LogClass::Normal,
                );
                let pw = ((z.w / 100.0 * res.width() as f32) - gap as f32)
                    .max(0.0)
                    .round() as i32;
                let ph = ((z.h / 100.0 * res.work_height() as f32) - gap as f32)
                    .max(0.0)
                    .round() as i32;
                self.queue_line(
                    format!(
                        "-> Applying bounds: {}x{} @ {},{}",
                        pw,
                        ph,
                        (z.x / 100.0 * res.width() as f32).round() as i32,
                        (z.y / 100.0 * res.work_height() as f32).round() as i32
                    ),
                    LogClass::Normal,
                );
                if !z.url.is_empty() {
                    self.queue_line(format!("-> Opening URL: {}", z.url), LogClass::Accent);
                }
                if !z.macro_text.is_empty() {
                    self.queue_line(format!("-> Macro: {}", z.macro_text), LogClass::Accent);
                }
                self.queue_line(
                    format!(
                        "OK {} tiled",
                        if z.label.is_empty() {
                            format!("Zone {}", z.id)
                        } else {
                            z.label.clone()
                        }
                    ),
                    LogClass::Good,
                );
            }
            self.queue_line(
                format!("OK Done - {} windows positioned", zones.len()),
                LogClass::Good,
            );
        } else {
            let Some(z) = zones.iter().find(|z| z.id == self.selected_zone) else {
                self.toast("Select a zone first");
                return;
            };
            self.queue_line(
                format!("Testing zone {} - {}", z.id, z.label),
                LogClass::Accent,
            );
            self.queue_line(
                format!(
                    "-> Checking window: {}",
                    if z.exe.is_empty() {
                        "(unassigned)"
                    } else {
                        &z.exe
                    }
                ),
                LogClass::Normal,
            );
            let pw = ((z.w / 100.0 * res.width() as f32) - gap as f32)
                .max(0.0)
                .round() as i32;
            let ph = ((z.h / 100.0 * res.work_height() as f32) - gap as f32)
                .max(0.0)
                .round() as i32;
            self.queue_line(
                format!("-> Applying bounds: {}x{}", pw, ph),
                LogClass::Normal,
            );
            if !z.url.is_empty() {
                self.queue_line(
                    format!("-> Focusing tab / URL: {}", z.url),
                    LogClass::Accent,
                );
            }
            if !z.macro_text.is_empty() {
                self.queue_line(
                    format!("-> Sending macro: {}", z.macro_text),
                    LogClass::Accent,
                );
            }
            self.queue_line("OK Zone test complete", LogClass::Good);
        }
        self.exec_next = Instant::now();
    }
}

fn clock() -> String {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{:02}:{:02}:{:02}", (s / 3600) % 24, (s / 60) % 60, s % 60)
}

/* ============================================================================
 * UI
 * ========================================================================== */

use shadcn_egui::Theme;

impl Editor {
    /* ------------------------- shell ------------------------- */

    fn body(&mut self, ui: &mut egui::Ui) {
        let p = *Theme::current(ui.ctx()).palette();
        let left_frame = egui::Frame::new()
            .fill(p.card)
            .stroke(egui::Stroke::new(1.0, p.border))
            .inner_margin(egui::Margin::symmetric(
                MARGIN_PANEL_LEFT.0,
                MARGIN_PANEL_LEFT.1,
            ));
        egui::Panel::left("zf_left")
            .default_size(PANEL_LEFT_W)
            .min_size(PANEL_LEFT_MIN_W)
            .max_size(PANEL_LEFT_MAX_W)
            .frame(left_frame)
            .show(ui, |ui| self.left_panel(ui));

        if self.view == View::Canvas {
            let right_frame = egui::Frame::new()
                .fill(p.card)
                .stroke(egui::Stroke::new(1.0, p.border))
                .inner_margin(egui::Margin::symmetric(
                    MARGIN_PANEL_RIGHT.0,
                    MARGIN_PANEL_RIGHT.1,
                ));
            egui::Panel::right("zf_right")
                .default_size(PANEL_RIGHT_W)
                .min_size(PANEL_RIGHT_MIN_W)
                .max_size(PANEL_RIGHT_MAX_W)
                .frame(right_frame)
                .show(ui, |ui| self.right_panel(ui));
        }

        // CentralPanel must always be added LAST (§7).
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.background))
            .show(ui, |ui| self.center(ui));
    }

    fn center(&mut self, ui: &mut egui::Ui) {
        match self.view {
            View::Canvas => {
                self.toolbar(ui);
                ui.add_space(SPACE_SM);
                self.canvas_view(ui);
                ui.add_space(SPACE_XS);
                self.action_bar(ui);
                self.apps_strip(ui);
            }
            View::Hotkeys => self.hotkeys_view(ui),
            View::Settings => self.settings_view(ui),
        }
    }

    fn duplicate_profile(&mut self) {
        let base = self.cfg().active_profile.clone();
        let mut n = format!("{} Copy", base);
        let mut i = 2;
        while self.cfg().profiles.contains_key(&n) {
            n = format!("{} Copy {}", base, i);
            i += 1;
        }
        let p = self.cfg().active().clone();
        self.cfg_mut().profiles.insert(n.clone(), p);
        self.cfg_mut().active_profile = n.clone();
        self.selected_zone = self.cfg().active().zones.first().map(|z| z.id).unwrap_or(0);
        self.fields_zone = -1;
        self.push_history();
        self.log(&format!("Profile duplicated -> {}", n), LogClass::Good);
    }

    fn delete_profile(&mut self) {
        if self.cfg().profiles.len() <= 1 {
            self.toast("Keep at least one profile");
            return;
        }
        let name = self.cfg().active_profile.clone();
        self.cfg_mut().profiles.remove(&name);
        self.cfg_mut().active_profile = self
            .cfg()
            .profiles
            .keys()
            .next()
            .cloned()
            .unwrap_or_default();
        self.selected_zone = self.cfg().active().zones.first().map(|z| z.id).unwrap_or(0);
        self.fields_zone = -1;
        self.push_history();
        self.log(&format!("Profile deleted: {}", name), LogClass::Good);
    }

    /* ------------------------- draw mode ------------------------- */

    fn toggle_draw(&mut self) {
        self.draw_active = !self.draw_active;
        self.draw_start = None;
        self.draw_current = None;
        self.cancel_merge();
        self.log(
            if self.draw_active {
                "Draw mode on"
            } else {
                "Draw mode off"
            },
            LogClass::Accent,
        );
    }

    fn begin_rename_selected(&mut self) {
        let id = self.selected_zone;
        if let Some(z) = self.cfg().active().zones.iter().find(|z| z.id == id) {
            self.text_dialog = Some(TextDialogState {
                kind: TextDialogKind::RenameZone(id),
                value: z.label.clone(),
            });
        }
    }

    /* ------------------------- shortcuts ------------------------- */

    fn handle_shortcuts(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let typing = ctx.egui_wants_keyboard_input();
        let dialog_open = self.wizard_open
            || self.picker_open
            || self.help_open
            || self.reset_confirm_open
            || self.text_dialog.is_some()
            || self.close_dialog;

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.wizard_open {
                self.wizard_open = false;
            } else if self.picker_open {
                self.picker_open = false;
            } else if self.help_open {
                self.help_open = false;
            } else if self.reset_confirm_open {
                self.reset_confirm_open = false;
            } else if self.text_dialog.is_some() {
                self.text_dialog = None;
            } else if self.merge_mode {
                self.cancel_merge();
                self.log("Merge cancelled", LogClass::Accent);
            } else if !typing && self.view == View::Canvas {
                self.selected_zone = 0;
                self.fields_zone = -1;
            }
            return;
        }
        if typing || dialog_open {
            return;
        }

        let undo = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z);
        let redo1 = egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::Z,
        );
        let redo2 = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Y);
        if ctx.input_mut(|i| i.consume_shortcut(&redo1) || i.consume_shortcut(&redo2)) {
            self.redo();
        } else if ctx.input_mut(|i| i.consume_shortcut(&undo)) {
            self.undo();
        }

        if self.view != View::Canvas {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F2)) {
            self.begin_rename_selected();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            self.delete_zone();
        }
        // Arrow-key nudge step is config-driven (§2.2 resize_step_pct); Shift
        // still gives a 5x coarse step. Default 1.0 preserves prior behaviour.
        let base_step = self.cfg().settings.resize_step_pct;
        let step = if ctx.input(|i| i.modifiers.shift) {
            base_step * 5.0
        } else {
            base_step
        };
        let mut dx = 0.0;
        let mut dy = 0.0;
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            dx -= step;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            dx += step;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            dy -= step;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            dy += step;
        }
        if dx != 0.0 || dy != 0.0 {
            self.nudge_selected(dx, dy);
        }
    }

    /* ------------------------- startup / display parity ------------------------- */

    /// AHK `AutoLaunchAndSnapAll` parity (§2c / T1). Fires once, off the UI
    /// thread: `execute_zone` can wait up to 8 s for a window, so the work runs
    /// on a worker and its results are drained back here each frame.
    fn maybe_auto_launch(&mut self) {
        if !self.auto_launched && self.cfg().settings.auto_launch_on_startup {
            self.auto_launched = true;
            let cfg = self.cfg().clone();
            let area = engine::work_area();
            let jobs: Vec<(Zone, engine::Rect)> = cfg
                .active()
                .zones
                .iter()
                .filter(|z| !z.exe.is_empty())
                .map(|z| (z.clone(), engine::zone_pixels(z, area, cfg.gap)))
                .collect();
            if jobs.is_empty() {
                self.log(
                    "Auto-launch is enabled but no zones have an app assigned.",
                    LogClass::Accent,
                );
            } else {
                self.log(
                    &format!("Auto-launching {} app(s) on startup...", jobs.len()),
                    LogClass::Accent,
                );
                let (tx, rx) = std::sync::mpsc::channel();
                self.auto_launch_rx = Some(rx);
                std::thread::spawn(move || {
                    for (zone, rect) in jobs {
                        let label = if zone.label.is_empty() {
                            zone.exe.clone()
                        } else {
                            zone.label.clone()
                        };
                        let result = engine::execute_zone_with_config(&zone, rect, &cfg)
                            .map_err(|e| format!("{}: {}", label, e));
                        // A dropped receiver means the app is shutting down.
                        if tx.send(result).is_err() {
                            break;
                        }
                    }
                });
            }
        }

        // Drain worker results without holding a borrow of `self` across `log`.
        let results: Vec<Result<String, String>> = self
            .auto_launch_rx
            .as_ref()
            .map(|rx| {
                let mut v = Vec::new();
                while let Ok(r) = rx.try_recv() {
                    v.push(r);
                }
                v
            })
            .unwrap_or_default();
        for result in results {
            match result {
                Ok(msg) => {
                    tracing::info!(%msg, "startup auto-launch succeeded");
                    self.log(&msg, LogClass::Good);
                }
                Err(msg) => {
                    tracing::warn!(%msg, "startup auto-launch failed");
                    self.log(&msg, LogClass::Bad);
                }
            }
        }
    }

    /// AHK `WM_DISPLAYCHANGE` / `WM_SETTINGCHANGE` parity (§2c / T2) via polling
    /// in the egui frame loop. The first check only records a baseline so a
    /// re-tile never fires on startup.
    fn poll_display_change(&mut self) {
        let debounce = Duration::from_millis(self.cfg().settings.display_debounce_ms);
        if self.last_work_area_check.elapsed() < debounce {
            return;
        }
        self.last_work_area_check = Instant::now();
        let area = engine::work_area();
        match self.last_work_area {
            None => {
                // First check: establish the baseline, do not re-tile.
                self.last_work_area = Some(area);
            }
            Some(prev) if prev != area => {
                self.last_work_area = Some(area);
                let cfg = self.cfg().clone();
                engine::retile(&cfg);
                tracing::info!(?prev, ?area, "display work area changed; re-tiled");
                self.log("Display changed - re-tiled", LogClass::Good);
            }
            Some(_) => {
                self.last_work_area = Some(area);
            }
        }
    }
}

impl eframe::App for Editor {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Graceful shutdown: persist any unsaved draft so nothing is lost on exit.
        if self.dirty() {
            if let Err(e) = config::save_config(self.cfg()) {
                tracing::warn!(error = %e, "Failed to save config on exit");
            } else {
                self.last_saved_json = serde_json::to_string(self.cfg()).unwrap_or_default();
            }
        }
        tracing::info!("ZoneFlow Studio shutting down");
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(sys) = &self.system {
            for a in sys.poll() {
                self.handle_action(ctx, a);
            }
        }
        self.poll_drag(ctx);
        self.advance_exec(ctx);
        self.maybe_auto_launch();
        self.poll_display_change();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // The shadcn theme is applied once in main.rs via apply_with_fonts.
        // Do NOT override it here (no set_visuals / set_fonts / apply_theme).

        if ctx.input(|i| i.viewport().close_requested()) && !self.exiting {
            match self.cfg().close_action {
                CloseAction::Ask => {
                    self.close_dialog = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                }
                CloseAction::Minimize => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                }
                CloseAction::Exit => {
                    // Remembered "exit completely" - allow the close and mark for cleanup.
                    self.exiting = true;
                }
            }
        }

        self.handle_shortcuts(ui);
        self.top_bar(ui);
        ui.add_space(SPACE_SM);
        self.body(ui);

        self.wizard_modal(&ctx);
        self.picker_modal(&ctx);
        self.help_modal(&ctx);
        self.reset_confirm_modal(&ctx);
        self.text_dialog_modal(&ctx);
        self.close_dialog_ui(&ctx);
        self.toast_ui(&ctx);
        let cfg = self.cfg();
        overlay::show_with_theme(
            &ctx,
            &self.overlay,
            &cfg.settings.zone_tints,
            cfg.settings.overlay_inactive_alpha,
            cfg.settings.overlay_active_alpha,
        );
    }
}
