//! Bottom action bar UI.

use super::state::DragZoneApp;
use super::tokens::*;
use super::Editor;
use crate::config::Zone;
use eframe::egui;
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::item::{Item, ItemSize};
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::scroll_area::ScrollArea;
use shadcn_egui::icons::lucide;
use shadcn_egui::layout;

impl Editor {
    pub(super) fn action_bar(&mut self, ui: &mut egui::Ui) {
        let p = *shadcn_egui::Theme::current(ui.ctx()).palette();
        let frame = egui::Frame::new()
            .fill(p.card)
            .stroke(egui::Stroke::new(1.0, p.border))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 4));

        frame.show(ui, |ui| {
            layout::space_between(
                ui,
                |ui| {
                    ui_label(ui, "Workspace Actions");
                },
                |ui| {
                    layout::row(ui, SPACE_SM, |ui| {
                        let clean = !self.dirty();
                        if ui
                            .add_enabled(
                                !clean,
                                Button::new("Revert")
                                    .icon(lucide::ROTATE_CCW)
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm),
                            )
                            .clicked()
                        {
                            self.revert();
                        }
                        let save_label = if clean { "Saved" } else { "Save & Apply" };
                        if ui
                            .add(
                                Button::new(save_label)
                                    .icon(lucide::SAVE)
                                    .variant(if clean {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Default
                                    })
                                    .size(ButtonSize::Sm),
                            )
                            .clicked()
                        {
                            self.save_apply();
                        }
                        if ui
                            .add(
                                Button::new("Launch")
                                    .icon(lucide::PLAY)
                                    .variant(ButtonVariant::Default)
                                    .size(ButtonSize::Sm),
                            )
                            .clicked()
                        {
                            self.run_sequence("launch");
                        }
                    });
                },
            );
        });
    }

    pub(super) fn apps_strip(&mut self, ui: &mut egui::Ui) {
        ui.add_space(SPACE_SM);
        layout::space_between(
            ui,
            |ui| {
                ui_label(ui, "APPS IN THIS WORKSPACE - CLICK A ROW TO EDIT");
            },
            |ui| {
                if ui
                    .add(
                        Button::new("Add app")
                            .icon(lucide::PLUS)
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm),
                    )
                    .clicked()
                {
                    self.picker_open = true;
                }
            },
        );
        let rows: Vec<Zone> = self
            .cfg()
            .active()
            .zones
            .iter()
            .filter(|z| !z.exe.is_empty())
            .cloned()
            .collect();
        ScrollArea::vertical()
            .id_salt("apps_strip")
            .max_height(APPS_STRIP_H)
            .show(ui, |ui| {
                if rows.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No apps assigned yet - use Add app or drag one from the left.",
                        )
                        .weak(),
                    );
                }
                for z in rows {
                    let on = z.id == self.selected_zone;
                    let resp = Item::new()
                        .icon(lucide::APP_WINDOW)
                        .title(z.label.as_str())
                        .description(z.exe.as_str())
                        .size(ItemSize::Sm)
                        .selected(on)
                        .sense(egui::Sense::click_and_drag())
                        .id_salt(("app_row", z.id))
                        .show(ui, |ui| {
                            ui.add(
                                Badge::new(format!("Zone {}", z.id))
                                    .variant(BadgeVariant::Secondary),
                            );
                        })
                        .response;
                    resp.dnd_set_drag_payload(DragZoneApp { id: z.id });
                    if resp.clicked() {
                        self.selected_zone = z.id;
                        self.fields_zone = -1;
                    }
                }
            });
    }
}
