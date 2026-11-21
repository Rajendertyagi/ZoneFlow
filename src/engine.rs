use crate::config::{AppConfig, TabBehavior, Zone};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::process::CommandExt;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS,
    DWMWA_WINDOW_CORNER_PREFERENCE,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, SetWindowPos,
    ShowWindow, SystemParametersInfoW, WindowFromPoint, GA_ROOT, GWL_EXSTYLE, SET_WINDOW_POS_FLAGS,
    SPI_GETWORKAREA, SW_MAXIMIZE, SW_RESTORE, WS_EX_TOOLWINDOW,
};

// Pure geometry lives in `engine_core`. Re-exporting keeps the exact `engine::*`
// API that `editor.rs` and `overlay.rs` already call.
use crate::engine_core::{
    border_offsets, compensate_rect, duplicate_app_assignments, keys_to_events, launch_target,
    text_to_events, KeyEvent,
};
pub use crate::engine_core::{compute_zones, zone_at, zone_by_id, zone_pixels, Rect};

/// Default number of times a window position is applied to work around DWM's
/// async snap. `move_window` keeps this default so its public signature stays
/// unchanged for `editor.rs`; callers that have a config can use
/// [`move_window_with_retries`] with `settings.dwm_snap_retries` instead.
const DWM_SNAP_RETRIES: u32 = 2;

/// Default minimum window size (px) used when no config is available. Callers
/// that have a config pass `settings.min_window_px` to
/// [`find_window_by_path_min`] / [`execute_zone_with_config`] instead.
const DEFAULT_MIN_WINDOW_PX: i32 = 150;

/// Work area of the primary monitor (excludes taskbar). Nothing hardcoded.
pub fn work_area() -> Rect {
    // SAFETY: `SystemParametersInfoW` is called with `SPI_GETWORKAREA` and a pointer
    // to a local, correctly aligned `RECT` that stays alive for the whole call. The
    // `pvParam` buffer is only written to (never read) and `fWinIni` is zero, so no
    // user profile is persisted. No pointer escapes this block.
    unsafe {
        let mut r = RECT::default();
        let ok = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut r as *mut _ as *mut _),
            Default::default(),
        );
        if ok.is_ok() && (r.right - r.left) > 0 {
            Rect {
                x: r.left,
                y: r.top,
                w: r.right - r.left,
                h: r.bottom - r.top,
            }
        } else {
            Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1040,
            }
        }
    }
}

fn invisible_borders(hwnd: HWND) -> (i32, i32, i32, i32) {
    // SAFETY: `hwnd` is a caller-supplied window handle used only for the duration
    // of these synchronous calls. Both output `RECT`s are local, aligned, and live
    // for the whole call; the DWM attribute id and byte size match `RECT`. No
    // pointers escape this block and the handle is not used after it returns.
    unsafe {
        let mut win = RECT::default();
        if GetWindowRect(hwnd, &mut win).is_err() {
            return (0, 0, 0, 0);
        }
        let mut dwm = RECT::default();
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut dwm as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
        .is_err()
        {
            return (0, 0, 0, 0);
        }
        border_offsets(
            Rect {
                x: win.left,
                y: win.top,
                w: win.right - win.left,
                h: win.bottom - win.top,
            },
            Rect {
                x: dwm.left,
                y: dwm.top,
                w: dwm.right - dwm.left,
                h: dwm.bottom - dwm.top,
            },
        )
    }
}

pub fn move_window_raw(raw: isize, zone: Rect) {
    move_window(HWND(raw as *mut _), zone);
}

pub fn move_window(hwnd: HWND, zone: Rect) {
    move_window_with_retries(hwnd, zone, DWM_SNAP_RETRIES);
}

