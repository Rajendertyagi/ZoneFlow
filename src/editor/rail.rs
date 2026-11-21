//! Side rail UI: app library, profiles and tools dock.

use super::state::{AppEntry, TextDialogKind, TextDialogState, View};
use super::tokens::*;
use super::Editor;
use eframe::egui;
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::input::Input;
use shadcn_egui::components::item::{Item, ItemSize};
use shadcn_egui::components::label::label as ui_label;
use shadcn_egui::components::scroll_area::ScrollArea;
use shadcn_egui::components::separator::separator;
use shadcn_egui::icons::lucide;
use shadcn_egui::layout;

/* ============================================================================
 * APP ENUMERATION (§2a / §2.2) — real system data, no baked-in app lists.
 *
 * The Start Menu roots and Uninstall keys below are the documented *default*
 * app sources. config.rs has no field for them yet, so they are named here as
 * defaults rather than hardcoded user data (no app names/exes are baked in).
 * Results are cached with OnceLock: enumeration spawns `reg`/`tasklist` and
 * walks the Start Menu, which must not happen on every egui frame.
 * ========================================================================== */

/* --------------------------------------------------------------------------
 * APP-LIST DATA CLEANUP
 * Installed-app enumeration mixes Start Menu shortcut names, registry
 * DisplayNames and raw .exe filenames. Normalise them to one clean entry per
 * app: strip .exe / version strings / parentheticals, drop system junk, and
 * collapse sub-product names ("7-Zip File Manager" -> "7-Zip"). All lists are
 * named constants here because config.rs has no field for them.
 * ------------------------------------------------------------------------ */

/// Trailing words that mark a helper / sub-product of the base application.
#[cfg(windows)]
const GENERIC_SUFFIX: &[&str] = &[
    "file",
    "manager",
    "dash",
    "window",
    "spy",
    "help",
    "documentation",
    "docs",
    "uninstaller",
    "descriptions",
    "console",
    "launcher",
    "shortcut",
    "setup",
    "installer",
    "portable",
    "x64",
    "x86",
    "wow",
    "64-bit",
    "32-bit",
    "edition",
    "sdk",
    "runtime",
    "tool",
    "tools",
    "utility",
    "utilities",
    "plugin",
    "extension",
    "addon",
    "driver",
    "service",
    "update",
    "updater",
    "server",
    "client",
    "monitor",
    "center",
    "viewer",
    "editor",
    "player",
    "recorder",
    "converter",
    "codec",
    "pack",
    "bundle",
    "suite",
    "full",
    "pro",
    "plus",
    "free",
    "home",
    "standard",
    "professional",
    "ultimate",
    "trial",
    "demo",
    "beta",
    "rc",
    "preview",
    "release",
    "stable",
    "nightly",
    "canary",
    "dev",
    "bit",
    "version",
];

/// Leading vendor words removed from the dedup key only (display label kept).
#[cfg(windows)]
const VENDOR_PREFIX: &[&str] = &[
    "google ",
    "microsoft ",
    "adobe ",
    "autodesk ",
    "oracle ",
    "ibm ",
    "apple ",
    "samsung ",
    "hp ",
    "dell ",
    "lenovo ",
    "asus ",
    "nvidia ",
    "intel ",
    "amd ",
    "vmware ",
    "canonical ",
    "redhat ",
    "openai ",
    "visual studio ",
];

