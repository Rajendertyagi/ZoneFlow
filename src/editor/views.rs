//! Auxiliary views (hotkeys, settings).

use super::tokens::*;
use super::Editor;
use crate::config::CloseAction;
use eframe::egui;
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::card::{card_description, card_title, Card};
use shadcn_egui::components::field::Field;
use shadcn_egui::components::input::Input;
use shadcn_egui::components::kbd::{Kbd, KbdGroup};
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::scroll_area::ScrollArea;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::separator::separator;
use shadcn_egui::components::slider::Slider;
use shadcn_egui::components::switch::Switch;
use shadcn_egui::icons::lucide;
use shadcn_egui::layout;

impl Editor {
    pub(super) fn hotkeys_view(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("hk_scroll").show(ui, |ui| {
            layout::vstack(ui, SPACE_16, |ui| {
                card_title(ui, "Hotkeys");
                card_description(
                    ui,
                    "Global bindings - editable. Changes are stored in config.",
                );
                Card::new().show(ui, |ui| {
                    layout::vstack(ui, SPACE_MD, |ui| {
                        let keys: Vec<(String, String)> = self
                            .cfg()
                            .hotkeys
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        for (name, binding) in keys {
                            Field::new()
                                .horizontal()
                                .label(name.as_str())
                                .show(ui, |ui| {
                                    let mut b = binding.clone();
                                    if ui
                                        .add(Input::new(&mut b).max_width(f32::INFINITY))
                                        .changed()
                                    {
                                        self.cfg_mut().hotkeys.insert(name.clone(), b);
                                        self.push_history();
                                    }
                                });
                            separator(ui);
                        }
                    });
                });
            });
        });
    }

    pub(super) fn settings_view(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().id_salt("set_scroll").show(ui, |ui| {
            layout::vstack(ui, SPACE_16, |ui| {
                card_title(ui, "Settings");
                card_description(ui, "Behaviour and engine preferences.");

                Card::new().show(ui, |ui| {
                    layout::vstack(ui, SPACE_12, |ui| {
                        card_title(ui, "Behaviour");
                        separator(ui);
                        let mut auto = self.cfg().auto_start;
                        Field::new()
                            .horizontal()
                            .label("Start with Windows")
                            .show(ui, |ui| {
                                if ui.add(Switch::new(&mut auto)).changed() {
                                    self.cfg_mut().auto_start = auto;
                                    crate::autostart::set_auto_start(auto);
                                    self.push_history();
                                }
                            });
                        let mut launch = self.cfg().settings.auto_launch_on_startup;
                        Field::new()
                            .horizontal()
                            .label("Launch & snap all apps on startup")
                            .description("Takes effect the next time the app starts.")
                            .show(ui, |ui| {
                                if ui.add(Switch::new(&mut launch)).changed() {
                                    self.cfg_mut().settings.auto_launch_on_startup = launch;
                                    self.push_history();
                                }
                            });
                        let mut close_idx = match self.cfg().close_action {
                            CloseAction::Ask => 0,
                            CloseAction::Minimize => 1,
                            CloseAction::Exit => 2,
                        };
                        let opts = ["Ask every time", "Minimize to tray", "Exit completely"];
                        Field::new()
                            .horizontal()
                            .label("Close behavior")
                            .show(ui, |ui| {
                                ui.add(
                                    Select::new("close_combo", &mut close_idx, &opts)
                                        .width(SELECT_W),
                                );
                            });
                        let close = match close_idx {
                            0 => CloseAction::Ask,
                            1 => CloseAction::Minimize,
                            _ => CloseAction::Exit,
                        };
                        if close != self.cfg().close_action {
                            self.cfg_mut().close_action = close;
                            self.push_history();
                        }
                    });
                });

                Card::new().show(ui, |ui| {
                    layout::vstack(ui, SPACE_12, |ui| {
                        card_title(ui, "Layout");
                        separator(ui);
                        let mut gap = self.cfg().gap as f32;
                        Field::new().title("Gap (px)").show(ui, |ui| {
                            if ui
                                .add(Slider::new(&mut gap, 0.0..=40.0).step(1.0).label("Gap"))
                                .changed()
                            {
                                self.cfg_mut().gap = gap as i32;
                                self.push_history();
                            }
                        });
                        ui.add(
                            Badge::new(format!("{}px", self.cfg().gap))
                                .variant(BadgeVariant::Secondary),
                        );
                    });
                });

                Card::new().show(ui, |ui| {
                    layout::vstack(ui, SPACE_12, |ui| {
                        card_title(ui, "Shortcuts");
                        card_description(ui, "Keyboard shortcuts available in the canvas.");
                        separator(ui);
                        KbdGroup::new().show(ui, |ui| {
                            ui.add(Kbd::new("Ctrl"));
                            ui.add(Kbd::new("Z"));
                            ui_label(ui, "Undo");
                        });
                        KbdGroup::new().show(ui, |ui| {
                            ui.add(Kbd::new("Ctrl"));
                            ui.add(Kbd::new("Shift"));
                            ui.add(Kbd::new("Z"));
                            ui_label(ui, "Redo");
                        });
                        KbdGroup::new().show(ui, |ui| {
                            ui.add(Kbd::new("F2"));
                            ui_label(ui, "Rename zone");
                        });
                        KbdGroup::new().show(ui, |ui| {
                            ui.add(Kbd::new("Delete"));
                            ui_label(ui, "Remove zone");
                        });
                    });
                });

                Card::new().show(ui, |ui| {
                    layout::vstack(ui, SPACE_12, |ui| {
                        card_title(ui, "Demo data");
                        separator(ui);
                        if ui
                            .add(
                                Button::new("Reset Demo")
                                    .icon(lucide::REFRESH_CW)
                                    .variant(ButtonVariant::Outline),
                            )
                            .clicked()
                        {
                            self.reset_confirm_open = true;
                        }
                    });
                });
            });
        });
    }
}