/// Move `hwnd` into `zone`, applying the position `retries` times to work
/// around DWM's asynchronous snap. Pass `settings.dwm_snap_retries` here when a
/// config is available; [`move_window`] is the fixed-default wrapper kept for
/// `editor.rs` compatibility.
///
/// # Safety contract
/// `hwnd` must be a valid window handle for the duration of the call. The
/// function performs only synchronous Win32 calls with self-contained, local
/// buffers (see the inline `SAFETY` notes).
pub fn move_window_with_retries(hwnd: HWND, zone: Rect, retries: u32) {
    // SAFETY: `hwnd` is valid for the duration of the calls. `pref` is a local
    // `i32` whose address and size match the DWORD expected by
    // `DWMWA_WINDOW_CORNER_PREFERENCE`. `SetWindowPos` receives no borrowed
    // pointers. All calls are synchronous and self-contained.
    unsafe {
        let was_visible = ShowWindow(hwnd, SW_RESTORE);
        if !was_visible.as_bool() {
            tracing::debug!("ShowWindow(SW_RESTORE) reported the window was previously hidden");
        }
        let mut pref: i32 = 1; // DWMWCP_DONOTROUND
        if let Err(e) = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &mut pref as *mut _ as *mut _,
            std::mem::size_of::<i32>() as u32,
        ) {
            tracing::warn!(error = %e, "DwmSetWindowAttribute(corner preference) failed");
        }
        let (l, t, r, b) = invisible_borders(hwnd);
        let target = compensate_rect(zone, (l, t, r, b));
        for _ in 0..retries {
            if let Err(e) = SetWindowPos(
                hwnd,
                None,
                target.x,
                target.y,
                target.w,
                target.h,
                SET_WINDOW_POS_FLAGS(0),
            ) {
                tracing::warn!(error = %e, "SetWindowPos failed while snapping window");
            }
        }
    }
}

fn process_path(pid: u32) -> Option<String> {
    // SAFETY: `OpenProcess` is checked with `.ok()?`, so `h` is a valid handle that
    // we close exactly once. `buf` is a 1024-element `u16` array; its pointer and
    // the capacity in `len` are passed to `QueryFullProcessImageNameW`, which on
    // success writes at most `len` UTF-16 units plus a NUL terminator and reports
    // the character count (excluding NUL) back in `len`. We clamp the read slice
    // to the buffer length, so `buf[..n]` is always in bounds.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let res = QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        if let Err(e) = windows::Win32::Foundation::CloseHandle(h) {
            tracing::warn!(error = %e, "CloseHandle failed after process path lookup");
        }
        if res.is_err() {
            return None;
        }
        let n = (len as usize).min(buf.len());
        Some(
            OsString::from_wide(&buf[..n])
                .to_string_lossy()
                .into_owned(),
        )
    }
}

fn window_title(hwnd: HWND) -> String {
    // SAFETY: `hwnd` is valid for the call. `buf` is a 512-element `u16` array;
    // `GetWindowTextW` copies at most 511 characters plus a NUL terminator and
    // returns the character count (excluding NUL). We clamp the slice to the
    // buffer length, so `buf[..n]` is always in bounds.
    unsafe {
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, &mut buf);
        let n = (n as usize).min(buf.len());
        String::from_utf16_lossy(&buf[..n])
    }
}

/// Default-threshold helper kept for callers without a config. The config-driven
/// path uses [`is_real_window_min`] directly with `settings.min_window_px`.
#[allow(dead_code)]
fn is_real_window(hwnd: HWND) -> bool {
    is_real_window_min(hwnd, DEFAULT_MIN_WINDOW_PX)
}

/// Whether `hwnd` is a visible, non-tool window larger than `min_px` on both
/// axes. `is_real_window` keeps the historical 150 px default so existing
/// callers are unchanged; pass `settings.min_window_px` here when a config is
/// available.
///
/// # Safety contract
/// `hwnd` must be a valid window handle for the duration of the call.
pub fn is_real_window_min(hwnd: HWND, min_px: i32) -> bool {
    // SAFETY: `hwnd` is valid for the duration of the calls. `r` is a local,
    // correctly aligned `RECT` whose address is valid for `GetWindowRect`.
    // `IsWindowVisible` and `GetWindowLongPtrW` take no caller pointers. All
    // calls are synchronous and no pointer escapes this block.
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if (ex & WS_EX_TOOLWINDOW.0 as isize) != 0 {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(hwnd, &mut r).is_err() {
            return false;
        }
        (r.right - r.left) > min_px && (r.bottom - r.top) > min_px
    }
}