/// Multi-word / substring patterns for system utilities that are not apps.
#[cfg(windows)]
const JUNK_PHRASE: &[&str] = &[
    "application verifier",
    "administrative tools",
    "control panel",
    "component services",
    "computer management",
    "windows defender",
    "windows firewall",
    "windows update",
    "windows features",
    "windows powershell",
    "windows command prompt",
    "windows mobility",
    "windows system",
    "windows tools",
    "windows accessories",
    "webview2",
    "redistributable",
    "visual c++",
    ".net framework",
    "microsoft edge webview",
    "device manager",
    "disk management",
    "event viewer",
    "task scheduler",
    "registry editor",
    "system configuration",
    "system information",
    "resource monitor",
    "performance monitor",
    "print management",
    "dns manager",
    "file explorer",
    "internet explorer",
    // Windows system applets / Control Panel tools that clutter the library.
    "device pairing",
    "diagnosticshub",
    "directx",
    "font settings",
    "get help",
    "magnify",
    "memory diagnostics",
    "iscsi initiator",
    "net host",
    "windows desktop runtime",
    "partition wizard",
    "odbc data sources",
    "odbc screen saver",
    "screen saver",
    "windows sound",
    "windows speech",
    "windows voice",
    "text to speech",
    "wordpad",
    "xps viewer",
    "remote desktop",
    "windows accessories",
    "windows system tools",
    "microsoft windows",
    "powershell terminal",
];

/// Single words (matched as whole tokens) that mark junk entries.
#[cfg(windows)]
const JUNK_WORD: &[&str] = &[
    "help",
    "documentation",
    "docs",
    "uninstaller",
    "verifier",
    "readme",
    "support",
    "setup",
    "installer",
    "launcher",
    "shortcut",
    "uninstall",
];

/// Labels that are generic placeholders rather than real app names.
#[cfg(windows)]
const GENERIC_LABEL: &[&str] = &[
    "x64",
    "x86",
    "wow",
    "bit",
    "edition",
    "version",
    "app",
    "application",
    "program",
    "new folder",
    "desktop",
    "start menu",
    "quick launch",
    "more",
    "more info",
    "website",
    "update",
    "updater",
    "service",
    "services",
    "tools",
    "utilities",
];

/// Dedup map: normalised key -> (index in `apps`, source priority).
#[cfg(windows)]
type SeenMap = std::collections::HashMap<String, (usize, u8)>;

/// True when a token looks like a version ("6.3.0.0", "26.06", "v1.9.36", "365").
#[cfg(windows)]
fn is_version_token(tok: &str) -> bool {
    let t = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.');
    if t.is_empty() {
        return false;
    }
    let body = t
        .strip_prefix('v')
        .or_else(|| t.strip_prefix('V'))
        .unwrap_or(t);
    if body.is_empty() {
        return false;
    }
    if body.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    body.chars().all(|c| c.is_ascii_digit() || c == '.')
        && body.contains('.')
        && body.split('.').filter(|p| !p.is_empty()).count() >= 2
}

/// Remove every parenthetical group, e.g. " (x64 edition)" / " (WOW)".
#[cfg(windows)]
fn strip_parentheticals(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0u32;
    for ch in s.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
            }
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Turn a raw registry DisplayName / shortcut stem / .exe name into a clean label.
#[cfg(windows)]
fn clean_app_label(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.len() >= 4 && s[s.len() - 4..].eq_ignore_ascii_case(".exe") {
        s.truncate(s.len() - 4);
    }
    let s = strip_parentheticals(&s);
    s.split_whitespace()
        .filter(|tok| !is_version_token(tok))
        .collect::<Vec<_>>()
        .join(" ")
}

/// True for system utilities, installers, help/docs and generic placeholders.
#[cfg(windows)]
fn is_junk_app(label: &str) -> bool {
    let low = label.to_lowercase();
    if JUNK_PHRASE.iter().any(|p| low.contains(*p)) {
        return true;
    }
    let tokens: Vec<&str> = low
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '+' && c != '.')
        .filter(|t| !t.is_empty())
        .collect();
    if JUNK_WORD.iter().any(|w| tokens.contains(w)) {
        return true;
    }
    if GENERIC_LABEL.iter().any(|g| low == *g) {
        return true;
    }
    is_version_token(&low)
}

/// Normalised dedup key: lowercase, vendor prefix and trailing sub-product
/// words removed, whitespace collapsed. The display label is left untouched.
#[cfg(windows)]
fn app_dedup_key(clean: &str) -> String {
    let low = clean.to_lowercase();
    let mut key = low.as_str();
    for prefix in VENDOR_PREFIX {
        if let Some(rest) = key.strip_prefix(*prefix) {
            if !rest.trim().is_empty() {
                key = rest.trim_start();
            }
            break;
        }
    }
    let mut words: Vec<&str> = key.split_whitespace().collect();
    while words.len() > 1 {
        match words.last() {
            Some(last) if GENERIC_SUFFIX.contains(last) => {
                words.pop();
            }
            _ => break,
        }
    }
    if words.is_empty() {
        low
    } else {
        words.join(" ")
    }
}

