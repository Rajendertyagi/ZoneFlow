//! Monitor canvas UI: rendering and interaction for the hand-painted stage.

use crate::config::Zone;
use crate::engine_core;
use eframe::egui;
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::separator::Separator;
use shadcn_egui::icons::lucide;
use shadcn_egui::layout;

use super::state::{Boundary, BoundaryDrag, DragApp, DragZoneApp, LogClass, ZoneDrag};
use super::tokens::*;
use super::Editor;

/// Blend two opaque colors. Used for the subtle selected-zone surface wash.
fn mix_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    egui::Color32::from_rgb(
        (a.r() as f32 * (1.0 - t) + b.r() as f32 * t).round() as u8,
        (a.g() as f32 * (1.0 - t) + b.g() as f32 * t).round() as u8,
        (a.b() as f32 * (1.0 - t) + b.b() as f32 * t).round() as u8,
    )
}

impl Editor {
    /* ------------------------- canvas ------------------------- */

    pub(super) fn canvas_view(&mut self, ui: &mut egui::Ui) {
        let taskbar_h = self.cfg().settings.taskbar_h;
        let profile = self.cfg().active_profile.clone();
        let nz = self.cfg().active().zones.len();
        layout::row(ui, SPACE_8, |ui| {
            ui_label(ui, &profile);
            ui.add(Badge::new(format!("{} zones", nz)).variant(BadgeVariant::Secondary));
        });

        let avail = ui.available_size();
        // Fill the stage: reserve space for the hint line, action bar and the
        // fixed-height assigned-apps strip so all three stay visible.
        let w = (avail.x - 20.0).max(160.0);
        let h = (avail.y - self.cfg().settings.stage_lower_h).max(140.0);
        let (screen, response) =
            ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click_and_drag());

        // Draw modern monitor shell background & border
        let p = *shadcn_egui::Theme::current(ui.ctx()).palette();
        let painter = ui.painter();
        painter.rect_filled(screen, 12.0, p.card);
        painter.rect_stroke(
            screen,
            12.0,
            egui::Stroke::new(1.5, p.border),
            egui::StrokeKind::Inside,
        );

        // Zones live above the taskbar strip; pointer math uses the same rect.
        let zone_area = egui::Rect::from_min_max(
            screen.min + egui::vec2(6.0, 6.0),
            egui::pos2(screen.max.x - 6.0, screen.max.y - taskbar_h - 4.0),
        );