/// Normalize a Windows path for case-insensitive comparison: trim surrounding
/// whitespace, convert `/` separators to `\`, and lowercase. Used for the AHK
/// `matchType: path` parity (§2c / T3).
fn normalize_path(path: &str) -> String {
    path.trim().replace('/', "\\").to_lowercase()
}

struct FindCtx {
    /// True when the caller supplied a non-empty full path to match.
    path_requested: bool,
    /// Lowercased exe name to match (always used as the fallback).
    target_exe: String,
    /// Normalized full path to prefer when `path_requested` is true.
    target_path: String,
    /// Minimum real-window size in pixels (from `settings.min_window_px`).
    min_px: i32,
    /// Exact path match (or, in exe-only mode, the first exe-name match).
    found: Option<HWND>,
    /// First exe-name match seen while searching for a better path match.
    exe_match: Option<HWND>,
}

// SAFETY: this callback is only ever handed to `EnumWindows` by
// `find_window_by_path_min`, which invokes it on the calling thread with a valid
// `HWND` and with `lparam` set to the address of a live `FindCtx`. We dereference
// that pointer only while the callback runs, before the caller returns and drops
// the context. Returning `BOOL(1)` continues enumeration; `BOOL(0)` stops it.
// Any other caller must uphold the same `lparam` contract.
unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut FindCtx);
    if !is_real_window_min(hwnd, ctx.min_px) {
        return BOOL(1);
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let Some(path) = process_path(pid) else {
        return BOOL(1);
    };
    let exe = path.rsplit('\\').next().unwrap_or("").to_lowercase();
    if exe == ctx.target_exe {
        if !ctx.path_requested {
            // Legacy exe-only behaviour: first match wins, stop enumerating.
            ctx.found = Some(hwnd);
            return BOOL(0);
        }
        // Path mode: remember the first exe-name match as a fallback but keep
        // looking for an exact full-path match.
        if ctx.exe_match.is_none() {
            ctx.exe_match = Some(hwnd);
        }
    }
    if ctx.path_requested && normalize_path(&path) == ctx.target_path {
        ctx.found = Some(hwnd);
        return BOOL(0);
    }
    BOOL(1)
}

/// Find a window by exe name, optionally preferring an exact full-path match.
///
/// When `path` is non-empty the enumeration continues past exe-name matches so
/// a window whose full process path matches (case-insensitive, `/` and `\`
/// normalized) wins; the first exe-name match is kept as a fallback. When `path`
/// is empty this is identical to [`find_window`]: the first exe-name match wins.
///
/// Public compatibility wrapper; config-aware callers use
/// [`find_window_by_path_min`] so `settings.min_window_px` is honoured.
#[allow(dead_code)]
pub fn find_window_by_path(exe: &str, path: &str) -> Option<HWND> {
    find_window_by_path_min(exe, path, DEFAULT_MIN_WINDOW_PX)
}

/// Config-driven variant of [`find_window_by_path`] using `min_px` as the
/// minimum real-window size (from `settings.min_window_px`).
pub fn find_window_by_path_min(exe: &str, path: &str, min_px: i32) -> Option<HWND> {
    let mut ctx = FindCtx {
        path_requested: !path.is_empty(),
        target_exe: exe.to_lowercase(),
        target_path: normalize_path(path),
        min_px,
        found: None,
        exe_match: None,
    };
    // SAFETY: `ctx` is a live stack local for the whole call. Its address is
    // passed as `LPARAM` to `EnumWindows`, whose callback casts it back to
    // `&mut FindCtx`. `EnumWindows` is synchronous, so the pointer is valid for
    // every callback invocation and `ctx` is not moved or dropped until it returns.
    unsafe {
        if let Err(e) = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut _ as isize)) {
            tracing::warn!(error = %e, "EnumWindows failed during window lookup");
        }
    }
    ctx.found.or(ctx.exe_match)
}