/// Build a child command that does not flash a console window in this GUI app.
#[cfg(windows)]
fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(windows)]
fn push_app(
    label: String,
    exe: String,
    path: String,
    priority: u8,
    apps: &mut Vec<AppEntry>,
    seen: &mut SeenMap,
) {
    let clean = clean_app_label(&label);
    if clean.is_empty() || is_junk_app(&clean) {
        return;
    }
    let key = app_dedup_key(&clean);
    if key.is_empty() {
        return;
    }
    // Every entry needs a best-effort exe so attaching it never yields a blank zone.
    let exe = if exe.is_empty() {
        format!("{}.exe", clean)
    } else {
        exe
    };
    match seen.get(&key) {
        Some(&(idx, old_prio)) => {
            // Prefer the shortest / cleanest label; source priority breaks ties.
            let better = clean.len() < apps[idx].label.len()
                || (clean.len() == apps[idx].label.len() && priority > old_prio);
            if better {
                apps[idx] = AppEntry {
                    label: clean,
                    exe,
                    path,
                };
                seen.insert(key, (idx, priority));
            }
        }
        None => {
            seen.insert(key, (apps.len(), priority));
            apps.push(AppEntry {
                label: clean,
                exe,
                path,
            });
        }
    }
}

/// Recursively collect `.lnk` shortcuts. The stem is the label; a best-effort
/// `<stem>.exe` is used as the exe name (resolving the target is optional).
#[cfg(windows)]
fn scan_lnk_dir(dir: &std::path::Path, depth: u32, apps: &mut Vec<AppEntry>, seen: &mut SeenMap) {
    if depth > 2 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_lnk_dir(&path, depth + 1, apps, seen);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let clean = clean_app_label(stem);
            if clean.is_empty() {
                continue;
            }
            // Shortcuts are the preferred source (priority 2).
            push_app(
                clean.clone(),
                format!("{}.exe", clean),
                String::new(),
                2,
                apps,
                seen,
            );
        }
    }
}

/// Read `DisplayName` values from one registry Uninstall key (and its subkeys).
#[cfg(windows)]
fn scan_uninstall_key(key: &str, apps: &mut Vec<AppEntry>, seen: &mut SeenMap) {
    let output = hidden_command("reg")
        .args(["query", key, "/s", "/v", "DisplayName"])
        .output();
    let Ok(output) = output else { return };
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let Some(after) = line.split_once("DisplayName").map(|(_, r)| r) else {
            continue;
        };
        let after = after.trim();
        let value = if let Some(pos) = after.find("REG_EXPAND_SZ") {
            &after[pos + "REG_EXPAND_SZ".len()..]
        } else if let Some(pos) = after.find("REG_SZ") {
            &after[pos + "REG_SZ".len()..]
        } else {
            continue;
        };
        let name = value.trim();
        if !name.is_empty() {
            // Registry DisplayNames are noisier than shortcuts (priority 1).
            push_app(
                name.to_string(),
                String::new(),
                String::new(),
                1,
                apps,
                seen,
            );
        }
    }
}

#[cfg(windows)]
fn enumerate_installed_apps() -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = Vec::new();
    let mut seen: SeenMap = SeenMap::new();

    // 1) Start Menu shortcuts — per-user and machine-wide.
    let mut roots: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        roots
            .push(std::path::PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(programdata) = std::env::var("ProgramData") {
        roots.push(
            std::path::PathBuf::from(programdata).join(r"Microsoft\Windows\Start Menu\Programs"),
        );
    }
    for root in &roots {
        scan_lnk_dir(root, 0, &mut apps, &mut seen);
    }

    // 2) Registry Uninstall keys — HKLM (64/32-bit) + HKCU.
    for key in [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ] {
        scan_uninstall_key(key, &mut apps, &mut seen);
    }

    apps.sort_by(|a, b| a.label.cmp(&b.label));
    apps.truncate(crate::config::loaded_settings().app_list_cap);
    apps
}

