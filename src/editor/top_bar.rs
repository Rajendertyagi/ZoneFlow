//! Top bar UI.

use super::state::{LogClass, ResKey};
use super::tokens::*;
use super::Editor;
use eframe::egui;
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::separator::Separator;
use shadcn_egui::icons::lucide;
use shadcn_egui::layout;

impl Editor {
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui) {
        let profile = self.cfg().active_profile.clone();
        let dirty = self.dirty();
        layout::row(ui, SPACE_MD, |ui| {
            ui.add(Badge::new("Z").icon(lucide::ZAP));
            ui_label(ui, "ZoneFlow");
            ui.add(Separator::new().vertical());
            ui_label(ui, &profile);

            let mut res_idx = self.res.idx();
            let opts: Vec<&str> = ResKey::all().iter().map(|r| r.label()).collect();
            if ui
                .add(Select::new("res_preset", &mut res_idx, &opts).width(SELECT_W))
                .changed()
            {
                self.res = ResKey::all()[res_idx];
                self.push_history();
                self.log(
                    &format!("Resolution -> {}", self.res.label()),
                    LogClass::Accent,
                );
            }

            if dirty {
                ui.add(
                    Badge::new("Unsaved changes")
                        .variant(BadgeVariant::Outline)
                        .icon(lucide::CIRCLE_DOT),
                );
            }

            layout::space_between(
                ui,
                |_ui| {},
                |ui| {
                    ui.add(
                        Badge::new("Engine ready")
                            .variant(BadgeVariant::Secondary)
                            .icon(lucide::CHECK),
                    );
                },
            );
        });
    }
}
