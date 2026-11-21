//! Modal dialogs: wizard, app picker, help, reset confirmation, text input,
//! close confirmation, and the transient toast.

use super::rail;
use super::state::{
    AppEntry, LogClass, PickerTab, TextDialogKind, WizardOption, PRESETS, TEMPLATES,
};
use super::tokens::*;
use super::Editor;
use crate::config::{self, CloseAction, Profile};
use eframe::egui;
use shadcn_egui::components::badge::Badge;
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::dialog::Dialog;
use shadcn_egui::components::field::Field;
use shadcn_egui::components::input::Input;
use shadcn_egui::components::kbd::{Kbd, KbdGroup};
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::scroll_area::ScrollArea;
use shadcn_egui::components::separator::separator;
use shadcn_egui::components::tabs::Tabs;
use shadcn_egui::icons::lucide;
use shadcn_egui::{layout, Theme};
use std::time::{Duration, Instant};

impl Editor {
    /// Return the current running-app list, re-enumerating only when the cache
    /// is empty or older than the configured `running_apps_ttl_secs` (default 2s).
    /// This keeps the "Running now" tab live without spawning `tasklist` on every
    /// frame (N1).
    fn running_apps_cached(&mut self) -> &[AppEntry] {
        let ttl = Duration::from_secs(self.cfg().settings.running_apps_ttl_secs);
        let stale = self
            .running_apps_cache
            .as_ref()
            .map(|(at, _)| at.elapsed() >= ttl)
            .unwrap_or(true);
        if stale {
            let apps = rail::enumerate_running_apps();
            self.running_apps_cache = Some((Instant::now(), apps));
        }
        // The `Some` was just ensured above; fall back to an empty slice only
        // if enumeration somehow produced nothing (still a valid borrow).
        self.running_apps_cache
            .as_ref()
            .map(|(_, apps)| apps.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn wizard_modal(&mut self, ctx: &egui::Context) {
        if !self.wizard_open {
            return;
        }
        let mut open = true;
        let mut dismiss = false;
        let mut apply = false;
        Dialog::new("zf_wizard", "New layout")
            .description("Choose how you want to lay out the current profile.")
            .width(660.0)
            .show(ctx, &mut open, |ui| {
                layout::vstack(ui, SPACE_16, |ui| {
                    layout::row_top(ui, SPACE_20, |ui| {
                        layout::vstack(ui, SPACE_8, |ui| {
                            ui_label(ui, "WHAT DO YOU WANT TO DO?");
                            let opts = [
                                ("Use a template", WizardOption::Template),
                                ("Custom grid", WizardOption::Grid),
                                ("Single zone", WizardOption::Single),
                                ("Draw on grid", WizardOption::Draw),
                            ];
                            for (name, opt) in opts {
                                let on = self.wizard.option == opt;
                                if ui
                                    .add(Button::new(name).variant(if on {
                                        ButtonVariant::Default
                                    } else {
                                        ButtonVariant::Outline
                                    }))
                                    .clicked()
                                {
                                    self.wizard.option = opt;
                                }
                            }
                            separator(ui);
                            match self.wizard.option {
                                WizardOption::Template => {
                                    for (i, (name, _)) in TEMPLATES.iter().enumerate() {
                                        let on = self.wizard.template == i;
                                        if ui
                                            .add(
                                                Button::new(*name)
                                                    .variant(if on {
                                                        ButtonVariant::Secondary
                                                    } else {
                                                        ButtonVariant::Outline
                                                    })
                                                    .size(ButtonSize::Sm),
                                            )
                                            .clicked()
                                        {
                                            self.wizard.template = i;
                                        }
                                    }
                                }
                                WizardOption::Grid => {
                                    layout::row(ui, SPACE_8, |ui| {
                                        Field::new().horizontal().label("Cols").show(ui, |ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut self.wizard.cols)
                                                    .range(1..=8),
                                            );
                                        });
                                        Field::new().horizontal().label("Rows").show(ui, |ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut self.wizard.rows)
                                                    .range(1..=6),
                                            );
                                        });
                                    });
                                }
                                WizardOption::Single => {
                                    for (i, (name, _)) in PRESETS.iter().enumerate() {
                                        let on = self.wizard.preset == i;
                                        if ui
                                            .add(
                                                Button::new(*name)
                                                    .variant(if on {
                                                        ButtonVariant::Secondary
                                                    } else {
                                                        ButtonVariant::Outline
                                                    })
                                                    .size(ButtonSize::Sm),
                                            )
                                            .clicked()
                                        {
                                            self.wizard.preset = i;
                                        }
                                    }
                                }
                                WizardOption::Draw => {
                                    ui_label(ui, "Closes this window and turns on Draw mode.");
                                }
                            }
                        });
                        layout::vstack(ui, SPACE_8, |ui| {
                            ui_label(ui, "PREVIEW");
                            let (pr, _) = ui.allocate_exact_size(
                                egui::vec2(300.0, 230.0),
                                egui::Sense::hover(),
                            );
                            let painter = ui.painter();
                            let t = self.canvas_theme(ui.ctx());
                            painter.rect_filled(pr, t.r_md, t.bg);
                            painter.rect_stroke(
                                pr,
                                t.r_md,
                                egui::Stroke::new(1.0, t.line2),
                                egui::StrokeKind::Inside,
                            );
                            let zones = self.wizard.zones();
                            if zones.is_empty() {
                                painter.text(
                                    pr.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "Empty layout",
                                    egui::FontId::proportional(t.f_small),
                                    t.faint,
                                );
                            }
                            for (i, (x, y, w, h)) in zones.iter().enumerate() {
                                let zr = egui::Rect::from_min_max(
                                    pr.min
                                        + egui::vec2(
                                            x / 100.0 * pr.width(),
                                            y / 100.0 * pr.height(),
                                        ),
                                    pr.min
                                        + egui::vec2(
                                            (x + w) / 100.0 * pr.width(),
                                            (y + h) / 100.0 * pr.height(),
                                        ),
                                );
                                painter.rect_filled(zr.shrink(2.0), t.r_sm, t.accent_bg);
                                painter.rect_stroke(
                                    zr.shrink(2.0),
                                    t.r_sm,
                                    egui::Stroke::new(1.0, t.accent_line),
                                    egui::StrokeKind::Inside,
                                );
                                painter.text(
                                    zr.center(),
                                    egui::Align2::CENTER_CENTER,
                                    (i + 1).to_string(),
                                    egui::FontId::proportional(t.f_small),
                                    t.accent2,
                                );
                            }
                        });
                    });
                    separator(ui);
                    layout::space_between(
                        ui,
                        |ui| {
                            ui_label(ui, "This replaces the zones in the current profile.");
                        },
                        |ui| {
                            if ui
                                .add(Button::new("Cancel").variant(ButtonVariant::Ghost))
                                .clicked()
                            {
                                dismiss = true;
                            }
                            if ui
                                .add(Button::new("Apply layout").icon(lucide::CHECK))
                                .clicked()
                            {
                                apply = true;
                                dismiss = true;
                            }
                        },
                    );
                });
            });
        if dismiss {
            open = false;
        }
        if apply {
            if self.wizard.option == WizardOption::Draw {
                if !self.draw_active {
                    self.toggle_draw();
                }
            } else {
                let zones = self.wizard.zones();
                self.apply_layout(&zones);
            }
        }
        if !open {
            self.wizard_open = false;
        }
    }

    pub(super) fn picker_modal(&mut self, ctx: &egui::Context) {
        if !self.picker_open {
            return;
        }
        let mut open = true;
        let mut dismiss = false;
        let mut attached: Option<(String, String, String)> = None;
        Dialog::new("zf_picker", "Choose an app")
            .description("Pick an app to place it in the selected zone.")
            .width(500.0)
            .show(ctx, &mut open, |ui| {
                layout::vstack(ui, SPACE_12, |ui| {
                    let mut tab_idx = match self.picker.tab {
                        PickerTab::Installed => 0,
                        PickerTab::Running => 1,
                        PickerTab::Browse => 2,
                    };
                    if ui
                        .add(Tabs::new(
                            &mut tab_idx,
                            &["Installed", "Running now", "Browse"],
                        ))
                        .changed()
                    {
                        self.picker.tab = match tab_idx {
                            0 => PickerTab::Installed,
                            1 => PickerTab::Running,
                            _ => PickerTab::Browse,
                        };
                    }
                    separator(ui);
                    match self.picker.tab {
                        PickerTab::Installed | PickerTab::Running => {
                            // Clone into an owned Vec so the short-TTL running-app
                            // cache borrow of `self` ends before the loop below
                            // mutably borrows `self` again (attach_app_to_selected).
                            let list: Vec<AppEntry> = if self.picker.tab == PickerTab::Installed {
                                rail::installed_apps().to_vec()
                            } else {
                                self.running_apps_cached().to_vec()
                            };
                            ui.add(
                                Input::new(&mut self.picker.search)
                                    .hint("Search...")
                                    .max_width(f32::INFINITY),
                            );
                            let q = self.picker.search.to_lowercase();
                            ScrollArea::vertical()
                                .max_height(DIALOG_LIST_H)
                                .show(ui, |ui| {
                                    if list.is_empty() {
                                        ui.label(
                                            egui::RichText::new("No apps found - use Browse...")
                                                .weak(),
                                        );
                                    }
                                    for app in &list {
                                        if !app.label.to_lowercase().contains(&q)
                                            && !app.exe.to_lowercase().contains(&q)
                                        {
                                            continue;
                                        }
                                        if ui
                                            .add(
                                                Button::new(app.label.as_str())
                                                    .icon(lucide::APP_WINDOW)
                                                    .variant(ButtonVariant::Ghost),
                                            )
                                            .clicked()
                                        {
                                            attached = Some((
                                                app.label.clone(),
                                                app.exe.clone(),
                                                app.path.clone(),
                                            ));
                                            dismiss = true;
                                        }
                                    }
                                });
                        }
                        PickerTab::Browse => {
                            Field::new()
                                .label("Executable path or name")
                                .show(ui, |ui| {
                                    ui.add(
                                        Input::new(&mut self.picker.path)
                                            .hint("C:/Program Files/App/app.exe")
                                            .max_width(f32::INFINITY),
                                    );
                                });
                            layout::row(ui, SPACE_8, |ui| {
                                if ui
                                    .add(
                                        Button::new("Choose file...")
                                            .icon(lucide::FOLDER_OPEN)
                                            .variant(ButtonVariant::Outline),
                                    )
                                    .clicked()
                                {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("exe", &["exe"])
                                        .pick_file()
                                    {
                                        self.picker.path = path.to_string_lossy().into_owned();
                                    }
                                }
                                if ui.add(Button::new("Attach").icon(lucide::CHECK)).clicked() {
                                    let p = self.picker.path.trim();
                                    if !p.is_empty() {
                                        let exe =
                                            p.rsplit(['/', '\\']).next().unwrap_or(p).to_string();
                                        let label = exe
                                            .strip_suffix(".exe")
                                            .or_else(|| exe.strip_suffix(".EXE"))
                                            .unwrap_or(&exe)
                                            .to_string();
                                        attached = Some((label, exe, p.to_string()));
                                        dismiss = true;
                                    } else {
                                        self.toast("Enter a path or choose a file");
                                    }
                                }
                            });
                            ui_label(ui, "The file name becomes the app label.");
                        }
                    }
                });
            });
        if dismiss {
            open = false;
        }
        if let Some((label, exe, path)) = attached {
            self.attach_app_to_selected(label, exe, path);
        }
        if !open {
            self.picker_open = false;
        }
    }

    pub(super) fn help_modal(&mut self, ctx: &egui::Context) {
        if !self.help_open {
            return;
        }
        let mut open = true;
        let mut dismiss = false;
        Dialog::new("zf_help", "Quick help")
            .description("A quick tour of ZoneFlow Studio.")
            .width(480.0)
            .show(ctx, &mut open, |ui| {
                layout::vstack(ui, SPACE_MD, |ui| {
                    let lines = [
                        (
                            "Create a layout:",
                            "click Layout for the wizard, or turn on Draw and drag on the grid.",
                        ),
                        (
                            "Attach apps:",
                            "drag one from the left library onto a zone, or double-click it.",
                        ),
                        (
                            "Edit:",
                            "click a zone or an app row, then use the inspector on the right.",
                        ),
                        (
                            "Shortcuts:",
                            "F2 rename, Delete remove, Esc deselect, Arrows nudge, Ctrl+Z undo.",
                        ),
                        (
                            "Tools:",
                            "bottom-left opens Hotkeys, Settings, Reset and this help.",
                        ),
                    ];
                    for (k, v) in lines {
                        layout::row(ui, SPACE_8, |ui| {
                            ui.add(egui::Label::new(egui::RichText::new(k).strong()));
                            ui.add(egui::Label::new(v));
                        });
                    }
                    separator(ui);
                    KbdGroup::new().show(ui, |ui| {
                        ui.add(Kbd::new("Ctrl"));
                        ui.add(Kbd::new("Z"));
                        ui_label(ui, "Undo");
                    });
                    KbdGroup::new().show(ui, |ui| {
                        ui.add(Kbd::new("F2"));
                        ui_label(ui, "Rename zone");
                    });
                    if ui.add(Button::new("Got it")).clicked() {
                        dismiss = true;
                    }
                });
            });
        if dismiss {
            open = false;
        }
        if !open {
            self.help_open = false;
        }
    }

    pub(super) fn reset_confirm_modal(&mut self, ctx: &egui::Context) {
        if !self.reset_confirm_open {
            return;
        }
        let mut open = true;
        let mut dismiss = false;
        let mut confirm = false;
        Dialog::new("zf_reset", "Reset Demo?")
            .description(
                "This restores the empty Website, Coding and Trading starter profiles and clears your saved demo data.",
            )
            .width(420.0)
            .show(ctx, &mut open, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new("Cancel").variant(ButtonVariant::Ghost))
                        .clicked()
                    {
                        dismiss = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(Button::new("Confirm").icon(lucide::ROTATE_CCW))
                            .clicked()
                        {
                            confirm = true;
                            dismiss = true;
                        }
                    });
                });
            });
        if dismiss {
            open = false;
        }
        if confirm {
            self.reset_demo();
        }
        if !open {
            self.reset_confirm_open = false;
        }
    }

    pub(super) fn text_dialog_modal(&mut self, ctx: &egui::Context) {
        let Some(mut dlg) = self.text_dialog.take() else {
            return;
        };
        let title = match dlg.kind {
            TextDialogKind::RenameZone(_) => "Rename zone",
            TextDialogKind::NewProfile => "New profile name",
            TextDialogKind::RenameProfile => "Rename profile",
        };
        let mut open = true;
        let mut dismiss = false;
        let mut commit = false;
        Dialog::new("zf_text_dialog", title)
            .width(380.0)
            .show(ctx, &mut open, |ui| {
                layout::vstack(ui, SPACE_12, |ui| {
                    let resp = ui.add(
                        Input::new(&mut dlg.value)
                            .hint("Name")
                            .max_width(f32::INFINITY),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        commit = true;
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add(Button::new("Cancel").variant(ButtonVariant::Ghost))
                            .clicked()
                        {
                            dismiss = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(Button::new("Save").icon(lucide::CHECK)).clicked() {
                                commit = true;
                            }
                        });
                    });
                });
            });
        if dismiss {
            open = false;
        }
        if commit {
            let v = dlg.value.trim().to_string();
            if !v.is_empty() {
                match dlg.kind {
                    TextDialogKind::RenameZone(id) => {
                        if let Some(z) = self
                            .cfg_mut()
                            .active_mut()
                            .zones
                            .iter_mut()
                            .find(|z| z.id == id)
                        {
                            z.label = v;
                        }
                        self.push_history();
                        self.log("Zone renamed", LogClass::Good);
                    }
                    TextDialogKind::NewProfile => {
                        if !self.cfg().profiles.contains_key(&v) {
                            self.cfg_mut().profiles.insert(
                                v.clone(),
                                Profile {
                                    name: v.clone(),
                                    hotkey: String::new(),
                                    zones: Vec::new(),
                                },
                            );
                            self.cfg_mut().active_profile = v;
                            self.selected_zone = 0;
                            self.fields_zone = -1;
                            self.push_history();
                            self.log("Profile created", LogClass::Good);
                        } else {
                            self.toast("Name exists");
                        }
                    }
                    TextDialogKind::RenameProfile => {
                        let old = self.cfg().active_profile.clone();
                        if old != v && !self.cfg().profiles.contains_key(&v) {
                            if let Some(p) = self.cfg_mut().profiles.remove(&old) {
                                let mut p = p;
                                p.name = v.clone();
                                self.cfg_mut().profiles.insert(v.clone(), p);
                                self.cfg_mut().active_profile = v;
                                self.push_history();
                                self.log("Profile renamed", LogClass::Good);
                            }
                        } else {
                            self.toast("Name exists");
                        }
                    }
                }
                open = false;
            }
        }
        if open {
            self.text_dialog = Some(dlg);
        }
    }

    pub(super) fn close_dialog_ui(&mut self, ctx: &egui::Context) {
        if !self.close_dialog {
            return;
        }
        let mut open = true;
        let mut dismiss = false;
        let mut remember = matches!(
            self.cfg().close_action,
            CloseAction::Minimize | CloseAction::Exit
        );
        let mut minimize = false;
        let mut exit = false;
        Dialog::new("zf_close", "Close ZoneFlow Studio?")
            .description(
                "Keep running in the tray so your layout and hotkeys stay active, or exit completely.",
            )
            .width(440.0)
            .show(ctx, &mut open, |ui| {
                layout::vstack(ui, SPACE_12, |ui| {
                    Field::new()
                        .horizontal()
                        .label("Remember my choice")
                        .show(ui, |ui| {
                            ui.add(Checkbox::new(&mut remember));
                        });
                    ui.horizontal(|ui| {
                        if ui
                            .add(Button::new("Cancel").variant(ButtonVariant::Ghost))
                            .clicked()
                        {
                            dismiss = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    Button::new("Exit ZoneFlow")
                                        .icon(lucide::LOG_OUT)
                                        .variant(ButtonVariant::Outline),
                                )
                                .clicked()
                            {
                                exit = true;
                                dismiss = true;
                            }
                            if ui
                                .add(Button::new("Minimize to tray").icon(lucide::MINIMIZE_2))
                                .clicked()
                            {
                                minimize = true;
                                dismiss = true;
                            }
                        });
                    });
                });
            });
        if dismiss {
            open = false;
        }
        if minimize {
            self.cfg_mut().close_action = if remember {
                CloseAction::Minimize
            } else {
                CloseAction::Ask
            };
            if let Err(e) = config::save_config(self.cfg()) {
                tracing::warn!(error = %e, "Failed to persist close action");
            }
            self.last_saved_json = serde_json::to_string(self.cfg()).unwrap_or_default();
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if exit {
            self.cfg_mut().close_action = if remember {
                CloseAction::Exit
            } else {
                CloseAction::Ask
            };
            if let Err(e) = config::save_config(self.cfg()) {
                tracing::warn!(error = %e, "Failed to persist close action");
            }
            self.last_saved_json = serde_json::to_string(self.cfg()).unwrap_or_default();
            self.exiting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !open {
            self.close_dialog = false;
        }
    }

    pub(super) fn toast_ui(&mut self, ctx: &egui::Context) {
        let expired = self
            .toast
            .as_ref()
            .map(|(_, t)| t.elapsed().as_secs() >= 3)
            .unwrap_or(false);
        if expired {
            self.toast = None;
        }
        if let Some((msg, _)) = &self.toast {
            let p = *Theme::current(ctx).palette();
            egui::Window::new("toast")
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
                .collapsible(false)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(p.popover)
                        .stroke(egui::Stroke::new(1.0, p.border))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(MARGIN_DIALOG.0, MARGIN_DIALOG.1)),
                )
                .show(ctx, |ui| {
                    layout::row(ui, SPACE_8, |ui| {
                        ui.add(Badge::new("").icon(lucide::INFO));
                        ui.add(egui::Label::new(msg.as_str()));
                    });
                });
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }
}