#[cfg(windows)]
pub(super) fn enumerate_running_apps() -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = Vec::new();
    let mut seen: SeenMap = SeenMap::new();
    // tasklist CSV: "Image Name","PID","Session Name","Session#","Mem Usage"
    if let Ok(output) = hidden_command("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let name = line
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"');
            if name.is_empty() {
                continue;
            }
            let clean = clean_app_label(name);
            if clean.is_empty() {
                continue;
            }
            // Running processes are the lowest-priority source (priority 0).
            push_app(
                clean,
                name.to_string(),
                String::new(),
                0,
                &mut apps,
                &mut seen,
            );
        }
    }
    apps.sort_by(|a, b| a.label.cmp(&b.label));
    apps
}

#[cfg(not(windows))]
fn enumerate_installed_apps() -> Vec<AppEntry> {
    Vec::new()
}
#[cfg(not(windows))]
pub(super) fn enumerate_running_apps() -> Vec<AppEntry> {
    Vec::new()
}

pub(super) fn installed_apps() -> &'static [AppEntry] {
    static APPS: std::sync::OnceLock<Vec<AppEntry>> = std::sync::OnceLock::new();
    APPS.get_or_init(enumerate_installed_apps)
}

impl Editor {
    /* ------------------------- left panel ------------------------- */

    pub(super) fn left_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(SPACE_SM);
        ui_label(ui, "APP LIBRARY");
        ui.add_space(SPACE_XS);
        if ui
            .add(
                Button::new("Add app to workspace")
                    .icon(lucide::PLUS)
                    .variant(ButtonVariant::Outline),
            )
            .clicked()
        {
            self.picker_open = true;
        }
        ui.add_space(SPACE_SM);
        ui.add(
            Input::new(&mut self.lib_search)
                .hint("Search your apps...")
                .max_width(f32::INFINITY),
        );

        ui.add_space(SPACE_SM);
        let q = self.lib_search.to_lowercase();
        let apps = installed_apps();
        // Reserve room at the bottom for the Profiles panel so it is never pushed off.
        let lib_h = (ui.available_height() - PROFILES_H).max(LIB_MIN_H);
        ScrollArea::vertical()
            .id_salt("lib_scroll")
            .auto_shrink([false, false])
            .max_height(lib_h)
            .show(ui, |ui| {
                if apps.is_empty() {
                    ui.label(egui::RichText::new("No apps found - use Browse...").weak());
                }
                for app in apps {
                    if !app.label.to_lowercase().contains(&q)
                        && !app.exe.to_lowercase().contains(&q)
                    {
                        continue;
                    }
                    let resp = ui.add(
                        Item::new()
                            .icon(lucide::APP_WINDOW)
                            .title(app.label.as_str())
                            .description(app.exe.as_str())
                            .size(ItemSize::Sm)
                            .sense(egui::Sense::click_and_drag())
                            .id_salt(("lib_row", app.label.as_str())),
                    );
                    resp.dnd_set_drag_payload(app.clone());
                    if resp.double_clicked() {
                        self.attach_app_to_selected(
                            app.label.clone(),
                            app.exe.clone(),
                            app.path.clone(),
                        );
                    }
                }
            });