/// Public compatibility wrapper (exe-only lookup). Kept unchanged per the
/// engine API contract; config-aware callers use [`find_window_by_path_min`].
#[allow(dead_code)]
pub fn find_window(exe: &str) -> Option<HWND> {
    find_window_by_path(exe, "")
}

pub fn focus_window(hwnd: HWND) {
    // SAFETY: `hwnd` is valid for the duration of both synchronous calls and
    // neither function takes a caller-owned pointer that outlives the block.
    unsafe {
        if !ShowWindow(hwnd, SW_RESTORE).as_bool() {
            tracing::debug!("ShowWindow(SW_RESTORE) reported the window was previously hidden");
        }
        if !SetForegroundWindow(hwnd).as_bool() {
            tracing::debug!("SetForegroundWindow was refused (Windows focus-stealing prevention)");
        }
    }
}

/// Execute one zone with the engine defaults. Kept for callers without a config;
/// prefer [`execute_zone_with_config`] so `dwm_snap_retries`, `min_window_px` and
/// path-based matching are all driven by settings (§2.2 / T3 / T4).
///
/// Public compatibility wrapper; UI callers use [`execute_zone_with_config`].
#[allow(dead_code)]
pub fn execute_zone(zone: &Zone, rect: Rect) -> Result<String, String> {
    execute_zone_inner(zone, rect, DWM_SNAP_RETRIES, DEFAULT_MIN_WINDOW_PX)
}

/// Execute one zone using the caller's config for DWM snap retries and the
/// minimum window size. Window lookup honours `zone.path` (AHK `matchType: path`).
pub fn execute_zone_with_config(
    zone: &Zone,
    rect: Rect,
    cfg: &AppConfig,
) -> Result<String, String> {
    execute_zone_inner(
        zone,
        rect,
        cfg.settings.dwm_snap_retries,
        cfg.settings.min_window_px,
    )
}

