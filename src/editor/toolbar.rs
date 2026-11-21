//! Toolbar UI.

use super::state::LogClass;
use super::tokens::*;
use super::Editor;
use eframe::egui;
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::field::Field;
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::separator::Separator;
use shadcn_egui::components::slider::Slider;
use shadcn_egui::icons::lucide;
use shadcn_egui::layout;

impl Editor {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        layout::wrap_row(ui, SPACE_8, |ui| {
            if ui.add(Button::new("Layout").icon(lucide::LAYOUT)).clicked() {
                self.wizard_open = true;
            }
            ui.add(Separator::new().vertical());

            let draw_on = self.draw_mode();
            if ui
                .add(Button::new("Draw").icon(lucide::GRID).variant(if draw_on {
                    ButtonVariant::Default
                } else {
                    ButtonVariant::Outline
                }))
                .clicked()
            {
                self.toggle_draw();
            }
            if draw_on {
                ui_label(ui, "Grid");
                let mut cols = self.cfg().settings.grid_cols;
                let mut rows = self.cfg().settings.grid_rows;
                if ui
                    .add(egui::DragValue::new(&mut cols).range(2..=24).speed(0.2))
                    .changed()
                {
                    self.cfg_mut().settings.grid_cols = cols;
                }
                ui_label(ui, "x");
                if ui
                    .add(egui::DragValue::new(&mut rows).range(2..=16).speed(0.2))
                    .changed()
                {
                    self.cfg_mut().settings.grid_rows = rows;
                }
                Field::new().horizontal().label("Snap").show(ui, |ui| {
                    ui.add(Checkbox::new(&mut self.snap_draw));
                });
                if ui
                    .add(
                        Button::new("Clear")
                            .icon(lucide::ERASER)
                            .variant(ButtonVariant::Ghost),
                    )
                    .clicked()
                {
                    self.cfg_mut().active_mut().zones.clear();
                    self.selected_zone = 0;
                    self.push_history();
                    self.log("Zones cleared", LogClass::Accent);
                }
            }
            ui.add(Separator::new().vertical());

            let can_undo = self.hindex > 0;
            let can_redo = self.hindex + 1 < self.history.len();
            if ui
                .add_enabled(
                    can_undo,
                    Button::new("Undo")
                        .icon(lucide::UNDO)
                        .variant(ButtonVariant::Outline),
                )
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(
                    can_redo,
                    Button::new("Redo")
                        .icon(lucide::REDO)
                        .variant(ButtonVariant::Outline),
                )
                .clicked()
            {
                self.redo();
            }
            ui.add(Separator::new().vertical());

            if ui
                .add(
                    Button::new("Split V")
                        .icon(lucide::MOVE_HORIZONTAL)
                        .variant(ButtonVariant::Outline),
                )
                .clicked()
            {
                self.split_zone(true, 0.5);
            }
            if ui
                .add(
                    Button::new("Split H")
                        .icon(lucide::MOVE_VERTICAL)
                        .variant(ButtonVariant::Outline),
                )
                .clicked()
            {
                self.split_zone(false, 0.5);
            }
            if self.merge_mode {
                ui.add(Button::new("Merge...").icon(lucide::COMBINE));
            } else if ui
                .add(
                    Button::new("Merge")
                        .icon(lucide::COMBINE)
                        .variant(ButtonVariant::Outline),
                )
                .clicked()
            {
                self.begin_merge();
            }
            if ui
                .add(
                    Button::new("Remove")
                        .icon(lucide::TRASH)
                        .variant(ButtonVariant::Outline),
                )
                .clicked()
            {
                self.delete_zone();
            }
            ui.add(Separator::new().vertical());

            layout::row(ui, SPACE_SM, |ui| {
                ui_label(ui, "Gap");
                let mut gap = self.cfg().gap as f32;
                if ui
                    .add(Slider::new(&mut gap, 0.0..=40.0).step(1.0).label("Gap"))
                    .changed()
                {
                    self.cfg_mut().gap = gap as i32;
                    self.push_history();
                }
                ui_label(ui, &format!("{}px", self.cfg().gap));
            });
        });
    }

    fn draw_mode(&self) -> bool {
        self.draw_start.is_some() || self.draw_active
    }
}