        // Profiles
        ui.add_space(SPACE_SM);
        layout::space_between(
            ui,
            |ui| {
                ui_label(ui, "PROFILES");
            },
            |ui| {
                if ui
                    .add(
                        Button::new("New profile")
                            .icon(lucide::PLUS)
                            .size(ButtonSize::Icon)
                            .variant(ButtonVariant::Ghost),
                    )
                    .on_hover_text("New profile")
                    .clicked()
                {
                    self.text_dialog = Some(TextDialogState {
                        kind: TextDialogKind::NewProfile,
                        value: String::new(),
                    });
                }
                if ui
                    .add(
                        Button::new("Duplicate profile")
                            .icon(lucide::COPY)
                            .size(ButtonSize::Icon)
                            .variant(ButtonVariant::Ghost),
                    )
                    .on_hover_text("Duplicate profile")
                    .clicked()
                {
                    self.duplicate_profile();
                }
                if ui
                    .add(
                        Button::new("Rename profile")
                            .icon(lucide::PENCIL)
                            .size(ButtonSize::Icon)
                            .variant(ButtonVariant::Ghost),
                    )
                    .on_hover_text("Rename profile")
                    .clicked()
                {
                    self.text_dialog = Some(TextDialogState {
                        kind: TextDialogKind::RenameProfile,
                        value: self.cfg().active_profile.clone(),
                    });
                }
                if ui
                    .add(
                        Button::new("Delete profile")
                            .icon(lucide::TRASH)
                            .size(ButtonSize::Icon)
                            .variant(ButtonVariant::Ghost),
                    )
                    .on_hover_text("Delete profile")
                    .clicked()
                {
                    self.delete_profile();
                }
            },
        );
        ui.add_space(SPACE_XS);
        let active = self.cfg().active_profile.clone();
        let names: Vec<String> = self.cfg().profiles.keys().cloned().collect();
        ScrollArea::vertical()
            .id_salt("prof_scroll")
            .max_height(PROFILES_H)
            .show(ui, |ui| {
                for name in names {
                    let on = name == active;
                    if ui
                        .add(
                            Button::new(name.as_str())
                                .variant(if on {
                                    ButtonVariant::Secondary
                                } else {
                                    ButtonVariant::Ghost
                                })
                                .size(ButtonSize::Sm),
                        )
                        .clicked()
                    {
                        self.cfg_mut().active_profile = name.clone();
                        self.selected_zone =
                            self.cfg().active().zones.first().map(|z| z.id).unwrap_or(0);
                        self.fields_zone = -1;
                        self.push_history();
                    }
                }
            });

        // Tools dock at the bottom
        ui.add_space(SPACE_SM);
        separator(ui);
        ui.add_space(SPACE_XS);
        ui_label(ui, "TOOLS");
        ui.add_space(SPACE_XS);
        layout::row(ui, SPACE_SM, |ui| {
            let canvas_on = self.view == View::Canvas;
            if ui
                .add(
                    Button::new("Canvas")
                        .icon(lucide::MONITOR)
                        .size(ButtonSize::Icon)
                        .variant(if canvas_on {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        }),
                )
                .on_hover_text("Canvas")
                .clicked()
            {
                self.view = View::Canvas;
                self.cancel_merge();
            }
            let hotkeys_on = self.view == View::Hotkeys;
            if ui
                .add(
                    Button::new("Hotkeys")
                        .icon(lucide::KEYBOARD)
                        .size(ButtonSize::Icon)
                        .variant(if hotkeys_on {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        }),
                )
                .on_hover_text("Hotkeys")
                .clicked()
            {
                self.view = View::Hotkeys;
                self.cancel_merge();
            }
            let settings_on = self.view == View::Settings;
            if ui
                .add(
                    Button::new("Settings")
                        .icon(lucide::SETTINGS)
                        .size(ButtonSize::Icon)
                        .variant(if settings_on {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        }),
                )
                .on_hover_text("Settings")
                .clicked()
            {
                self.view = View::Settings;
                self.cancel_merge();
            }
            if ui
                .add(
                    Button::new("Reset demo")
                        .icon(lucide::REFRESH_CW)
                        .size(ButtonSize::Icon)
                        .variant(ButtonVariant::Ghost),
                )
                .on_hover_text("Reset Demo")
                .clicked()
            {
                self.reset_confirm_open = true;
            }
            if ui
                .add(
                    Button::new("Help")
                        .icon(lucide::CIRCLE_HELP)
                        .size(ButtonSize::Icon)
                        .variant(ButtonVariant::Ghost),
                )
                .on_hover_text("Help")
                .clicked()
            {
                self.help_open = true;
            }
        });
    }
}