fn execute_zone_inner(
    zone: &Zone,
    rect: Rect,
    retries: u32,
    min_px: i32,
) -> Result<String, String> {
    let mut hwnd = find_window_by_path_min(&zone.exe, &zone.path, min_px);
    if hwnd.is_none() {
        if matches!(zone.launch_mode, crate::config::LaunchMode::Attach) {
            return Err(format!("{} not running (attach mode)", zone.exe));
        }
        // Launch the exact stored path when present (AHK parity), falling back
        // to the exe name resolved via PATH. CREATE_NO_WINDOW avoids a console
        // flash for console-subsystem executables.
        let target = launch_target(zone);
        let mut cmd = std::process::Command::new(target);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        if cmd.spawn().is_err() {
            return Err(format!("failed to launch {}", target));
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            if let Some(h) = find_window_by_path_min(&zone.exe, &zone.path, min_px) {
                hwnd = Some(h);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let Some(h) = hwnd else {
        return Err(format!("{} window not found", zone.exe));
    };
    std::thread::sleep(std::time::Duration::from_millis(200));
    move_window_with_retries(h, rect, retries);
    focus_window(h);

    let title = window_title(h);
    // URL / tab rule
    if !zone.url.is_empty() && !matches!(zone.tab_behavior, TabBehavior::Ignore) {
        let need_tab = !zone.title_match.is_empty() && !title.contains(&zone.title_match);
        if need_tab || zone.title_match.is_empty() {
            let keys = match zone.tab_behavior {
                TabBehavior::ReuseBlank => "^l",
                _ => "^t",
            };
            send_keys(keys);
            std::thread::sleep(std::time::Duration::from_millis(150));
            send_text(&zone.url);
            send_keys("{ENTER}");
        }
    }
    // Macro
    if zone.macro_enabled && !zone.macro_text.is_empty() {
        let ok = zone.trigger_title.is_empty() || title.contains(&zone.trigger_title);
        if ok {
            std::thread::sleep(std::time::Duration::from_millis(zone.key_delay as u64));
            send_keys(&zone.macro_text);
        }
    }
    Ok(format!(
        "snapped {} to {},{} {}x{}",
        zone.exe, rect.x, rect.y, rect.w, rect.h
    ))
}

fn send_keys(keys: &str) {
    // Pure token mapping lives in engine_core (unit-tested); this function only
    // turns the resulting KeyEvents into SendInput calls.
    for event in keys_to_events(keys) {
        inject_event(event);
    }
}

fn send_text(text: &str) {
    // Unicode injection: preserves case, punctuation and non-ASCII, and is
    // independent of the keyboard layout. Replaces the broken ASCII-as-VK path.
    for event in text_to_events(text) {
        inject_event(event);
    }
}

fn inject_event(event: KeyEvent) {
    match event {
        KeyEvent::Unicode(unit) => send_unicode(unit),
        KeyEvent::Vk(vk) => tap(vk),
        KeyEvent::Chord(mod_vk, key_vk) => chord(mod_vk, key_vk),
    }
}

/// Inject one UTF-16 code unit via KEYEVENTF_UNICODE (down + up).
fn send_unicode(unit: u16) {
    // SAFETY: `input` is a local, correctly initialized keyboard INPUT with
    // wVk = 0 and wScan = the code unit; KEYEVENTF_UNICODE makes Windows read
    // wScan. The one-element slice is contiguous and lives for the call.
    unsafe {
        let size = std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32;
        let mut input = windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
            r#type: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
            ..Default::default()
        };
        input.Anonymous.ki.wVk = windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0);
        input.Anonymous.ki.wScan = unit;
        input.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE;
        if windows::Win32::UI::Input::KeyboardAndMouse::SendInput(&[input], size) == 0 {
            tracing::warn!("SendInput failed to inject Unicode key-down");
        }
        input.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
        if windows::Win32::UI::Input::KeyboardAndMouse::SendInput(&[input], size) == 0 {
            tracing::warn!("SendInput failed to inject Unicode key-up");
        }
    }
}

fn tap(vk: u16) {
    // SAFETY: `input` is a local, correctly initialized `INPUT` with type
    // `INPUT_KEYBOARD` and a valid `wVk`. `&[input]` is a contiguous one-element
    // slice that lives for the call, and `size` is `size_of::<INPUT>()`, matching
    // what `SendInput` expects for `cbsize`.
    unsafe {
        let mut input = windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
            r#type: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
            ..Default::default()
        };
        input.Anonymous.ki.wVk = windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk);
        let size = std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32;
        let sent = windows::Win32::UI::Input::KeyboardAndMouse::SendInput(&[input], size);
        if sent == 0 {
            tracing::warn!("SendInput failed to inject key-down event");
        }
        input.Anonymous.ki.dwFlags = windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP;
        let sent = windows::Win32::UI::Input::KeyboardAndMouse::SendInput(&[input], size);
        if sent == 0 {
            tracing::warn!("SendInput failed to inject key-up event");
        }
    }
}

fn chord(mod_vk: u16, key_vk: u16) {
    // SAFETY: all four `INPUT` structs are local, correctly initialized keyboard
    // events; the slice `&[down, keydown, keyup, modup]` is contiguous and lives
    // for the call, and `size` is `size_of::<INPUT>()`. `SendInput` is called once
    // with a consistent array/size pair.
    unsafe {
        let size = std::mem::size_of::<windows::Win32::UI::Input::KeyboardAndMouse::INPUT>() as i32;
        let mut down = windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
            r#type: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
            ..Default::default()
        };
        down.Anonymous.ki.wVk = windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(mod_vk);
        let mut keydown = windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
            r#type: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD,
            ..Default::default()
        };
        keydown.Anonymous.ki.wVk = windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(key_vk);
        let mut keyup = keydown;
        keyup.Anonymous.ki.dwFlags = windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP;
        let mut modup = down;
        modup.Anonymous.ki.dwFlags = windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP;
        let sent = windows::Win32::UI::Input::KeyboardAndMouse::SendInput(
            &[down, keydown, keyup, modup],
            size,
        );
        if sent == 0 {
            tracing::warn!("SendInput failed to inject chord events");
        }
    }
}

