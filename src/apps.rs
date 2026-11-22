//! Runtime application discovery (§5.4).
//!
//! The library is never persisted: it is enumerated from Start Menu shortcuts,
//! registry Uninstall keys and `tasklist`. This is a pure Rust module (no UI
//! dependency) so the command worker can call it directly for `/api/apps`.
//!
//! Identity rule (path-based resolution plan, Decisions 1–2): an app's `exe`
//! is always derived from a *verified full path*, never from a display label.
//! Running processes supply that path via `QueryFullProcessImageNameW`;
//! installed shortcuts via `IShellLinkW::GetPath`; registry entries only when
//! `DisplayIcon` / `InstallLocation` point at an existing `.exe`.

use std::collections::HashMap;

/// A discovered application: display label, verified exe name and full path
/// when known. `path` is empty only when the OS refused to disclose the image
/// path (running processes) — never because we guessed from the label.
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

/// Parse one line of `tasklist /FO CSV /NH` into unquoted fields.
///
/// `tasklist` emits `"Image Name","PID","Session Name","Session#","Mem Usage"`.
/// A naive `split(',')` breaks on names or memory values containing commas, so
/// this small parser honours quotes and doubled quotes. The PID is field 1.
fn parse_csv_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    // Doubled quote inside a quoted field.
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(field.trim().to_string());
                field.clear();
            }
            _ => field.push(c),
        }
    }
    fields.push(field.trim().to_string());
    fields
}

/// File name (with extension) from a Windows or POSIX path. This is the only
/// source of `AppEntry.exe` — never a display label.
fn exe_from_path(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Expand `%VAR%` references in a `REG_EXPAND_SZ` value. Bounded iterations
/// avoid looping forever if a variable expands to a string containing itself.
fn expand_env_vars(input: &str) -> String {
    let mut out = input.to_string();
    for _ in 0..4 {
        let mut changed = false;
        let mut next = String::with_capacity(out.len());
        let mut i = 0;
        while i < out.len() {
            if out.as_bytes()[i] == b'%' {
                if let Some(end) = out[i + 1..].find('%') {
                    let name = &out[i + 1..i + 1 + end];
                    if !name.is_empty() {
                        if let Ok(value) = std::env::var(name) {
                            next.push_str(&value);
                            i += end + 2;
                            changed = true;
                            continue;
                        }
                    }
                }
            }
            let ch = out[i..].chars().next().unwrap();
            next.push(ch);
            i += ch.len_utf8();
        }
        out = next;
        if !changed {
            break;
        }
    }
    out
}

/// Strip a trailing icon index (`,0`) and surrounding quotes from a
/// `DisplayIcon` value. A comma followed by non-digits is part of the path and
/// is left alone.
fn normalize_icon_path(raw: &str) -> String {
    let mut s = raw.trim().trim_matches('"').trim().to_string();
    if let Some((base, index)) = s.rsplit_once(',') {
        if !index.trim().is_empty() && index.trim().chars().all(|c| c.is_ascii_digit()) {
            s = base.trim().trim_matches('"').trim().to_string();
        }
    }
    s
}

/// Accept a registry / shortcut candidate only when it names an existing
/// `.exe`. Returns `(exe file name, full path)`. The existence check is
/// injected so the rule is unit-testable without touching the real filesystem.
fn verified_exe<F: Fn(&str) -> bool>(candidate: &str, exists: F) -> Option<(String, String)> {
    let path = expand_env_vars(&normalize_icon_path(candidate));
    if path.is_empty() || !path.to_ascii_lowercase().ends_with(".exe") {
        return None;
    }
    if !exists(&path) {
        return None;
    }
    let exe = exe_from_path(&path);
    if exe.is_empty() {
        return None;
    }
    Some((exe, path))
}

/// Insert or replace an entry. A verified full path always beats an entry
/// without one, regardless of label length or source priority (Decision 2).
fn push_app(
    label: String,
    exe: String,
    path: String,
    priority: u8,
    apps: &mut Vec<AppEntry>,
    seen: &mut SeenMap,
) {
    let clean = clean_label(&label);
    // Never fabricate an exe from a label: an entry without an exe is useless
    // for resolution and must be dropped rather than guessed.
    if clean.is_empty() || is_junk(&clean) || exe.is_empty() {
        return;
    }
    let key = dedup_key(&clean);
    if key.is_empty() {
        return;
    }
    match seen.get(&key) {
        Some(&(idx, old_prio)) => {
            let old_has_path = !apps[idx].path.is_empty();
            let new_has_path = !path.is_empty();
            let better = if new_has_path != old_has_path {
                new_has_path
            } else {
                clean.len() < apps[idx].label.len()
                    || (clean.len() == apps[idx].label.len() && priority > old_prio)
            };
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

/// Resolve a Start Menu `.lnk` to its target with `IShellLinkW::GetPath`.
/// COM is initialized and uninitialized in matched pairs on this thread; the
/// wide path buffer is NUL-terminated and lives for the whole call.
#[cfg(windows)]
fn resolve_lnk_target(lnk: &std::path::Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_ALL,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    let wide: Vec<u16> = lnk
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `CoInitializeEx` is paired with `CoUninitialize` on every path.
    // `wide` is a live NUL-terminated UTF-16 string; `buf` is a local, aligned
    // UTF-16 buffer whose length is passed to `GetPath`. No pointer escapes.
    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if !init.is_ok() {
            return None;
        }
        let result = (|| {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL).ok()?;
            // Load the .lnk file into the ShellLink object. `SetPath` would set
            // the *target*, not read the shortcut — a classic mix-up.
            let file: IPersistFile = link.cast().ok()?;
            file.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;
            let mut buf = [0u16; 2048];
            link.GetPath(&mut buf, std::ptr::null_mut(), 0).ok()?;
            let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let target = String::from_utf16_lossy(&buf[..n]);
            (!target.is_empty()).then_some(target)
        })();
        CoUninitialize();
        result
    }
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
            // Resolve the real target; a shortcut stem is a label, not an exe.
            let Some(target) = resolve_lnk_target(&path) else {
                continue;
            };
            let Some((exe, full)) = verified_exe(&target, |p| std::path::Path::new(p).is_file())
            else {
                continue;
            };
            let label = clean_label(&exe);
            if label.is_empty() {
                continue;
            }
            push_app(label, exe, full, 2, apps, seen);
        }
    }
}

