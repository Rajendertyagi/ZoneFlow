//! Inspector panel UI (zone & automation settings).

use super::state::{LogClass, StrField};
use super::tokens::*;
use super::Editor;
use crate::config::{LaunchMode, TabBehavior, Zone};
use eframe::egui;
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::card::{card_description, card_title, Card};
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::field::Field;
use shadcn_egui::components::input::Input;
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::scroll_area::ScrollArea;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::separator::separator;
use shadcn_egui::components::slider::Slider;
use shadcn_egui::icons::lucide;
use shadcn_egui::{layout, Theme};

impl Editor {
    /* ------------------------- inspector ------------------------- */

    pub(super) fn right_panel(&mut self, ui: &mut egui::Ui) {
        // Inner padding on all sides (especially right) so inputs never clip.
        egui::Frame::new()
            .inner_margin(egui::Margin::same(MARGIN_INSPECTOR))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("inspector_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        layout::vstack(ui, SPACE_16, |ui| {
                            card_title(ui, "Zone & Automation");
                            card_description(
                                ui,
                                "Configure the selected zone and its automation rules.",
                            );

                            let Some(zone) = self
                                .cfg()
                                .active()
                                .zones
                                .iter()
                                .find(|z| z.id == self.selected_zone)
                                .cloned()
                            else {
                                ui_label(ui, "Select a zone or click an app row to edit.");
                                return;
                            };
                            if self.fields_zone != zone.id {
                                self.sync_buffers(&zone);
                            }
                            let pw = ((zone.w / 100.0 * self.res.width() as f32)
                                - self.cfg().gap as f32)
                                .max(0.0)
                                .round() as i32;
                            let ph = ((zone.h / 100.0 * self.res.work_height() as f32)
                                - self.cfg().gap as f32)
                                .max(0.0)
                                .round() as i32;

                            layout::row(ui, SPACE_8, |ui| {
                                ui.add(
                                    Badge::new(format!("Zone {}", zone.id))
                                        .variant(BadgeVariant::Secondary),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "X:{} Y:{} - {}x{} px",
                                        (zone.x / 100.0 * self.res.width() as f32).round() as i32,
                                        (zone.y / 100.0 * self.res.work_height() as f32).round()
                                            as i32,
                                        pw,
                                        ph
                                    ))
                                    .weak()
                                    .monospace(),
                                );
                            });

                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        Button::new(if zone.exe.is_empty() {
                                            "Choose app"
                                        } else {
                                            "Change app"
                                        })
                                        .icon(lucide::SEARCH)
                                        .variant(ButtonVariant::Outline)
                                        .size(ButtonSize::Sm),
                                    )
                                    .clicked()
                                {
                                    self.picker_open = true;
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add(
                                                Button::new("Test zone")
                                                    .icon(lucide::ZAP)
                                                    .variant(ButtonVariant::Outline)
                                                    .size(ButtonSize::Sm),
                                            )
                                            .clicked()
                                        {
                                            self.run_sequence("test");
                                        }
                                    },
                                );
                            });

                            separator(ui);

                            // Section 1 - Window & process
                            Card::new().show(ui, |ui| {
                                layout::vstack(ui, SPACE_12, |ui| {
                                    card_title(ui, "Window & process");
                                    card_description(
                                        ui,
                                        "The executable and window matching rules.",
                                    );
                                    separator(ui);
                                    self.text_field(ui, "Label", StrField::Label, zone.id, false);
                                    self.text_field(ui, "Executable", StrField::Exe, zone.id, true);
                                    self.text_field(ui, "Path", StrField::Path, zone.id, true);
                                    self.launch_combo(ui, &zone);
                                });
                            });

                            // Section 2 - Browser tab & URL
                            Card::new().show(ui, |ui| {
                                layout::vstack(ui, SPACE_12, |ui| {
                                    card_title(ui, "Browser tab & URL");
                                    card_description(
                                        ui,
                                        "Open a URL and control how its tab is reused.",
                                    );
                                    separator(ui);
                                    self.text_field(ui, "URL", StrField::Url, zone.id, true);
                                    self.text_field(
                                        ui,
                                        "Title match",
                                        StrField::Title,
                                        zone.id,
                                        false,
                                    );
                                    self.tab_combo(ui, &zone);
                                });
                            });

                            // Section 3 - Keystroke macro
                            Card::new().show(ui, |ui| {
                                layout::vstack(ui, SPACE_12, |ui| {
                                    card_title(ui, "Keystroke macro");
                                    card_description(
                                        ui,
                                        "Send keys when a matching window appears.",
                                    );
                                    separator(ui);
                                    let mut on = zone.macro_enabled;
                                    Field::new().horizontal().label("Enable macro").show(
                                        ui,
                                        |ui| {
                                            if ui.add(Checkbox::new(&mut on)).changed() {
                                                if let Some(z) = self
                                                    .cfg_mut()
                                                    .active_mut()
                                                    .zones
                                                    .iter_mut()
                                                    .find(|z| z.id == zone.id)
                                                {
                                                    z.macro_enabled = on;
                                                }
                                                self.push_history();
                                            }
                                        },
                                    );
                                    self.text_field(
                                        ui,
                                        "Trigger title",
                                        StrField::Trigger,
                                        zone.id,
                                        false,
                                    );
                                    self.text_field(
                                        ui,
                                        "Keys / PIN",
                                        StrField::Macro,
                                        zone.id,
                                        true,
                                    );
                                    Field::new().title("Delay (ms)").show(ui, |ui| {
                                        if ui
                                            .add(
                                                Slider::new(
                                                    &mut self.edit_bufs.delay,
                                                    10.0..=1000.0,
                                                )
                                                .step(10.0)
                                                .label("Delay"),
                                            )
                                            .changed()
                                        {
                                            let d = self.edit_bufs.delay as u32;
                                            if let Some(z) = self
                                                .cfg_mut()
                                                .active_mut()
                                                .zones
                                                .iter_mut()
                                                .find(|z| z.id == zone.id)
                                            {
                                                z.key_delay = d;
                                            }
                                            self.push_history();
                                        }
                                    });
                                });
                            });

                            // Console
                            Card::new().show(ui, |ui| {
                                layout::vstack(ui, SPACE_MD, |ui| {
                                    card_title(ui, "Console");
                                    card_description(ui, "Execution log for this session.");
                                    separator(ui);
                                    ScrollArea::vertical()
                                        .id_salt("console_scroll")
                                        .stick_to_bottom(true)
                                        .max_height(CONSOLE_H)
                                        .show(ui, |ui| {
                                            // Shared success token from the canvas theme (B1).
                                            let good = self.canvas_theme(ui.ctx()).good;
                                            for line in &self.console {
                                                let color = match line.class {
                                                    LogClass::Good => good,
                                                    LogClass::Accent => {
                                                        Theme::current(ui.ctx()).palette().primary
                                                    }
                                                    LogClass::Bad => {
                                                        Theme::current(ui.ctx())
                                                            .palette()
                                                            .destructive
                                                    }
                                                    LogClass::Normal => {
                                                        Theme::current(ui.ctx())
                                                            .palette()
                                                            .muted_foreground
                                                    }
                                                };
                                                // Keep a single long line from dominating the inspector.
                                                let mut chars = line.text.chars();
                                                let text: String =
                                                    chars.by_ref().take(120).collect();
                                                let text = if chars.next().is_some() {
                                                    format!("{}...", text)
                                                } else {
                                                    text
                                                };
                                                ui.label(
                                                    egui::RichText::new(text)
                                                        .monospace()
                                                        .color(color)
                                                        .size(9.5),
                                                );
                                            }
                                        });
                                    if ui
                                        .add(
                                            Button::new("Clear console")
                                                .icon(lucide::ERASER)
                                                .variant(ButtonVariant::Ghost)
                                                .size(ButtonSize::Sm),
                                        )
                                        .clicked()
                                    {
                                        self.console.clear();
                                    }
                                });
                            });
                        });
                    });
            });
    }

    fn text_field(&mut self, ui: &mut egui::Ui, name: &str, field: StrField, id: i32, _mono: bool) {
        let (gained, changed, lost, value) = Field::new()
            .label(name)
            .show(ui, |ui| {
                let buf = match field {
                    StrField::Label => &mut self.edit_bufs.label,
                    StrField::Exe => &mut self.edit_bufs.exe,
                    StrField::Path => &mut self.edit_bufs.path,
                    StrField::Url => &mut self.edit_bufs.url,
                    StrField::Title => &mut self.edit_bufs.title,
                    StrField::Trigger => &mut self.edit_bufs.trigger,
                    StrField::Macro => &mut self.edit_bufs.macro_text,
                };
                let resp = ui.add(Input::new(&mut *buf).hint(name).max_width(f32::INFINITY));
                (
                    resp.gained_focus(),
                    resp.changed(),
                    resp.lost_focus(),
                    buf.clone(),
                )
            })
            .inner;
        if gained {
            self.edit_session_active = true;
        }
        if changed {
            self.set_zone_str(id, field, value);
        }
        if lost && self.edit_session_active {
            self.push_history();
            self.edit_session_active = false;
        }
    }

    fn launch_combo(&mut self, ui: &mut egui::Ui, zone: &Zone) {
        let mut idx = match zone.launch_mode {
            LaunchMode::Launch => 0,
            LaunchMode::Attach => 1,
        };
        let opts = ["Launch if closed", "Attach if running"];
        Field::new().label("Launch").show(ui, |ui| {
            ui.add(
                Select::new("launch_combo", &mut idx, &opts)
                    .width(ui.available_width().max(SELECT_MIN_W)),
            );
        });
        let mode = if idx == 0 {
            LaunchMode::Launch
        } else {
            LaunchMode::Attach
        };
        if mode != zone.launch_mode {
            let id = zone.id;
            if let Some(z) = self
                .cfg_mut()
                .active_mut()
                .zones
                .iter_mut()
                .find(|z| z.id == id)
            {
                z.launch_mode = mode;
            }
            self.push_history();
        }
    }

    fn tab_combo(&mut self, ui: &mut egui::Ui, zone: &Zone) {
        let mut idx = match zone.tab_behavior {
            TabBehavior::FocusOrNew => 0,
            TabBehavior::ReuseBlank => 1,
            TabBehavior::Ignore => 2,
        };
        let opts = ["Focus or Ctrl+T", "Reuse blank tab", "Window tile only"];
        Field::new().label("Tab rule").show(ui, |ui| {
            ui.add(
                Select::new("tab_combo", &mut idx, &opts)
                    .width(ui.available_width().max(SELECT_MIN_W)),
            );
        });
        let tb = match idx {
            0 => TabBehavior::FocusOrNew,
            1 => TabBehavior::ReuseBlank,
            _ => TabBehavior::Ignore,
        };
        if tb != zone.tab_behavior {
            let id = zone.id;
            if let Some(z) = self
                .cfg_mut()
                .active_mut()
                .zones
                .iter_mut()
                .find(|z| z.id == id)
            {
                z.tab_behavior = tb;
            }
            self.push_history();
        }
    }
}