pub fn retile(cfg: &AppConfig) {
    let area = work_area();
    let zones = compute_zones(cfg.active(), area, cfg.gap);
    let retries = cfg.settings.dwm_snap_retries;
    let min_px = cfg.settings.min_window_px;
    // Report ambiguous duplicate assignments rather than silently moving the
    // same HWND twice (last zone wins). No behavior change — warning only.
    for (app, ids) in duplicate_app_assignments(cfg.active()) {
        tracing::warn!(
            app = %app,
            zone_ids = ?ids,
            "app assigned to multiple zones in one profile; only one window can be placed, last zone wins"
        );
    }
    for z in cfg.active().zones.iter() {
        if let Some(r) = zone_by_id(&zones, z.id) {
            if let Some(h) = find_window_by_path_min(&z.exe, &z.path, min_px) {
                move_window_with_retries(h, r, retries);
            }
        }
    }
}

pub fn maximize_active() {
    // SAFETY: `GetForegroundWindow` returns a valid `HWND` or null (checked below);
    // `ShowWindow` takes no caller pointers and the handle is used only here.
    unsafe {
        let h = GetForegroundWindow();
        if !h.0.is_null() && !ShowWindow(h, SW_MAXIMIZE).as_bool() {
            tracing::debug!("ShowWindow(SW_MAXIMIZE) reported the window was previously hidden");
        }
    }
}

pub fn restore_active(cfg: &AppConfig) {
    // SAFETY: the foreground `HWND` is null-checked before use; `work_area` and
    // `compute_zones` are safe Rust; `ShowWindow` takes no borrowed pointers.
    // All Win32 calls are synchronous and self-contained.
    unsafe {
        let h = GetForegroundWindow();
        if h.0.is_null() {
            return;
        }
        let area = work_area();
        let zones = compute_zones(cfg.active(), area, cfg.gap);
        for z in cfg.active().zones.iter() {
            if find_window_by_path_min(&z.exe, &z.path, cfg.settings.min_window_px) == Some(h) {
                if let Some(r) = zone_by_id(&zones, z.id) {
                    move_window_with_retries(h, r, cfg.settings.dwm_snap_retries);
                    return;
                }
            }
        }
        if !ShowWindow(h, SW_RESTORE).as_bool() {
            tracing::debug!("ShowWindow(SW_RESTORE) reported the window was previously hidden");
        }
    }
}