/// Read one registry value for every subkey of `key`, keyed by the full subkey
/// path. `reg query /s /v <name>` prints the key path before each matching
/// value, which lets us associate `DisplayIcon` / `InstallLocation` with the
/// right `DisplayName` instead of guessing from the label.
#[cfg(windows)]
fn reg_values(key: &str, value_name: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(output) = hidden_command("reg")
        .args(["query", key, "/s", "/v", value_name])
        .output()
    else {
        return out;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_key = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("HKEY_") {
            current_key = trimmed.to_string();
            continue;
        }
        let Some((_, after)) = line.split_once(value_name) else {
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
        let value = value.trim().to_string();
        if !value.is_empty() && !current_key.is_empty() {
            out.insert(current_key.clone(), value);
        }
    }
    out
}

#[cfg(windows)]
fn scan_uninstall_key(key: &str, apps: &mut Vec<AppEntry>, seen: &mut SeenMap) {
    let names = reg_values(key, "DisplayName");
    let icons = reg_values(key, "DisplayIcon");
    let locations = reg_values(key, "InstallLocation");
    for (subkey, name) in names {
        let label = clean_label(&name);
        if label.is_empty() {
            continue;
        }
        // Only an existing .exe is a fact. If neither value yields one, skip
        // the entry entirely — Running and Browse sources still cover it.
        let verified = icons
            .get(&subkey)
            .and_then(|icon| verified_exe(icon, |p| std::path::Path::new(p).is_file()))
            .or_else(|| {
                locations
                    .get(&subkey)
                    .and_then(|loc| verified_exe(loc, |p| std::path::Path::new(p).is_file()))
            });
        let Some((exe, path)) = verified else {
            continue;
        };
        push_app(label, exe, path, 1, apps, seen);
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

#[cfg(windows)]
struct RunningProcess {
    /// Image name from `tasklist` (used only when the path lookup fails).
    name: String,
    /// Authoritative full image path, empty when the OS refused the lookup.
    path: String,
}

/// Raw `tasklist` → PID → `QueryFullProcessImageNameW` pipeline. No label
/// filtering or dedup: every process identity is preserved for resolution.
#[cfg(windows)]
fn enumerate_running_processes() -> Vec<RunningProcess> {
    let mut out = Vec::new();
    let Ok(output) = hidden_command("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
    else {
        return out;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let fields = parse_csv_fields(line);
        if fields.len() < 2 {
            continue;
        }
        let name = fields[0].trim();
        if name.is_empty() {
            continue;
        }
        let Ok(pid) = fields[1].trim().parse::<u32>() else {
            continue;
        };
        // Reuse the engine's single Win32 implementation (Decision 1).
        let path = crate::engine::process_path(pid).unwrap_or_default();
        out.push(RunningProcess {
            name: name.to_string(),
            path,
        });
    }
    out
}

/// Enumerate currently running processes via `tasklist /FO CSV /NH`.
pub fn enumerate_running_apps() -> Vec<AppEntry> {
    #[cfg(windows)]
    {
        let mut apps: Vec<AppEntry> = Vec::new();
        let mut seen: SeenMap = SeenMap::new();
        for p in enumerate_running_processes() {
            let (exe, path) = if p.path.is_empty() {
                // Lookup failed: keep it selectable with the process name as
                // exe; runtime resolution will retry on a fresh enumeration.
                (p.name.clone(), String::new())
            } else {
                (exe_from_path(&p.path), p.path)
            };
            let clean = clean_label(&exe);
            if clean.is_empty() {
                continue;
            }
            push_app(clean, exe, path, 0, &mut apps, &mut seen);
        }
        apps.sort_by(|a, b| a.label.cmp(&b.label));
        apps
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Full image paths of all running processes, without label filtering or
/// dedup. Used by the engine for runtime exe-name → path resolution
/// (Decision 5). This is resolution input, not window matching.
pub fn running_process_image_paths() -> Vec<String> {
    #[cfg(windows)]
    {
        enumerate_running_processes()
            .into_iter()
            .filter(|p| !p.path.is_empty())
            .map(|p| p.path)
            .collect()
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

    #[test]
    fn parse_csv_fields_reads_pid_from_quoted_tasklist_row() {
        let line = r#""chrome.exe","4242","Console","1","1,234 K""#;
        let fields = parse_csv_fields(line);
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "chrome.exe");
        assert_eq!(fields[1], "4242");
        assert_eq!(fields[4], "1,234 K");
        assert_eq!(fields[1].parse::<u32>().unwrap(), 4242);
    }

    #[test]
    fn parse_csv_fields_handles_commas_and_doubled_quotes_in_names() {
        let line = r#""App, ""Special"".exe","7","Console","1","100 K""#;
        let fields = parse_csv_fields(line);
        assert_eq!(fields[0], r#"App, "Special".exe"#);
        assert_eq!(fields[1], "7");
    }

    #[test]
    fn pid_resolves_to_full_image_path() {
        let pid = std::process::id();
        let path = crate::engine::process_path(pid).expect("own process path must resolve");
        assert!(!path.is_empty());
        assert!(
            exe_from_path(&path)
                .to_ascii_lowercase()
                .contains("screenlayout"),
            "unexpected image path: {path}"
        );
    }

    #[test]
    fn exe_is_derived_from_path_not_label() {
        assert_eq!(exe_from_path(r"C:\Program Files\Vendor\App.exe"), "App.exe");
        assert_eq!(exe_from_path("C:/Tools/tool.EXE"), "tool.EXE");
        assert_eq!(exe_from_path(""), "");
    }

    #[test]
    fn normalize_icon_path_strips_trailing_index_and_quotes() {
        assert_eq!(
            normalize_icon_path(r#""C:\Program Files\App\app.exe",0"#),
            r"C:\Program Files\App\app.exe"
        );
        assert_eq!(
            normalize_icon_path(r"C:\App, Inc\app.exe"),
            r"C:\App, Inc\app.exe"
        );
    }

    #[test]
    fn verified_exe_rejects_unverifiable_candidates() {
        let exists = |_p: &str| true;
        assert!(verified_exe("Notepad", exists).is_none()); // no .exe suffix
        assert!(verified_exe(r"C:\Windows\System32\imageres.dll,0", exists).is_none());
        assert!(verified_exe("", exists).is_none());
    }

    #[test]
    fn verified_exe_accepts_only_existing_exe() {
        let candidate = r"C:\Program Files\Vendor\App.exe,0";
        let missing = verified_exe(candidate, |_p| false);
        assert!(missing.is_none(), "a non-existent path must be skipped");
        let found = verified_exe(candidate, |p| p.ends_with("App.exe"));
        assert_eq!(
            found,
            Some((
                "App.exe".to_string(),
                r"C:\Program Files\Vendor\App.exe".to_string()
            ))
        );
    }

    #[test]
    fn dedup_prefers_path_bearing_entry_regardless_of_label_or_priority() {
        let mut apps = Vec::new();
        let mut seen = SeenMap::new();
        // Same dedup key, high priority, but no verified path.
        push_app(
            "App".into(),
            "app.exe".into(),
            String::new(),
            9,
            &mut apps,
            &mut seen,
        );
        // Same key, lower priority, but a real path — must win.
        push_app(
            "APP".into(),
            "real.exe".into(),
            r"C:\Program Files\Vendor\real.exe".into(),
            1,
            &mut apps,
            &mut seen,
        );
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].exe, "real.exe");
        assert_eq!(apps[0].path, r"C:\Program Files\Vendor\real.exe");
    }

    #[test]
    fn push_app_never_guesses_exe_from_label() {
        let mut apps = Vec::new();
        let mut seen = SeenMap::new();
        push_app(
            "Mystery App".into(),
            String::new(),
            String::new(),
            1,
            &mut apps,
            &mut seen,
        );
        assert!(
            apps.is_empty(),
            "an entry without a verified exe must be skipped"
        );
    }

    #[cfg(windows)]
    #[test]
    fn lnk_target_resolves_via_shell_link() {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::{Interface, PCWSTR};
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_ALL,
            COINIT_APARTMENTTHREADED,
        };
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

        let to_wide =
            |s: &OsStr| -> Vec<u16> { s.encode_wide().chain(std::iter::once(0)).collect() };
        let dir = std::env::temp_dir().join(format!("zoneflow-lnk-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let lnk = dir.join("App.lnk");
        let target = r"C:\Program Files\Vendor\App.exe";

        // SAFETY: matched COM init/uninit; all wide buffers are NUL-terminated
        // and live for the duration of their calls.
        unsafe {
            let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            assert!(init.is_ok(), "COM init failed in test thread");
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL).unwrap();
            let target_wide = to_wide(OsStr::new(target));
            assert!(link.SetPath(PCWSTR(target_wide.as_ptr())).is_ok());
            let file: IPersistFile = link.cast().unwrap();
            let lnk_wide = to_wide(lnk.as_os_str());
            assert!(file.Save(PCWSTR(lnk_wide.as_ptr()), true).is_ok());
            CoUninitialize();
        }

        let resolved = resolve_lnk_target(&lnk);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            resolved
                .as_ref()
                .is_some_and(|p| p.eq_ignore_ascii_case(target)),
            "unexpected .lnk target: {resolved:?}"
        );
    }
}
