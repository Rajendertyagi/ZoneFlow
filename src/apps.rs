//! Runtime application discovery (§5.4).
//!
//! The library is never persisted: it is enumerated from Start Menu shortcuts,
//! registry Uninstall keys and `tasklist`. This is a pure Rust module (no UI
//! dependency) so the command worker can call it directly. The legacy egui
//! `editor::rail` keeps its own copy until Phase 6 removes that frontend.

use std::collections::HashMap;

/// A discovered application: display label, best-effort exe name and full
/// path when known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppEntry {
    pub label: String,
    pub exe: String,
    pub path: String,
}

type SeenMap = HashMap<String, (usize, u8)>;

const JUNK_PHRASES: &[&str] = &[
    "uninstall",
    "help",
    "documentation",
    "readme",
    "update for",
    "microsoft visual c++",
    "redistributable",
];

const JUNK_WORDS: &[&str] = &[
    "uninstall",
    "help",
    "documentation",
    "readme",
    "setup",
    "installer",
    "license",
    "support",
];

fn is_version_token(tok: &str) -> bool {
    let mut seen_digit = false;
    let mut seen_dot = false;
    for c in tok.chars() {
        if c.is_ascii_digit() {
            seen_digit = true;
        } else if c == '.' {
            seen_dot = true;
        } else {
            return false;
        }
    }
    seen_digit && (seen_dot || tok.len() >= 3)
}

fn strip_parentheticals(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0u32;
    for ch in s.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn clean_label(raw: &str) -> String {
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

fn is_junk(label: &str) -> bool {
    let low = label.to_lowercase();
    if JUNK_PHRASES.iter().any(|p| low.contains(*p)) {
        return true;
    }
    let tokens: Vec<&str> = low
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '+' && c != '.')
        .filter(|t| !t.is_empty())
        .collect();
    JUNK_WORDS.iter().any(|w| tokens.contains(w))
}

fn dedup_key(clean: &str) -> String {
    clean
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_app(
    label: String,
    exe: String,
    path: String,
    priority: u8,
    apps: &mut Vec<AppEntry>,
    seen: &mut SeenMap,
) {
    let clean = clean_label(&label);
    if clean.is_empty() || is_junk(&clean) {
        return;
    }
    let key = dedup_key(&clean);
    if key.is_empty() {
        return;
    }
    let exe = if exe.is_empty() {
        format!("{clean}.exe")
    } else {
        exe
    };
    match seen.get(&key) {
        Some(&(idx, old_prio)) => {
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

#[cfg(windows)]
fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

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
            let clean = clean_label(stem);
            if clean.is_empty() {
                continue;
            }
            push_app(
                clean.clone(),
                format!("{clean}.exe"),
                String::new(),
                2,
                apps,
                seen,
            );
        }
    }
}

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

/// Enumerate installed applications (cached by the caller; this function is
/// intentionally uncached so tests and the worker control freshness).
pub fn enumerate_installed_apps() -> Vec<AppEntry> {
    #[cfg(windows)]
    {
        let mut apps: Vec<AppEntry> = Vec::new();
        let mut seen: SeenMap = SeenMap::new();
        let mut roots: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(appdata) = std::env::var("APPDATA") {
            roots.push(
                std::path::PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"),
            );
        }
        if let Ok(programdata) = std::env::var("ProgramData") {
            roots.push(
                std::path::PathBuf::from(programdata)
                    .join(r"Microsoft\Windows\Start Menu\Programs"),
            );
        }
        for root in &roots {
            scan_lnk_dir(root, 0, &mut apps, &mut seen);
        }
        for key in [
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        ] {
            scan_uninstall_key(key, &mut apps, &mut seen);
        }
        apps.sort_by(|a, b| a.label.cmp(&b.label));
        let cap = crate::config::loaded_settings().app_list_cap;
        apps.truncate(cap);
        apps
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Enumerate currently running processes via `tasklist /FO CSV /NH`.
pub fn enumerate_running_apps() -> Vec<AppEntry> {
    #[cfg(windows)]
    {
        let mut apps: Vec<AppEntry> = Vec::new();
        let mut seen: SeenMap = SeenMap::new();
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
                let clean = clean_label(name);
                if clean.is_empty() {
                    continue;
                }
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
    {
        Vec::new()
    }
}

/// Rust-side native file picker for `POST /api/apps/browse`. Returns at most
/// one entry for the picked executable. `rfd` handles its own STA threading on
/// Windows, so this is safe to call from the worker thread.
pub fn browse() -> Vec<AppEntry> {
    let Some(path) = rfd::FileDialog::new().pick_file() else {
        return Vec::new();
    };
    let label = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Application")
        .to_string();
    let exe = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("app.exe")
        .to_string();
    vec![AppEntry {
        label: clean_label(&label),
        exe,
        path: path.to_string_lossy().into_owned(),
    }]
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_label_strips_exe_parentheticals_and_versions() {
        assert_eq!(clean_label("  App Name (64-bit) 1.2.3.exe "), "App Name");
        assert_eq!(clean_label("Firefox.exe"), "Firefox");
    }

    #[test]
    fn junk_filter_rejects_installers_and_help() {
        assert!(is_junk("App Uninstaller"));
        assert!(is_junk("Help"));
        assert!(!is_junk("Visual Studio Code"));
    }

    #[test]
    fn push_app_deduplicates_and_prefers_shorter_label() {
        let mut apps = Vec::new();
        let mut seen = SeenMap::new();
        push_app(
            "Chrome".into(),
            "chrome.exe".into(),
            String::new(),
            1,
            &mut apps,
            &mut seen,
        );
        push_app(
            "CHROME".into(),
            "chrome2.exe".into(),
            String::new(),
            2,
            &mut apps,
            &mut seen,
        );
        assert_eq!(apps.len(), 1);
        // Equal-length labels: the higher-priority source wins.
        assert_eq!(apps[0].exe, "chrome2.exe");
    }

    #[test]
    fn enumeration_returns_a_sorted_list() {
        let installed = enumerate_installed_apps();
        assert!(installed.windows(2).all(|w| w[0].label <= w[1].label));
        let running = enumerate_running_apps();
        assert!(running.windows(2).all(|w| w[0].label <= w[1].label));
    }
}