        self.canvas_interact(ui, zone_area, &response);
        self.canvas_paint(ui, screen, zone_area, &response);
    }

    fn canvas_interact(
        &mut self,
        ui: &mut egui::Ui,
        screen: egui::Rect,
        response: &egui::Response,
    ) {
        let ctx = ui.ctx();
        let pointer = Self::pointer_percent(ctx, screen);

        // Drag & drop from library / apps strip
        if let Some(app) = response.dnd_release_payload::<DragApp>() {
            if let Some(p) = pointer {
                if let Some(id) = self.zone_at_percent(p) {
                    self.attach_app(id, &app);
                }
            }
        }
        if let Some(za) = response.dnd_release_payload::<DragZoneApp>() {
            if let Some(p) = pointer {
                if let Some(id) = self.zone_at_percent(p) {
                    self.swap_zones(za.id, id);
                }
            }
        }
        self.drop_target = None;
        if response.dnd_hover_payload::<DragApp>().is_some()
            || response.dnd_hover_payload::<DragZoneApp>().is_some()
        {
            self.drop_target = pointer.and_then(|p| self.zone_at_percent(p));
        }

        // Right-click selects the zone under the cursor; context menu is attached below.
        if response.secondary_clicked() {
            if let Some(p) = pointer {
                if let Some(id) = self.zone_at_percent(p) {
                    self.selected_zone = id;
                    self.fields_zone = -1;
                }
            }
        }

        if response.drag_started() {
            if let Some(p) = pointer {
                if self.draw_active {
                    self.draw_start = Some(p);
                    self.draw_current = Some(p);
                } else if !self.merge_mode {
                    if let Some(b) = self.boundary_at(p, screen) {
                        let axis = if b.vertical { p.x } else { p.y };
                        let coord_idx = if b.vertical { 0 } else { 1 };
                        let size_idx = if b.vertical { 2 } else { 3 };
                        let zones = &self.cfg().active().zones;
                        let collect = |ids: &Vec<i32>| -> Vec<(i32, f32, f32)> {
                            ids.iter()
                                .filter_map(|id| zones.iter().find(|z| z.id == *id))
                                .map(|z| {
                                    let vals = [z.x, z.y, z.w, z.h];
                                    (z.id, vals[coord_idx], vals[size_idx])
                                })
                                .collect()
                        };
                        let start_pos = b.pos;
                        self.boundary_drag = Some(BoundaryDrag {
                            before_orig: collect(&b.before),
                            after_orig: collect(&b.after),
                            boundary: b,
                            start_pointer: axis,
                            start_pos,
                        });
                    } else if let Some(id) = self.zone_at_percent(p) {
                        if let Some(z) = self.cfg().active().zones.iter().find(|z| z.id == id) {
                            self.zone_drag = Some(ZoneDrag {
                                id,
                                grab: egui::vec2(p.x - z.x, p.y - z.y),
                                orig: (z.x, z.y, z.w, z.h),
                            });
                            self.selected_zone = id;
                            self.fields_zone = -1;
                        }
                    }
                }
            }
        }

        if response.dragged() {
            if let Some(p) = pointer {
                if self.draw_active && self.draw_start.is_some() {
                    self.draw_current = Some(p);
                } else if let Some(drag) = self.boundary_drag.take() {
                    self.update_boundary_drag(&drag, p, screen);
                    self.boundary_drag = Some(drag);
                } else if let Some(drag) = &self.zone_drag {
                    let id = drag.id;
                    let grab = drag.grab;
                    if let Some(z) = self
                        .cfg_mut()
                        .active_mut()
                        .zones
                        .iter_mut()
                        .find(|z| z.id == id)
                    {
                        z.x = (p.x - grab.x).clamp(0.0, 100.0 - z.w);
                        z.y = (p.y - grab.y).clamp(0.0, 100.0 - z.h);
                    }
                }
            }
        }

        if response.drag_stopped() {
            if let Some(drag) = self.zone_drag.take() {
                // If released over a different zone, swap geometries.
                // Exclude the dragged zone itself - it follows the cursor, so it
                // always contains the pointer at drop time.
                if let Some(p) = pointer {
                    let target = self
                        .cfg()
                        .active()
                        .zones
                        .iter()
                        .find(|z| {
                            z.id != drag.id
                                && p.x >= z.x
                                && p.x <= z.x + z.w
                                && p.y >= z.y
                                && p.y <= z.y + z.h
                        })
                        .map(|z| z.id);
                    if let Some(target) = target {
                        let orig = drag.orig;
                        let zones = &mut self.cfg_mut().active_mut().zones;
                        let tgt = zones.iter().find(|z| z.id == target).cloned();
                        if let Some(tgt) = tgt {
                            if let Some(s) = zones.iter_mut().find(|z| z.id == drag.id) {
                                s.x = tgt.x;
                                s.y = tgt.y;
                                s.w = tgt.w;
                                s.h = tgt.h;
                            }
                            if let Some(tz) = zones.iter_mut().find(|z| z.id == target) {
                                tz.x = orig.0;
                                tz.y = orig.1;
                                tz.w = orig.2;
                                tz.h = orig.3;
                            }
                        }
                    }
                }
                self.push_history();
                self.log("Zone moved / swapped", LogClass::Good);
            }
            if self.boundary_drag.take().is_some() {
                self.hud = None;
                self.snap_guide = None;
                self.push_history();
                self.log("Zone dimensions updated", LogClass::Good);
            }
            if let (Some(start), Some(cur)) = (self.draw_start.take(), self.draw_current.take()) {
                self.finish_draw(start, cur);
            }
        }

        if response.clicked() {
            if let Some(p) = pointer {
                if self.merge_mode {
                    if let Some(id) = self.zone_at_percent(p) {
                        if id != self.merge_source {
                            self.do_merge(id);
                        }
                    } else {
                        self.cancel_merge();
                        self.log("Merge cancelled", LogClass::Accent);
                    }
                } else if let Some(id) = self.zone_at_percent(p) {
                    self.selected_zone = id;
                    self.fields_zone = -1;
                } else {
                    self.selected_zone = 0;
                }
            }
        }

        // Context menu (clamped by egui automatically)
        let ctx_zone = pointer.and_then(|p| self.zone_at_percent(p));
        response.context_menu(|ui| {
            self.context_menu(ui, ctx_zone);
        });
    }

    fn update_boundary_drag(&mut self, drag: &BoundaryDrag, p: egui::Vec2, screen: egui::Rect) {
        let b = &drag.boundary;
        let axis_now = if b.vertical { p.x } else { p.y };
        let delta = axis_now - drag.start_pointer;
        let min_size = self.cfg().settings.min_zone_pct;
        let coord_idx = if b.vertical { 0 } else { 1 };
        let size_idx = if b.vertical { 2 } else { 3 };

        let mn = drag
            .before_orig
            .iter()
            .map(|(_, c, _)| c + min_size)
            .fold(f32::MIN, f32::max);
        let mx = drag
            .after_orig
            .iter()
            .map(|(_, c, s)| c + s - min_size)
            .fold(f32::MAX, f32::min);
        let mut np = (drag.start_pos + delta).clamp(mn, mx);

        let mut snapped = None;
        for g in self.cfg().settings.snap_points.iter() {
            if (np - g).abs() <= 2.5 && *g >= mn && *g <= mx {
                np = *g;
                snapped = Some(*g);
                break;
            }
        }
        self.snap_guide = snapped;
        self.snap_guide_vertical = b.vertical;

        let zones = &mut self.cfg_mut().active_mut().zones;
        for (id, c, _) in &drag.before_orig {
            if let Some(z) = zones.iter_mut().find(|z| z.id == *id) {
                let mut vals = [z.x, z.y, z.w, z.h];
                vals[size_idx] = np - c;
                z.x = vals[0];
                z.y = vals[1];
                z.w = vals[2];
                z.h = vals[3];
            }
        }
        for (id, c, s) in &drag.after_orig {
            if let Some(z) = zones.iter_mut().find(|z| z.id == *id) {
                let edge = c + s;
                let mut vals = [z.x, z.y, z.w, z.h];
                vals[coord_idx] = np;
                vals[size_idx] = edge - np;
                z.x = vals[0];
                z.y = vals[1];
                z.w = vals[2];
                z.h = vals[3];
            }
        }

        let res = self.res;
        let px = (np / 100.0 * res.width() as f32).round() as i32;
        let rest = ((100.0 - np) / 100.0 * res.width() as f32).round() as i32;
        let axis_name = if b.vertical { "X" } else { "Y" };
        self.hud = Some(match snapped {
            Some(g) => format!("SNAP {}% - {}px | {}px", g.round() as i32, px, rest),
            None => format!(
                "Split {}: {}% ({}px | {}px)",
                axis_name,
                np.round() as i32,
                px,
                rest
            ),
        });
        let _ = screen;
    }

    fn finish_draw(&mut self, start: egui::Vec2, cur: egui::Vec2) {
        let cols = self.cfg().settings.grid_cols.max(1) as f32;
        let rows = self.cfg().settings.grid_rows.max(1) as f32;
        let (x, y, w, h) = if self.snap_draw {
            let c1 = (start.x / 100.0 * cols).floor().clamp(0.0, cols - 1.0);
            let c2 = (cur.x / 100.0 * cols).floor().clamp(0.0, cols - 1.0);
            let r1 = (start.y / 100.0 * rows).floor().clamp(0.0, rows - 1.0);
            let r2 = (cur.y / 100.0 * rows).floor().clamp(0.0, rows - 1.0);
            let cx = c1.min(c2);
            let cy = r1.min(r2);
            (
                cx / cols * 100.0,
                cy / rows * 100.0,
                (c1.max(c2) - cx + 1.0) / cols * 100.0,
                (r1.max(r2) - cy + 1.0) / rows * 100.0,
            )
        } else {
            (
                start.x.min(cur.x),
                start.y.min(cur.y),
                (start.x - cur.x).abs(),
                (start.y - cur.y).abs(),
            )
        };
        if w > 0.5 && h > 0.5 {
            let id = self.next_zone_id();
            let z = self.blank_zone(id, x, y, w, h);
            self.cfg_mut().active_mut().zones.push(z);
            self.selected_zone = id;
            self.fields_zone = -1;
            self.push_history();
            self.log(
                &format!("Zone drawn - {}% x {}%", w.round() as i32, h.round() as i32),
                LogClass::Good,
            );
        }
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, zone: Option<i32>) {
        let t = self.canvas_theme(ui.ctx());
        if let Some(id) = zone {
            self.selected_zone = id;
            self.fields_zone = -1;
        }
        let has_zone = zone.is_some();
        if ui
            .add(
                Button::new("Add app...")
                    .icon(lucide::PLUS)
                    .variant(ButtonVariant::Ghost),
            )
            .clicked()
        {
            self.picker_open = true;
            ui.close();
        }
        if has_zone
            && ui
                .add(
                    Button::new("Rename zone")
                        .icon(lucide::PENCIL)
                        .variant(ButtonVariant::Ghost),
                )
                .clicked()
        {
            self.begin_rename_selected();
            ui.close();
        }
        ui.add(Separator::new());
        for (label, ratio) in [
            ("Split 50 / 50", 0.5),
            ("Split 1/3 - 2/3", 1.0 / 3.0),
            ("Split 2/3 - 1/3", 2.0 / 3.0),
        ] {
            if has_zone
                && ui
                    .add(Button::new(label).variant(ButtonVariant::Ghost))
                    .clicked()
            {
                self.split_zone(true, ratio);
                ui.close();
            }
        }
        ui.add(Separator::new());
        if has_zone
            && ui
                .add(
                    Button::new("Split right (free)")
                        .icon(lucide::MOVE_HORIZONTAL)
                        .variant(ButtonVariant::Ghost),
                )
                .clicked()
        {
            self.split_zone(true, 0.5);
            ui.close();
        }
        if has_zone
            && ui
                .add(
                    Button::new("Split down (free)")
                        .icon(lucide::MOVE_VERTICAL)
                        .variant(ButtonVariant::Ghost),
                )
                .clicked()
        {
            self.split_zone(false, 0.5);
            ui.close();
        }
        ui.add(Separator::new());
        if has_zone
            && ui
                .add(
                    Button::new("Merge with...")
                        .icon(lucide::COMBINE)
                        .variant(ButtonVariant::Ghost),
                )
                .clicked()
        {
            self.begin_merge();
            ui.close();
        }
        if has_zone
            && ui
                .add(
                    Button::new("Remove zone")
                        .icon(lucide::TRASH)
                        .variant(ButtonVariant::Ghost),
                )
                .clicked()
        {
            self.delete_zone();
            ui.close();
        }
        let _ = t;
    }

    fn canvas_paint(
        &mut self,
        ui: &mut egui::Ui,
        screen: egui::Rect,
        zone_area: egui::Rect,
        response: &egui::Response,
    ) {
        let t = self.canvas_theme(ui.ctx());
        let gap = self.cfg().gap as f32;
        let taskbar_h = self.cfg().settings.taskbar_h;
        let zones = self.cfg().active().zones.clone();
        let painter = ui.painter();

        // Monitor bezel + shadow
        painter.rect_filled(screen.expand(6.0), t.r_xl, t.surface2);
        painter.rect_stroke(
            screen.expand(6.0),
            t.r_xl,
            egui::Stroke::new(1.0, t.line2),
            egui::StrokeKind::Inside,
        );
        painter.rect_filled(screen, t.r_lg, t.bg);

        // Draw-mode grid
        if self.draw_active {
            let cols = self.cfg().settings.grid_cols.max(1) as f32;
            let rows = self.cfg().settings.grid_rows.max(1) as f32;
            for i in 1..cols as i32 {
                let x = zone_area.min.x + i as f32 / cols * zone_area.width();
                painter.line_segment(
                    [
                        egui::pos2(x, zone_area.min.y),
                        egui::pos2(x, zone_area.max.y),
                    ],
                    egui::Stroke::new(1.0, t.accent_bg),
                );
            }
            for i in 1..rows as i32 {
                let y = zone_area.min.y + i as f32 / rows * zone_area.height();
                painter.line_segment(
                    [
                        egui::pos2(zone_area.min.x, y),
                        egui::pos2(zone_area.max.x, y),
                    ],
                    egui::Stroke::new(1.0, t.accent_bg),
                );
            }
        }

        // Snap guides (25/50/75) while resizing
        if self.boundary_drag.is_some() {
            for g in self.cfg().settings.snap_points.clone() {
                let active = self
                    .snap_guide
                    .map(|s| (s - g).abs() < 0.01)
                    .unwrap_or(false);
                let col = if active { t.accent } else { t.accent_bg };
                let w = if active { 2.0 } else { 1.0 };
                let x = zone_area.min.x + g / 100.0 * zone_area.width();
                painter.line_segment(
                    [
                        egui::pos2(x, zone_area.min.y),
                        egui::pos2(x, zone_area.max.y),
                    ],
                    egui::Stroke::new(w, col),
                );
                let y = zone_area.min.y + g / 100.0 * zone_area.height();
                painter.line_segment(
                    [
                        egui::pos2(zone_area.min.x, y),
                        egui::pos2(zone_area.max.x, y),
                    ],
                    egui::Stroke::new(w, col),
                );
            }
        }

        // Zones
        for (idx, z) in zones.iter().enumerate() {
            let r = Self::zone_rect(z, zone_area, gap);
            if r.width() <= 0.0 || r.height() <= 0.0 {
                continue;
            }
            let sel = z.id == self.selected_zone;
            let tint = t.tint(idx);
            let is_drop = self.drop_target == Some(z.id);
            let is_merge_src = self.merge_mode && z.id == self.merge_source;
            let is_merge_tgt = self.merge_mode && z.id != self.merge_source;

            let fill = if is_drop {
                t.accent_bg
            } else if sel {
                mix_color(t.raised, tint, 0.15)
            } else {
                t.raised
            };
            painter.rect_filled(r, t.r_lg, fill);
            let stroke_col = if is_drop || is_merge_src {
                t.accent
            } else if is_merge_tgt {
                t.accent_line
            } else if sel {
                tint
            } else {
                t.line2
            };
            painter.rect_stroke(
                r,
                t.r_lg,
                egui::Stroke::new(
                    if sel || is_drop || is_merge_src {
                        2.5
                    } else {
                        1.0
                    },
                    stroke_col,
                ),
                egui::StrokeKind::Inside,
            );
            // Left accent bar
            let bar = egui::Rect::from_min_max(r.min, egui::pos2(r.min.x + 3.0, r.max.y));
            painter.rect_filled(bar, 3.0, tint);

            if z.exe.is_empty() {
                // Blank zone: modern drop target styling with plus
                let center = r.center();
                let circle_r = 16.0;
                painter.circle_filled(center - egui::vec2(0.0, 8.0), circle_r, t.bg);
                painter.circle_stroke(
                    center - egui::vec2(0.0, 8.0),
                    circle_r,
                    egui::Stroke::new(1.0, t.line),
                );
                painter.text(
                    center - egui::vec2(0.0, 9.0),
                    egui::Align2::CENTER_CENTER,
                    "+",
                    egui::FontId::proportional(18.0),
                    if is_merge_tgt { t.accent2 } else { t.muted },
                );
                painter.text(
                    center + egui::vec2(0.0, 16.0),
                    egui::Align2::CENTER_CENTER,
                    "Drop app or click to assign",
                    egui::FontId::proportional(11.0),
                    if is_merge_tgt { t.accent2 } else { t.muted },
                );
            } else {
                let pad = 12.0;
                let mut cy = r.min.y + pad;
                let badge_size = 20.0;
                let badge = egui::Rect::from_min_size(
                    egui::pos2(r.min.x + pad, cy),
                    egui::vec2(badge_size, badge_size),
                );
                painter.rect_filled(badge, t.r_sm, if sel { tint } else { t.tint_bg(idx) });
                painter.rect_stroke(
                    badge,
                    t.r_sm,
                    egui::Stroke::new(1.0, if sel { tint } else { t.tint_line(idx) }),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    badge.center(),
                    egui::Align2::CENTER_CENTER,
                    z.id.to_string(),
                    egui::FontId::proportional(10.5),
                    if sel { t.bg } else { tint },
                );
                let tx = badge.max.x + 8.0;
                let font_title = egui::FontId::proportional(12.5);
                let font_exe = egui::FontId::proportional(10.5);

                let max_text_w = (r.max.x - pad - tx).max(10.0);
                let mut title_text = z.label.clone();
                let mut exe_text = z.exe.clone();

                // Truncate cleanly if zone width is small
                if painter
                    .layout_no_wrap(title_text.clone(), font_title.clone(), t.ink)
                    .size()
                    .x
                    > max_text_w
                {
                    while !title_text.is_empty()
                        && painter
                            .layout_no_wrap(format!("{}...", title_text), font_title.clone(), t.ink)
                            .size()
                            .x
                            > max_text_w
                    {
                        title_text.pop();
                    }
                    title_text.push_str("...");
                }
                if painter
                    .layout_no_wrap(exe_text.clone(), font_exe.clone(), tint)
                    .size()
                    .x
                    > max_text_w
                {
                    while !exe_text.is_empty()
                        && painter
                            .layout_no_wrap(format!("{}...", exe_text), font_exe.clone(), tint)
                            .size()
                            .x
                            > max_text_w
                    {
                        exe_text.pop();
                    }
                    exe_text.push_str("...");
                }

                painter.text(
                    egui::pos2(tx, cy + 1.0),
                    egui::Align2::LEFT_TOP,
                    title_text,
                    font_title,
                    t.ink,
                );
                cy += 16.0;
                painter.text(
                    egui::pos2(tx, cy + 1.0),
                    egui::Align2::LEFT_TOP,
                    exe_text,
                    font_exe,
                    tint,
                );
                cy += 18.0;
                let pill = egui::Rect::from_min_size(
                    egui::pos2(r.min.x + pad, cy),
                    egui::vec2(46.0, 15.0),
                );
                painter.rect_filled(pill, 4.0, t.good_bg);
                painter.text(
                    pill.center(),
                    egui::Align2::CENTER_CENTER,
                    "Tiled",
                    egui::FontId::proportional(9.0),
                    t.good,
                );
                if !z.url.is_empty() {
                    let ub = egui::Rect::from_min_size(
                        egui::pos2(pill.max.x + 5.0, cy),
                        egui::vec2(30.0, 15.0),
                    );
                    painter.rect_filled(ub, 4.0, t.surface2);
                    painter.text(
                        ub.center(),
                        egui::Align2::CENTER_CENTER,
                        "URL",
                        egui::FontId::proportional(9.0),
                        t.accent,
                    );
                }
                // Footer: % and px - only rendered if zone height has ample clearance
                let fy = r.max.y - pad - 16.0;
                if fy > cy + 22.0 && r.height() >= 90.0 {
                    painter.line_segment(
                        [
                            egui::pos2(r.min.x + pad, fy - 4.0),
                            egui::pos2(r.max.x - pad, fy - 4.0),
                        ],
                        egui::Stroke::new(1.0, t.line),
                    );
                    let pct = format!("{}% x {}%", z.w.round() as i32, z.h.round() as i32);
                    let pw = ((z.w / 100.0 * self.res.width() as f32) - gap)
                        .max(0.0)
                        .round() as i32;
                    let ph = ((z.h / 100.0 * self.res.work_height() as f32) - gap)
                        .max(0.0)
                        .round() as i32;
                    painter.text(
                        egui::pos2(r.min.x + pad, fy),
                        egui::Align2::LEFT_TOP,
                        pct,
                        egui::FontId::proportional(9.5),
                        t.muted,
                    );
                    painter.text(
                        egui::pos2(r.max.x - pad, fy),
                        egui::Align2::RIGHT_TOP,
                        format!("{}x{} px", pw, ph),
                        egui::FontId::proportional(9.5),
                        tint,
                    );
                }
            }

            if is_merge_tgt {
                painter.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "MERGE HERE",
                    egui::FontId::proportional(10.0),
                    t.accent2,
                );
            }
            if is_drop {
                painter.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    "DROP TO ATTACH",
                    egui::FontId::proportional(10.0),
                    t.accent2,
                );
            }
        }

        // Resize handles
        if !self.draw_active && !self.merge_mode {
            for b in self.find_boundaries() {
                let (hx, hy, hw, hh) = if b.vertical {
                    (
                        zone_area.min.x + b.pos / 100.0 * zone_area.width(),
                        zone_area.min.y + b.span_min / 100.0 * zone_area.height(),
                        14.0,
                        (b.span_max - b.span_min) / 100.0 * zone_area.height(),
                    )
                } else {
                    (
                        zone_area.min.x + b.span_min / 100.0 * zone_area.width(),
                        zone_area.min.y + b.pos / 100.0 * zone_area.height(),
                        (b.span_max - b.span_min) / 100.0 * zone_area.width(),
                        14.0,
                    )
                };
                let hr =
                    egui::Rect::from_min_size(egui::pos2(hx - 7.0, hy - 7.0), egui::vec2(hw, hh));
                let hovered = response
                    .hover_pos()
                    .map(|p| hr.contains(p))
                    .unwrap_or(false);
                let col = if hovered { t.accent } else { t.line2 };
                if b.vertical {
                    let cx = hx;
                    let cy = hy + hh * 0.5;
                    painter.line_segment(
                        [egui::pos2(cx, cy - 13.0), egui::pos2(cx, cy + 13.0)],
                        egui::Stroke::new(if hovered { 4.0 } else { 3.0 }, col),
                    );
                } else {
                    let cx = hx + hw * 0.5;
                    let cy = hy;
                    painter.line_segment(
                        [egui::pos2(cx - 13.0, cy), egui::pos2(cx + 13.0, cy)],
                        egui::Stroke::new(if hovered { 4.0 } else { 3.0 }, col),
                    );
                }
            }
        }

        // Draw preview
        if let (Some(start), Some(cur)) = (self.draw_start, self.draw_current) {
            let (x, y, w, h) = if self.snap_draw {
                let cols = self.cfg().settings.grid_cols.max(1) as f32;
                let rows = self.cfg().settings.grid_rows.max(1) as f32;
                let c1 = (start.x / 100.0 * cols).floor().clamp(0.0, cols - 1.0);
                let c2 = (cur.x / 100.0 * cols).floor().clamp(0.0, cols - 1.0);
                let r1 = (start.y / 100.0 * rows).floor().clamp(0.0, rows - 1.0);
                let r2 = (cur.y / 100.0 * rows).floor().clamp(0.0, rows - 1.0);
                (
                    c1.min(c2) / cols * 100.0,
                    r1.min(r2) / rows * 100.0,
                    (c1.max(c2) - c1.min(c2) + 1.0) / cols * 100.0,
                    (r1.max(r2) - r1.min(r2) + 1.0) / rows * 100.0,
                )
            } else {
                (
                    start.x.min(cur.x),
                    start.y.min(cur.y),
                    (start.x - cur.x).abs(),
                    (start.y - cur.y).abs(),
                )
            };
            let pr = egui::Rect::from_min_max(
                zone_area.min
                    + egui::vec2(
                        x / 100.0 * zone_area.width(),
                        y / 100.0 * zone_area.height(),
                    ),
                zone_area.min
                    + egui::vec2(
                        (x + w) / 100.0 * zone_area.width(),
                        (y + h) / 100.0 * zone_area.height(),
                    ),
            );
            painter.rect_filled(pr, t.r_lg, t.accent_bg);
            painter.rect_stroke(
                pr,
                t.r_lg,
                egui::Stroke::new(2.0, t.accent),
                egui::StrokeKind::Inside,
            );
        }

        // HUD
        if let Some(hud) = &self.hud {
            let hp = egui::pos2(zone_area.center().x, zone_area.min.y + 10.0);
            let galley = painter.layout_no_wrap(
                hud.to_string(),
                egui::FontId::proportional(t.f_small),
                t.accent2,
            );
            let hr = egui::Rect::from_center_size(hp, galley.size() + egui::vec2(20.0, 12.0));
            painter.rect_filled(hr, t.r_md, t.raised);
            painter.rect_stroke(
                hr,
                t.r_md,
                egui::Stroke::new(1.0, t.accent),
                egui::StrokeKind::Inside,
            );
            painter.galley(hr.min + egui::vec2(10.0, 6.0), galley, t.accent2);
        }

        // Taskbar strip - sits below the zone area, never overlapping zone footers.
        let tb = egui::Rect::from_min_max(
            egui::pos2(screen.min.x + 6.0, screen.max.y - taskbar_h - 4.0),
            egui::pos2(screen.max.x - 6.0, screen.max.y - 4.0),
        );
        painter.rect_filled(tb, 4.0, t.bg);
        painter.line_segment(
            [
                egui::pos2(tb.min.x, tb.min.y),
                egui::pos2(tb.max.x, tb.min.y),
            ],
            egui::Stroke::new(1.0, t.line),
        );
        let icon = egui::Rect::from_center_size(
            egui::pos2(screen.min.x + 12.0, tb.center().y),
            egui::vec2(8.0, 8.0),
        );
        painter.rect_filled(icon, 2.0, t.accent);
        for i in 1..4 {
            let d = egui::Rect::from_center_size(
                egui::pos2(screen.min.x + 28.0 + i as f32 * 14.0, tb.center().y),
                egui::vec2(8.0, 8.0),
            );
            painter.rect_filled(d, 2.0, t.surface2);
        }
    }

    /* ------------------------- geometry helpers ------------------------- */

    fn zone_rect(z: &Zone, screen: egui::Rect, gap: f32) -> egui::Rect {
        let r = egui::Rect::from_min_max(
            screen.min + egui::vec2(z.x / 100.0 * screen.width(), z.y / 100.0 * screen.height()),
            screen.min
                + egui::vec2(
                    (z.x + z.w) / 100.0 * screen.width(),
                    (z.y + z.h) / 100.0 * screen.height(),
                ),
        );
        let g = gap * 0.5;
        egui::Rect::from_min_max(r.min + egui::vec2(g, g), r.max - egui::vec2(g, g))
    }

    fn pointer_percent(ctx: &egui::Context, screen: egui::Rect) -> Option<egui::Vec2> {
        ctx.input(|i| i.pointer.latest_pos()).map(|p| {
            egui::vec2(
                (p.x - screen.min.x) / screen.width() * 100.0,
                (p.y - screen.min.y) / screen.height() * 100.0,
            )
        })
    }

    fn zone_at_percent(&self, p: egui::Vec2) -> Option<i32> {
        self.cfg()
            .active()
            .zones
            .iter()
            .find(|z| p.x >= z.x && p.x <= z.x + z.w && p.y >= z.y && p.y <= z.y + z.h)
            .map(|z| z.id)
    }

    fn find_boundaries(&self) -> Vec<Boundary> {
        let zones = &self.cfg().active().zones;
        let tol = self.cfg().settings.boundary_tolerance_pct;
        // Use the tested, platform-independent detector (§2b / issue 14) instead
        // of a local reimplementation. The core API returns position + span;
        // the divider-resize drag state also needs the touching zone ids, so we
        // recompute those from the same zones and tolerance.
        engine_core::find_shared_boundaries(zones, tol)
            .into_iter()
            .map(|sb| {
                let (vertical, pos, span_min, span_max) = match sb {
                    engine_core::SharedBoundary::Vertical { at, start, end } => {
                        (true, at, start, end)
                    }
                    engine_core::SharedBoundary::Horizontal { at, start, end } => {
                        (false, at, start, end)
                    }
                };
                let (before, after) = if vertical {
                    (
                        zones
                            .iter()
                            .filter(|z| (z.x + z.w - pos).abs() <= tol)
                            .map(|z| z.id)
                            .collect(),
                        zones
                            .iter()
                            .filter(|z| (z.x - pos).abs() <= tol)
                            .map(|z| z.id)
                            .collect(),
                    )
                } else {
                    (
                        zones
                            .iter()
                            .filter(|z| (z.y + z.h - pos).abs() <= tol)
                            .map(|z| z.id)
                            .collect(),
                        zones
                            .iter()
                            .filter(|z| (z.y - pos).abs() <= tol)
                            .map(|z| z.id)
                            .collect(),
                    )
                };
                Boundary {
                    vertical,
                    pos,
                    span_min,
                    span_max,
                    before,
                    after,
                }
            })
            .collect()
    }

    fn boundary_at(&self, p: egui::Vec2, screen: egui::Rect) -> Option<Boundary> {
        let tol_v = 9.0 / screen.width() * 100.0;
        let tol_h = 9.0 / screen.height() * 100.0;
        for b in self.find_boundaries() {
            if b.vertical {
                if (p.x - b.pos).abs() <= tol_v && p.y >= b.span_min && p.y <= b.span_max {
                    return Some(b);
                }
            } else if (p.y - b.pos).abs() <= tol_h && p.x >= b.span_min && p.x <= b.span_max {
                return Some(b);
            }
        }
        None
    }

    /* ------------------------- merge ------------------------- */

    fn do_merge(&mut self, target_id: i32) {
        let src = self.merge_source;
        if src == target_id {
            return;
        }
        let zones = &mut self.cfg_mut().active_mut().zones;
        let Some(s) = zones.iter().find(|z| z.id == src).cloned() else {
            self.cancel_merge();
            return;
        };
        let Some(t) = zones.iter().find(|z| z.id == target_id).cloned() else {
            self.cancel_merge();
            return;
        };
        let nx = s.x.min(t.x);
        let ny = s.y.min(t.y);
        let ex = (s.x + s.w).max(t.x + t.w);
        let ey = (s.y + s.h).max(t.y + t.h);
        if let Some(tz) = zones.iter_mut().find(|z| z.id == target_id) {
            tz.x = nx;
            tz.y = ny;
            tz.w = ex - nx;
            tz.h = ey - ny;
        }
        zones.retain(|z| z.id != src);
        self.selected_zone = target_id;
        self.fields_zone = -1;
        self.cancel_merge();
        self.push_history();
        self.log(
            &format!("Zones merged into zone {}", target_id),
            LogClass::Good,
        );
    }
}