pub fn cycle_zone(cfg: &AppConfig, forward: bool) {
    // SAFETY: the foreground `HWND` is null-checked; `r` is a local, aligned `RECT`
    // passed by address to `GetWindowRect` and read only after a successful call.
    // `move_window`'s Win32 use is documented at its own site. No pointer escapes.
    unsafe {
        let h = GetForegroundWindow();
        if h.0.is_null() {
            return;
        }
        let area = work_area();
        let zones = compute_zones(cfg.active(), area, cfg.gap);
        if zones.is_empty() {
            return;
        }
        let mut r = RECT::default();
        if GetWindowRect(h, &mut r).is_err() {
            return;
        }
        let cx = (r.left + r.right) / 2;
        let cy = (r.top + r.bottom) / 2;
        let current = zones
            .iter()
            .position(|(_, z)| cx >= z.x && cx < z.x + z.w && cy >= z.y && cy < z.y + z.h);
        let next = match current {
            Some(i) if forward => (i + 1) % zones.len(),
            Some(0) => zones.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        move_window_with_retries(h, zones[next].1, cfg.settings.dwm_snap_retries);
    }
}

/// Move the active window one zone over along `(dx, dy)` in the zone grid
/// (§2c): `(-1, 0)` left, `(1, 0)` right, `(0, -1)` up, `(0, 1)` down.
///
/// Unlike [`cycle_zone`], which walks the zone list linearly, this uses the
/// zones' percentage rectangles to find the nearest zone in the requested
/// direction, so Up/Down change rows and Left/Right change columns. The
/// foreground window must currently sit inside one of the computed zones;
/// otherwise the request is ignored.
pub fn cycle_zone_axis(cfg: &AppConfig, dx: i32, dy: i32) {
    // SAFETY: the foreground `HWND` is null-checked; `r` is a local, aligned
    // `RECT` passed by address to `GetWindowRect` and read only after a
    // successful call. `move_window`'s Win32 use is documented at its own site.
    // No pointer escapes this block.
    unsafe {
        let h = GetForegroundWindow();
        if h.0.is_null() {
            return;
        }
        let area = work_area();
        let zones = compute_zones(cfg.active(), area, cfg.gap);
        if zones.is_empty() {
            return;
        }
        let mut r = RECT::default();
        if GetWindowRect(h, &mut r).is_err() {
            return;
        }
        let cx = (r.left + r.right) / 2;
        let cy = (r.top + r.bottom) / 2;
        let current = zones
            .iter()
            .position(|(_, z)| cx >= z.x && cx < z.x + z.w && cy >= z.y && cy < z.y + z.h);
        let Some(current_idx) = current else {
            return;
        };
        // `compute_zones` maps over `profile.zones` in order, so indices align.
        let profile_zones = &cfg.active().zones;
        if let Some(next_idx) =
            crate::engine_core::nearest_zone_in_direction(profile_zones, current_idx, dx, dy)
        {
            if let Some(rect) = zones.get(next_idx).map(|(_, rect)| *rect) {
                move_window_with_retries(h, rect, cfg.settings.dwm_snap_retries);
            }
        }
    }
}

#[allow(dead_code)] // consumed by the retained egui editor until Phase 6
pub fn drag_keys_held() -> bool {
    // SAFETY: `GetAsyncKeyState` takes a virtual-key code by value and returns an
    // `i16`; it reads no caller memory and has no preconditions beyond a valid VK
    // constant (0x11 = Ctrl, 0x01 = left mouse button).
    unsafe {
        let ctrl = (GetAsyncKeyState(0x11) as u16 & 0x8000) != 0;
        let lbtn = (GetAsyncKeyState(0x01) as u16 & 0x8000) != 0;
        ctrl && lbtn
    }
}

#[allow(dead_code)] // consumed by the retained egui editor until Phase 6
pub fn cursor_pos() -> (i32, i32) {
    // SAFETY: `p` is a local, aligned `POINT` whose address is valid for the
    // duration of `GetCursorPos`; the API writes only to it and no pointer escapes.
    unsafe {
        let mut p = POINT::default();
        if let Err(e) = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut p) {
            tracing::warn!(error = %e, "GetCursorPos failed; using default cursor position");
        }
        (p.x, p.y)
    }
}

pub fn window_at(x: i32, y: i32) -> Option<HWND> {
    // SAFETY: `WindowFromPoint` takes a `POINT` by value and returns an `HWND`;
    // `GetAncestor` walks that handle to its root with a valid flag. Neither
    // reads caller memory. Handles are null-checked before use. Walking to the
    // GA_ROOT fixes bug B3: WindowFromPoint can return a child control, and
    // moving that child instead of the top-level app window displaces a subcontrol.
    unsafe {
        let h = WindowFromPoint(POINT { x, y });
        if h.0.is_null() {
            return None;
        }
        let root = GetAncestor(h, GA_ROOT);
        Some(if root.0.is_null() { h } else { root })
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_path;

    #[test]
    fn normalize_path_is_case_insensitive_and_slash_agnostic() {
        // AHK `matchType: path` parity (T3): `/` and `\` are equivalent and
        // comparison ignores case and surrounding whitespace.
        assert_eq!(
            normalize_path(r"C:/Program Files/App/app.EXE"),
            normalize_path(r"c:\program files\app\app.exe")
        );
        assert_eq!(normalize_path("  C:\\App\\A.exe  "), "c:\\app\\a.exe");
    }
}
