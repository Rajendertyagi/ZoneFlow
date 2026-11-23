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
use windows::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_KEYUP, KEYEVENTF_UNICODE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, SetWindowPos,
    ShowWindow, SystemParametersInfoW, WindowFromPoint, GA_ROOT, GWL_EXSTYLE, SET_WINDOW_POS_FLAGS,
    SPI_GETWORKAREA, SW_MAXIMIZE, SW_RESTORE, WS_EX_TOOLWINDOW,
};

// Pure geometry lives in `engine_core`. Re-exporting keeps the exact `engine::*`
// API that the API layer and native loop call.
use crate::engine_core::{
    border_offsets, compensate_rect, decide_attribution, describe_windows,
    duplicate_app_assignments, filter_job_windows, keys_to_events, launch_spec, launch_target,
    text_to_events, validate_script_path, AttributionDecision, CandidateWindow, KeyEvent,
    LaunchSpec,
};
pub use crate::engine_core::{compute_zones, zone_by_id, Rect};

/// Default number of times a window position is applied to work around DWM's
/// async snap. `move_window` keeps this default so its public signature stays
/// unchanged for compatibility; callers that have a config can use
/// [`move_window_with_retries`] with `settings.dwm_snap_retries` instead.
const DWM_SNAP_RETRIES: u32 = 2;

/// Default minimum window size (px) used when no config is available. Callers
/// that have a config pass `settings.min_window_px` to
/// [`find_window_by_paths_min`] / [`execute_zone_with_config`] instead.
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
/// API compatibility.
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

/// Full image path of a process, via `OpenProcess` +
/// `QueryFullProcessImageNameW`. `pub(crate)` so app enumeration can reuse the
/// single Win32 implementation instead of duplicating it (Decision 1).
pub(crate) fn process_path(pid: u32) -> Option<String> {
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

/// Context for `EnumWindows`: strict full-path candidates only. There is no
/// exe-name or title comparison here — window identity is established solely
/// by the owning process's full image path (Decision 4).
struct FindCtx {
    /// Normalized candidate full paths (case/separator normalized once).
    candidates: Vec<String>,
    /// Minimum real-window size in pixels (from `settings.min_window_px`).
    min_px: i32,
    /// First matching window handle.
    found: Option<HWND>,
}

// SAFETY: this callback is only ever handed to `EnumWindows` by
// `find_window_by_paths_min`, which invokes it on the calling thread with a valid
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
    if path_matches(&path, &ctx.candidates) {
        ctx.found = Some(hwnd);
        return BOOL(0);
    }
    BOOL(1)
}

/// Pure strict-path predicate: does `process_path` equal any normalized
/// candidate? No exe-name or title fallback exists by design (Decision 4).
fn path_matches(process_path: &str, normalized_candidates: &[String]) -> bool {
    let normalized = normalize_path(process_path);
    normalized_candidates.iter().any(|c| c == &normalized)
}

/// Find a window whose owning process image path exactly matches one of
/// `candidates` (case-insensitive, `/` and `\` normalized). `min_px` is the
/// minimum real-window size from `settings.min_window_px`.
pub fn find_window_by_paths_min(candidates: &[String], min_px: i32) -> Option<HWND> {
    if candidates.is_empty() {
        return None;
    }
    let normalized: Vec<String> = candidates.iter().map(|c| normalize_path(c)).collect();
    let mut ctx = FindCtx {
        candidates: normalized,
        min_px,
        found: None,
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
    ctx.found
}

/// Context for `EnumWindows` when collecting every eligible top-level window
/// for launcher attribution. No job filtering happens in the callback — the
/// pure [`filter_job_windows`] applies the authoritative PID set afterwards so
/// that rule is unit-testable with an injected PID→windows map.
struct JobWindowCtx {
    min_px: i32,
    windows: Vec<CandidateWindow>,
}

// SAFETY: this callback is only handed to `EnumWindows` by
// `enumerate_real_windows`, which passes the address of a live `JobWindowCtx`
// as `lparam`. The pointer is dereferenced only while the synchronous callback
// runs, before the caller drops the context. Returning `BOOL(1)` continues
// enumeration.
unsafe extern "system" fn enum_job_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam.0 as *mut JobWindowCtx);
    if !is_real_window_min(hwnd, ctx.min_px) {
        return BOOL(1);
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    let mut class_buf = [0u16; 256];
    let n = GetClassNameW(hwnd, &mut class_buf);
    let class = String::from_utf16_lossy(&class_buf[..(n as usize).min(class_buf.len())]);
    let process_path = process_path(pid).unwrap_or_default();

    ctx.windows.push(CandidateWindow {
        hwnd: hwnd.0 as isize,
        pid,
        class,
        process_path,
        title: window_title(hwnd),
    });
    BOOL(1)
}

/// Enumerate every eligible top-level window with the facts needed for job
/// attribution (PID, class, process path, title). Filtering by the job's PID
/// set is deliberately left to the pure [`filter_job_windows`] helper.
pub fn enumerate_real_windows(min_px: i32) -> Vec<CandidateWindow> {
    let mut ctx = JobWindowCtx {
        min_px,
        windows: Vec::new(),
    };
    // SAFETY: `ctx` is a live stack local for the whole call. Its address is
    // passed as `LPARAM` to `EnumWindows`, whose callback casts it back to
    // `&mut JobWindowCtx`. `EnumWindows` is synchronous, so the pointer is valid
    // for every callback invocation and `ctx` is not dropped until it returns.
    unsafe {
        if let Err(e) = EnumWindows(
            Some(enum_job_window_proc),
            LPARAM(&mut ctx as *mut _ as isize),
        ) {
            tracing::warn!(error = %e, "EnumWindows failed during launcher attribution");
        }
    }
    ctx.windows
}

/// Where a resolved candidate path came from. Recorded for diagnostics and
/// tests; it never affects window matching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionSource {
    ExactConfig,
    RunningProcess,
    LaunchedProcess,
}

/// Resolution of a zone's assignment to candidate full paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZoneResolution {
    /// Both `exe` and `path` are empty — nothing assigned.
    Unassigned,
    /// `exe` is set but no full path could be obtained in attach mode.
    Unresolved,
    /// A pathless legacy assignment matched MORE THAN ONE distinct full path.
    /// We refuse to pick an arbitrary installation (Finding 2). This is a
    /// resolution-time ambiguity, not a window-matching fallback: strict
    /// full-path matching is unchanged.
    Ambiguous { paths: Vec<String> },
    /// One or more candidate full paths, with the source they came from.
    Resolved {
        candidates: Vec<String>,
        source: ResolutionSource,
    },
}

/// Why a zone could not be arranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneFailure {
    Unassigned,
    Unresolved,
    /// A pathless legacy assignment resolved to multiple distinct installations.
    AmbiguousPath,
    /// A launcher script produced more than one eligible application window.
    AmbiguousWindow,
    NotRunning,
    NoEligibleWindow,
    LaunchFailed,
    /// Invalid launcher script path / command interpreter / job setup.
    LauncherFailed,
}

/// Outcome of executing one zone. `launched` is true only when this call
/// actually spawned a process — never a statement of intent. It is carried on
/// **both** variants so a launch that produced no eligible window is still
/// recorded as having launched (Decision 6/7: accurate per-zone outcomes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZoneExecution {
    Snapped {
        message: String,
        launched: bool,
    },
    Failed {
        failure: ZoneFailure,
        message: String,
        launched: bool,
    },
}

impl ZoneExecution {
    pub fn message(&self) -> &str {
        match self {
            Self::Snapped { message, .. } | Self::Failed { message, .. } => message,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Snapped { .. })
    }

    /// Whether this zone's execution actually spawned a process, regardless of
    /// whether it ended in success or failure.
    pub fn launched(&self) -> bool {
        match self {
            Self::Snapped { launched, .. } | Self::Failed { launched, .. } => *launched,
        }
    }

    /// Test-only accessor so outcome assertions can name the failure variant
    /// without destructuring the whole enum in every test.
    #[cfg(test)]
    pub fn failure_for_test(&self) -> Option<ZoneFailure> {
        match self {
            Self::Failed { failure, .. } => Some(*failure),
            Self::Snapped { .. } => None,
        }
    }
}

/// Build the failure outcome for "no eligible window found". Pure so the
/// `launched` flag can be pinned down by a test without spawning a real process.
fn no_window_execution(failure: ZoneFailure, message: String, launched: bool) -> ZoneExecution {
    ZoneExecution::Failed {
        failure,
        message,
        launched,
    }
}

/// Resolve a zone's assignment to candidate full paths from config and the
/// supplied running-process paths. **This is resolution, not window matching:**
/// the exe basename is compared only to obtain candidate full paths from
/// process identity. Window identity is always established by strict full-path
/// comparison in [`find_window_by_paths_min`]. No `SearchPathW` is used — its
/// registry-dependent search order can disagree with `CreateProcess`.
pub fn resolve_zone_paths(zone: &Zone, running_paths: &[String]) -> ZoneResolution {
    if !zone.path.trim().is_empty() {
        return ZoneResolution::Resolved {
            candidates: vec![zone.path.trim().to_string()],
            source: ResolutionSource::ExactConfig,
        };
    }
    if zone.exe.trim().is_empty() {
        return ZoneResolution::Unassigned;
    }
    let target = zone.exe.trim();
    // Collect distinct full paths (case/separator normalized) whose exe
    // basename matches. The basename comparison is only used to obtain
    // candidate paths from process identity — window identity is still decided
    // by strict full-path comparison in `find_window_by_paths_min`.
    let mut seen: Vec<String> = Vec::new();
    let mut candidates: Vec<String> = Vec::new();
    for path in running_paths.iter().filter(|p| {
        p.rsplit(['\\', '/'])
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case(target))
    }) {
        let normalized = normalize_path(path);
        if !seen.contains(&normalized) {
            seen.push(normalized);
            candidates.push(path.clone());
        }
    }
    match candidates.len() {
        0 => ZoneResolution::Unresolved,
        // A pathless legacy assignment with several distinct installations is
        // ambiguous. Do NOT silently pick the first one — report the paths so
        // the user can re-assign with an exact path (Finding 2).
        _ if candidates.len() > 1 => ZoneResolution::Ambiguous { paths: candidates },
        _ => ZoneResolution::Resolved {
            candidates,
            source: ResolutionSource::RunningProcess,
        },
    }
}

/// Build a `LaunchedProcess` resolution from a spawned child's PID. The path
/// returned by `QueryFullProcessImageNameW` is the exact image `CreateProcess`
/// launched, by construction (Decision 3).
pub fn resolution_from_pid(pid: u32) -> Option<ZoneResolution> {
    process_path(pid).map(|path| ZoneResolution::Resolved {
        candidates: vec![path],
        source: ResolutionSource::LaunchedProcess,
    })
}

/// Decide the failure when no eligible window was found for resolved paths.
fn no_window_failure(process_still_running: bool) -> ZoneFailure {
    if process_still_running {
        ZoneFailure::NoEligibleWindow
    } else {
        ZoneFailure::NotRunning
    }
}

/// Pure launch gate for one zone, given the current running-path snapshot.
///
/// Launch mode spawns only when there is no running candidate. A zone explicitly
/// configured as Attach never launches (the UI promises exactly this), even
/// during Launch & Arrange. An ambiguous pathless assignment is never launched —
/// it is reported so the user can re-assign with an exact path.
///
/// Kept pure so the "two zones, one launch" behavior (Finding 1) is testable
/// without spawning a real process: callers simulate a spawn by feeding the
/// recorded path back into the snapshot via [`record_launched_path`].
fn zone_needs_launch(
    zone: &Zone,
    resolution: &ZoneResolution,
    launch: bool,
    running_paths: &[String],
) -> bool {
    if !launch || matches!(zone.launch_mode, crate::config::LaunchMode::Attach) {
        return false;
    }
    match resolution {
        ZoneResolution::Unassigned | ZoneResolution::Ambiguous { .. } => false,
        ZoneResolution::Unresolved => true,
        ZoneResolution::Resolved { candidates, .. } => {
            let normalized: Vec<String> = candidates.iter().map(|c| normalize_path(c)).collect();
            !running_paths
                .iter()
                .any(|p| normalized.contains(&normalize_path(p)))
        }
    }
}

/// Record a successfully launched process path in the running snapshot so
/// later zones in the same Launch & Arrange operation see it as running and do
/// not spawn a duplicate. Pure and idempotent (dedup by normalized path).
fn record_launched_path(snapshot: &mut Vec<String>, path: String) {
    let normalized = normalize_path(&path);
    if !snapshot.iter().any(|p| normalize_path(p) == normalized) {
        snapshot.push(path);
    }
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
pub fn execute_zone(zone: &Zone, rect: Rect) -> ZoneExecution {
    let mut running = crate::apps::running_process_image_paths();
    execute_zone_inner(
        zone,
        rect,
        DWM_SNAP_RETRIES,
        DEFAULT_MIN_WINDOW_PX,
        true,
        &mut running,
    )
}

/// Execute one zone using the caller's config for DWM snap retries and the
/// minimum window size. `launch=true` is Launch & Arrange (may spawn when no
/// running candidate exists); `launch=false` is Apply (never spawns).
pub fn execute_zone_with_config(
    zone: &Zone,
    rect: Rect,
    cfg: &AppConfig,
    launch: bool,
) -> ZoneExecution {
    let running = crate::apps::running_process_image_paths();
    execute_zone_with_context(zone, rect, cfg, launch, &running)
}

/// Variant that accepts a pre-enumerated running-process path snapshot. The
/// caller's snapshot is treated as immutable: any path launched by this zone is
/// recorded in a local copy that is discarded on return, so single-zone callers
/// (hotkeys, `execute_zone`, `execute_zone_with_config`) keep their behavior.
pub fn execute_zone_with_context(
    zone: &Zone,
    rect: Rect,
    cfg: &AppConfig,
    launch: bool,
    running_paths: &[String],
) -> ZoneExecution {
    let mut snapshot = running_paths.to_vec();
    execute_zone_with_mutable_context(zone, rect, cfg, launch, &mut snapshot)
}

/// Variant used by `apply_profile`: the running-path snapshot is mutable and
/// evolves as zones launch. After a zone spawns a process, its resolved full
/// path is pushed into the snapshot so subsequent zones in the same operation
/// see it as running and do NOT spawn a duplicate (Finding 1).
pub fn execute_zone_with_mutable_context(
    zone: &Zone,
    rect: Rect,
    cfg: &AppConfig,
    launch: bool,
    running_paths: &mut Vec<String>,
) -> ZoneExecution {
    execute_zone_inner(
        zone,
        rect,
        cfg.settings.dwm_snap_retries,
        cfg.settings.min_window_px,
        launch,
        running_paths,
    )
}

/// How long to poll for a launcher's application window (matches the existing
/// post-launch window poll).
const LAUNCH_WINDOW_POLL_SECS: u64 = 8;
const POLL_INTERVAL_MS: u64 = 100;
/// Once the first app window appears, keep collecting for this long so a script
/// that opens several apps is reported as ambiguous instead of snapping the
/// first one prematurely.
const AMBIGUITY_SETTLE_MS: u64 = 400;

/// Pure, Win32-free timing state for launcher window attribution.
///
/// The quiet period is measured from the most recent NEW unique window, not
/// from the first one. A script that opens a second app after the initial
/// settle window must keep the collection loop alive so the result is reported
/// as ambiguous instead of silently snapping the first window.
#[derive(Clone, Debug)]
struct AttributionTracker {
    last_new_window_at: Option<std::time::Instant>,
    settle: std::time::Duration,
}

impl AttributionTracker {
    fn new(settle_ms: u64) -> Self {
        Self {
            last_new_window_at: None,
            settle: std::time::Duration::from_millis(settle_ms),
        }
    }

    /// Record `new_count` newly discovered unique windows at `now`. A zero
    /// count (repeated enumeration of windows already seen) does not reset the
    /// quiet period.
    fn observe(&mut self, new_count: usize, now: std::time::Instant) {
        if new_count > 0 {
            self.last_new_window_at = Some(now);
        }
    }

    /// True once at least one window has been seen and the quiet period since
    /// the most recent new window has elapsed. A `now` earlier than the last
    /// observation (e.g. probing the old break point after a later window was
    /// discovered) is never settled.
    fn is_settled(&self, now: std::time::Instant) -> bool {
        match self.last_new_window_at {
            Some(t) => match now.checked_duration_since(t) {
                Some(elapsed) => elapsed >= self.settle,
                None => false,
            },
            None => false,
        }
    }
}

/// Pure loop-exit decision for the attribution poll. The overall `deadline`
/// always caps the wait; otherwise we stop once the quiet period has elapsed.
/// Kept Win32-free so the timeout behaviour is unit-testable without sleeping.
fn should_stop_attribution(
    now: std::time::Instant,
    deadline: std::time::Instant,
    tracker: &AttributionTracker,
) -> bool {
    now >= deadline || tracker.is_settled(now)
}

/// Pure outcome of a Job PID query. A failed query must be a truthful error,
/// never an empty PID set that would fall through to the EXE-override fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
enum JobQueryOutcome {
    /// Query succeeded (possibly with an empty PID set) — proceed to window
    /// filtering and the existing no-window/EXE-override logic.
    Proceed(Vec<u32>),
    /// Query failed — report the error; do not use the EXE override.
    Failed(String),
}

impl JobQueryOutcome {
    fn from_query_result(result: Result<Vec<u32>, String>) -> Self {
        match result {
            Ok(pids) => Self::Proceed(pids),
            Err(e) => Self::Failed(e),
        }
    }
}

/// Execute a zone that has a `.bat`/`.cmd` launcher configured.
///
/// The script is executed via `cmd.exe /c` inside a Job Object; the job's PID
/// list is the authoritative process tree used for window attribution. There is
/// no HWND snapshot, name/title matching or parent-PID heuristic. An explicit
/// `Zone.path` is only a strict window-matching fallback when the job produced
/// no eligible window — it is never launched instead of the script.
fn execute_launcher_zone(
    zone: &Zone,
    rect: Rect,
    retries: u32,
    min_px: i32,
    launch: bool,
) -> ZoneExecution {
    let launcher = zone.launcher_path.trim();
    let display_name = launcher.rsplit(['\\', '/']).next().unwrap_or(launcher);

    // Validate before any process is created. Rejecting CMD metacharacters here
    // (rather than trying to escape them) keeps the quoted argv element safe.
    if let Err(e) = validate_script_path(launcher) {
        return ZoneExecution::Failed {
            failure: ZoneFailure::LauncherFailed,
            message: format!("invalid launcher script '{launcher}': {}", e.message()),
            launched: false,
        };
    }

    // D1/D3: Apply and Attach mode never launch. With a launcher configured
    // that means we can only arrange via the explicit EXE override below.
    let may_launch = launch && !matches!(zone.launch_mode, crate::config::LaunchMode::Attach);

    let spawn = if may_launch {
        match crate::launcher::spawn_script(launcher, zone.launcher_show_console) {
            Ok(s) => Some(s),
            Err(e) => {
                return ZoneExecution::Failed {
                    failure: ZoneFailure::LauncherFailed,
                    message: format!("failed to launch script '{launcher}': {e}"),
                    launched: false,
                };
            }
        }
    } else {
        None
    };
    let launched = spawn.is_some();

    // Attribute windows from the job's authoritative PID set. We keep every
    // distinct HWND seen during the poll (plus a short quiet period measured
    // from the LAST new window) so a multi-app script is reported as ambiguous
    // rather than guessed. A failed PID query is a truthful error, never an
    // empty set that would trigger the EXE-override fallback.
    let mut seen: Vec<CandidateWindow> = Vec::new();
    if let Some(ref s) = spawn {
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(LAUNCH_WINDOW_POLL_SECS);
        let mut tracker = AttributionTracker::new(AMBIGUITY_SETTLE_MS);
        while std::time::Instant::now() < deadline {
            let pids = match JobQueryOutcome::from_query_result(crate::launcher::query_job_pids(
                s.job_handle(),
            )) {
                JobQueryOutcome::Proceed(pids) => pids,
                JobQueryOutcome::Failed(e) => {
                    return ZoneExecution::Failed {
                        failure: ZoneFailure::LauncherFailed,
                        message: format!("could not query launcher process tree: {e}"),
                        launched,
                    };
                }
            };
            let matched = filter_job_windows(&enumerate_real_windows(min_px), &pids, s.cmd_pid);
            let before = seen.len();
            for w in matched {
                if !seen.iter().any(|u| u.hwnd == w.hwnd) {
                    seen.push(w);
                }
            }
            let now = std::time::Instant::now();
            tracker.observe(seen.len() - before, now);
            if !seen.is_empty() && should_stop_attribution(now, deadline, &tracker) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));
        }
    }

    let hwnd = match decide_attribution(seen.clone()) {
        AttributionDecision::Single(w) => HWND(w.hwnd as *mut _),
        AttributionDecision::Ambiguous(windows) => {
            return ZoneExecution::Failed {
                failure: ZoneFailure::AmbiguousWindow,
                message: format!(
                    "launcher '{display_name}' produced {} eligible windows; set an EXE override to disambiguate: {}",
                    windows.len(),
                    describe_windows(&windows)
                ),
                launched,
            };
        }
        AttributionDecision::None => {
            // Optional EXE override: strict full-path window matching only.
            // It is NEVER launched in place of the script.
            if zone.path.trim().is_empty() {
                let message = if may_launch {
                    format!("launcher '{display_name}' produced no eligible window")
                } else {
                    format!(
                        "launcher '{display_name}' was not run (Apply/Attach) and no EXE override is configured to identify its window"
                    )
                };
                return ZoneExecution::Failed {
                    failure: ZoneFailure::NoEligibleWindow,
                    message,
                    launched,
                };
            }
            let candidates = vec![zone.path.trim().to_string()];
            match find_window_by_paths_min(&candidates, min_px) {
                Some(h) => h,
                None => {
                    return ZoneExecution::Failed {
                        failure: ZoneFailure::NoEligibleWindow,
                        message: format!(
                            "launcher '{display_name}' produced no eligible window and EXE override '{}' has no arrangeable window",
                            zone.path.trim()
                        ),
                        launched,
                    };
                }
            }
        }
    };

    arrange_and_apply(zone, hwnd, rect, retries, launched, display_name)
}

fn execute_zone_inner(
    zone: &Zone,
    rect: Rect,
    retries: u32,
    min_px: i32,
    launch: bool,
    running_paths: &mut Vec<String>,
) -> ZoneExecution {
    // Launch precedence: a configured launcher script is ALWAYS the thing we
    // execute. `Zone.exe` / `Zone.path` are never substituted for it — `path`
    // is used only as a strict window-identification override. Zones without a
    // launcher keep the legacy EXE behavior below, unchanged.
    if matches!(launch_spec(zone), LaunchSpec::Script(_)) {
        return execute_launcher_zone(zone, rect, retries, min_px, launch);
    }

    let mut resolution = resolve_zone_paths(zone, running_paths);
    let mut launched = false;

    // Decision 5 step 4: launch mode spawns only when there is no running
    // candidate. A zone explicitly configured as Attach never launches (the
    // UI promises exactly this), even during Launch & Arrange. `CreateProcess`
    // resolves the target (path or exe name); the spawned PID's image path is
    // the exact image it launched, by construction.
    let needs_launch = zone_needs_launch(zone, &resolution, launch, running_paths);

    if needs_launch {
        let target = launch_target(zone);
        let mut cmd = std::process::Command::new(target);
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let child = match cmd.spawn() {
            Ok(child) => child,
            Err(_) => {
                return ZoneExecution::Failed {
                    failure: ZoneFailure::LaunchFailed,
                    message: format!("failed to launch {}", target),
                    launched: false, // spawn failed — nothing was launched
                };
            }
        };
        // A process was spawned successfully. Record that immediately so every
        // later outcome (even a failed image-path lookup) reports it truthfully.
        launched = true;
        let pid = child.id();
        let Some(ZoneResolution::Resolved {
            candidates: launched_candidates,
            ..
        }) = resolution_from_pid(pid)
        else {
            return ZoneExecution::Failed {
                failure: ZoneFailure::LaunchFailed,
                message: format!("failed to resolve launched {}", target),
                launched,
            };
        };
        let path = launched_candidates
            .into_iter()
            .next()
            .expect("resolution_from_pid always yields exactly one candidate");
        // Finding 1: publish the launched path into the shared snapshot so the
        // next zone in this operation sees it as running and does not spawn a
        // duplicate. This happens immediately after a successful spawn + path
        // resolution, before any window lookup.
        record_launched_path(running_paths, path.clone());
        match &mut resolution {
            ZoneResolution::Resolved { candidates, source } => {
                if !candidates
                    .iter()
                    .any(|c| normalize_path(c) == normalize_path(&path))
                {
                    candidates.push(path);
                }
                *source = ResolutionSource::LaunchedProcess;
            }
            ZoneResolution::Unresolved => {
                resolution = ZoneResolution::Resolved {
                    candidates: vec![path],
                    source: ResolutionSource::LaunchedProcess,
                };
            }
            ZoneResolution::Unassigned | ZoneResolution::Ambiguous { .. } => {
                unreachable!("unassigned/ambiguous zones never launch")
            }
        }
    }

    let candidates = match resolution {
        ZoneResolution::Unassigned => {
            return ZoneExecution::Failed {
                failure: ZoneFailure::Unassigned,
                message: format!("zone {} has no application assigned", zone.id),
                launched,
            };
        }
        ZoneResolution::Unresolved => {
            return ZoneExecution::Failed {
                failure: ZoneFailure::Unresolved,
                message: format!("could not resolve {} to a full path", zone.exe),
                launched,
            };
        }
        ZoneResolution::Ambiguous { paths } => {
            return ZoneExecution::Failed {
                failure: ZoneFailure::AmbiguousPath,
                message: format!(
                    "{} matches multiple installations; re-assign with an exact path: {}",
                    zone.exe,
                    paths.join(", ")
                ),
                launched,
            };
        }
        ZoneResolution::Resolved { candidates, .. } => candidates,
    };

    let normalized: Vec<String> = candidates.iter().map(|c| normalize_path(c)).collect();
    let mut hwnd = find_window_by_paths_min(&normalized, min_px);
    if hwnd.is_none() && launched {
        // Preserve the 8-second post-launch poll, now by resolved path only.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            if let Some(h) = find_window_by_paths_min(&normalized, min_px) {
                hwnd = Some(h);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let Some(h) = hwnd else {
        // Distinguish "process not running" from "running but no arrangeable
        // window" with a fresh enumeration (the earlier snapshot may be stale
        // after a launch).
        let fresh = crate::apps::running_process_image_paths();
        let still_running = fresh
            .iter()
            .any(|p| normalized.contains(&normalize_path(p)));
        let failure = no_window_failure(still_running);
        let message = match failure {
            ZoneFailure::NotRunning if launch => {
                format!("{} is not running after launch", zone.exe)
            }
            ZoneFailure::NotRunning => format!("{} is not running (attach mode)", zone.exe),
            _ => format!(
                "{} has no arrangeable window (minimized/hidden/0x0)",
                zone.exe
            ),
        };
        return no_window_execution(failure, message, launched);
    };

    arrange_and_apply(zone, h, rect, retries, launched, &zone.exe)
}

/// Move `hwnd` into `rect`, focus it, then apply the zone's URL/tab and macro
/// rules. Shared by the EXE and launcher paths so both report the same honest
/// `Snapped` outcome. `display_name` is only used in the message (the launcher
/// basename for script zones, `zone.exe` otherwise); it is never an identity
/// signal.
fn arrange_and_apply(
    zone: &Zone,
    hwnd: HWND,
    rect: Rect,
    retries: u32,
    launched: bool,
    display_name: &str,
) -> ZoneExecution {
    std::thread::sleep(std::time::Duration::from_millis(200));
    move_window_with_retries(hwnd, rect, retries);
    focus_window(hwnd);

    let title = window_title(hwnd);
    // URL / tab rule (unchanged — `title_match` is never used for identity)
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
    // Macro (unchanged)
    if zone.macro_enabled && !zone.macro_text.is_empty() {
        let ok = zone.trigger_title.is_empty() || title.contains(&zone.trigger_title);
        if ok {
            std::thread::sleep(std::time::Duration::from_millis(zone.key_delay as u64));
            send_keys(&zone.macro_text);
        }
    }
    ZoneExecution::Snapped {
        message: format!(
            "snapped {} to {},{} {}x{}",
            display_name, rect.x, rect.y, rect.w, rect.h
        ),
        launched,
    }
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
    let running = crate::apps::running_process_image_paths();
    for z in cfg.active().zones.iter() {
        match resolve_zone_paths(z, &running) {
            ZoneResolution::Resolved { candidates, .. } => {
                if let Some(r) = zone_by_id(&zones, z.id) {
                    if let Some(h) = find_window_by_paths_min(&candidates, min_px) {
                        move_window_with_retries(h, r, retries);
                    }
                }
            }
            // Skipping is correct: picking one of several distinct
            // installations would be arbitrary. Warn so it is observable.
            ZoneResolution::Ambiguous { paths } => {
                tracing::warn!(
                    zone_id = z.id,
                    exe = %z.exe,
                    paths = ?paths,
                    "ambiguous zone skipped during retile; re-assign with an exact path"
                );
            }
            ZoneResolution::Unassigned | ZoneResolution::Unresolved => {}
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
        let running = crate::apps::running_process_image_paths();
        for z in cfg.active().zones.iter() {
            match resolve_zone_paths(z, &running) {
                ZoneResolution::Resolved { candidates, .. } => {
                    if find_window_by_paths_min(&candidates, cfg.settings.min_window_px) == Some(h)
                    {
                        if let Some(r) = zone_by_id(&zones, z.id) {
                            move_window_with_retries(h, r, cfg.settings.dwm_snap_retries);
                            return;
                        }
                    }
                }
                // Skipping is correct: picking one of several distinct
                // installations would be arbitrary. Warn so it is observable.
                ZoneResolution::Ambiguous { paths } => {
                    tracing::warn!(
                        zone_id = z.id,
                        exe = %z.exe,
                        paths = ?paths,
                        "ambiguous zone skipped during restore; re-assign with an exact path"
                    );
                }
                ZoneResolution::Unassigned | ZoneResolution::Unresolved => {}
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
    use super::*;

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

    #[test]
    fn strict_matching_accepts_full_path_only() {
        let candidates = vec![normalize_path(r"C:\Program Files\Vendor\app.exe")];
        assert!(path_matches(
            r"C:/Program Files/Vendor/APP.EXE",
            &candidates
        ));
        assert!(path_matches(
            r"  c:\program files\vendor\app.exe  ",
            &candidates
        ));
    }

    #[test]
    fn strict_matching_rejects_exe_name_only_and_title_only() {
        let candidates = vec![normalize_path(r"C:\Program Files\Vendor\app.exe")];
        // An exe basename is not a full path — no fallback (Decision 4).
        assert!(!path_matches("app.exe", &candidates));
        assert!(!path_matches(r"C:\Other\app.exe", &candidates));
        // A window title is never an identity signal.
        assert!(!path_matches("MarketHubTS - Helium", &candidates));
        assert!(!path_matches("", &candidates));
    }

    #[test]
    fn find_window_by_paths_min_short_circuits_on_empty_candidates() {
        assert!(find_window_by_paths_min(&[], DEFAULT_MIN_WINDOW_PX).is_none());
    }

    #[test]
    fn resolution_prefers_exact_config_path() {
        let mut zone = Zone::new(1, "App", "app.exe", 0.0, 0.0, 50.0, 100.0);
        zone.path = r"C:\Program Files\Vendor\app.exe".into();
        let running = vec![r"D:\Elsewhere\app.exe".to_string()];
        assert_eq!(
            resolve_zone_paths(&zone, &running),
            ZoneResolution::Resolved {
                candidates: vec![r"C:\Program Files\Vendor\app.exe".to_string()],
                source: ResolutionSource::ExactConfig,
            }
        );
    }

    #[test]
    fn resolution_is_unassigned_when_exe_and_path_are_empty() {
        let zone = Zone::new(1, "", "", 0.0, 0.0, 50.0, 100.0);
        assert_eq!(
            resolve_zone_paths(&zone, &[r"C:\App\app.exe".to_string()]),
            ZoneResolution::Unassigned
        );
    }

    #[test]
    fn resolution_collects_running_process_paths_by_exe_basename() {
        let zone = Zone::new(1, "Chrome", "CHROME.EXE", 0.0, 0.0, 50.0, 100.0);
        // A single matching installation still resolves normally.
        let running = vec![
            r"C:\Program Files\Google\Chrome\chrome.exe".to_string(),
            r"C:\Windows\System32\notepad.exe".to_string(),
        ];
        assert_eq!(
            resolve_zone_paths(&zone, &running),
            ZoneResolution::Resolved {
                candidates: vec![r"C:\Program Files\Google\Chrome\chrome.exe".to_string()],
                source: ResolutionSource::RunningProcess,
            }
        );
    }

    #[test]
    fn resolution_is_unresolved_without_a_running_process() {
        let zone = Zone::new(1, "Helium", "Helium.exe", 0.0, 0.0, 50.0, 100.0);
        let running = vec![r"C:\Program Files\Google\Chrome\chrome.exe".to_string()];
        assert_eq!(
            resolve_zone_paths(&zone, &running),
            ZoneResolution::Unresolved
        );
    }

    #[test]
    fn resolution_from_pid_uses_launched_process_source() {
        let pid = std::process::id();
        let resolution = resolution_from_pid(pid).expect("own process path must resolve");
        match resolution {
            ZoneResolution::Resolved { candidates, source } => {
                assert_eq!(source, ResolutionSource::LaunchedProcess);
                assert_eq!(candidates.len(), 1);
                assert!(candidates[0].to_ascii_lowercase().contains("screenlayout"));
            }
            other => panic!("unexpected resolution: {other:?}"),
        }
    }

    #[test]
    fn no_window_failure_distinguishes_running_from_not_running() {
        assert_eq!(no_window_failure(true), ZoneFailure::NoEligibleWindow);
        assert_eq!(no_window_failure(false), ZoneFailure::NotRunning);
    }

    #[test]
    fn attach_zone_never_spawns_even_during_launch_arrange() {
        let mut zone = Zone::new(
            1,
            "Missing",
            "zoneflow-no-such-app-xyz.exe",
            0.0,
            0.0,
            50.0,
            100.0,
        );
        zone.launch_mode = crate::config::LaunchMode::Attach;
        let cfg = crate::config::AppConfig::default();
        let execution = execute_zone_with_context(
            &zone,
            Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
            &cfg,
            true, // Launch & Arrange verb
            &[],  // no running processes
        );
        assert_eq!(
            execution,
            ZoneExecution::Failed {
                failure: ZoneFailure::Unresolved,
                message: "could not resolve zoneflow-no-such-app-xyz.exe to a full path".into(),
                launched: false,
            }
        );
    }

    #[test]
    fn zone_execution_exposes_message_success_and_launched() {
        let snapped = ZoneExecution::Snapped {
            message: "snapped app.exe to 0,0 100x100".into(),
            launched: true,
        };
        assert!(snapped.is_success());
        assert!(snapped.launched());
        assert_eq!(snapped.message(), "snapped app.exe to 0,0 100x100");
        let failed = ZoneExecution::Failed {
            failure: ZoneFailure::Unassigned,
            message: "zone 1 has no application assigned".into(),
            launched: false,
        };
        assert!(!failed.is_success());
        assert!(!failed.launched());
        assert_eq!(failed.message(), "zone 1 has no application assigned");
    }

    #[test]
    fn launched_then_no_window_failure_records_launched() {
        // A real spawn is impractical in a unit test; the pure constructor used
        // by execute_zone_inner pins the flag on the failure path.
        let execution = no_window_execution(
            ZoneFailure::NoEligibleWindow,
            "app.exe has no arrangeable window (minimized/hidden/0x0)".into(),
            true,
        );
        assert!(!execution.is_success());
        assert!(
            execution.launched(),
            "a spawned process must be recorded even when no window appeared"
        );
        assert_eq!(
            execution.message(),
            "app.exe has no arrangeable window (minimized/hidden/0x0)"
        );
    }

    /* ---------------- Finding 1: no duplicate launches in one operation --------------- */

    #[test]
    fn record_launched_path_is_idempotent_and_normalizes() {
        let mut snapshot = Vec::new();
        record_launched_path(&mut snapshot, r"C:\Program Files\Vendor\app.exe".into());
        // Same path with different separators/case must not be added twice.
        record_launched_path(&mut snapshot, r"c:/program files/vendor/APP.EXE".into());
        assert_eq!(snapshot.len(), 1);
    }

    #[test]
    fn two_zones_same_exe_launch_only_once_with_evolving_snapshot() {
        // Deterministic simulation of apply_profile's loop using the exact pure
        // helpers the real execution path uses — no real process is spawned.
        let zone1 = Zone::new(1, "App", "app.exe", 0.0, 0.0, 50.0, 100.0);
        let zone2 = Zone::new(2, "App", "app.exe", 50.0, 0.0, 50.0, 100.0);
        let mut snapshot: Vec<String> = Vec::new(); // app initially not running

        // Zone 1: no running candidate -> launch.
        let r1 = resolve_zone_paths(&zone1, &snapshot);
        assert_eq!(r1, ZoneResolution::Unresolved);
        assert!(zone_needs_launch(&zone1, &r1, true, &snapshot));
        // execute_zone_inner records the launched path into the shared snapshot.
        let launched = r"C:\Program Files\Vendor\app.exe".to_string();
        record_launched_path(&mut snapshot, launched.clone());

        // Zone 2: same exe now resolves to the launched path -> no second launch.
        let r2 = resolve_zone_paths(&zone2, &snapshot);
        assert_eq!(
            r2,
            ZoneResolution::Resolved {
                candidates: vec![launched],
                source: ResolutionSource::RunningProcess,
            }
        );
        assert!(
            !zone_needs_launch(&zone2, &r2, true, &snapshot),
            "the second zone must see the launched app as running"
        );
    }

    #[test]
    fn two_zones_same_exact_path_launch_only_once_with_evolving_snapshot() {
        let path = r"C:\Program Files\Vendor\app.exe".to_string();
        let mut zone1 = Zone::new(1, "App", "app.exe", 0.0, 0.0, 50.0, 100.0);
        zone1.path = path.clone();
        let mut zone2 = Zone::new(2, "App", "app.exe", 50.0, 0.0, 50.0, 100.0);
        zone2.path = path.clone();
        let mut snapshot: Vec<String> = Vec::new();

        // Exact configured path is a candidate even when not running -> launch.
        let r1 = resolve_zone_paths(&zone1, &snapshot);
        assert_eq!(
            r1,
            ZoneResolution::Resolved {
                candidates: vec![path.clone()],
                source: ResolutionSource::ExactConfig,
            }
        );
        assert!(zone_needs_launch(&zone1, &r1, true, &snapshot));
        record_launched_path(&mut snapshot, path.clone());

        // Second zone sees the exact path as running -> no second launch.
        let r2 = resolve_zone_paths(&zone2, &snapshot);
        assert!(
            !zone_needs_launch(&zone2, &r2, true, &snapshot),
            "the second zone must not spawn the same exact path again"
        );
    }

    /* ---------------- Finding 2: ambiguity instead of arbitrary selection --------------- */

    #[test]
    fn pathless_zone_with_two_distinct_paths_is_ambiguous_and_never_launches() {
        let zone = Zone::new(1, "Chrome", "chrome.exe", 0.0, 0.0, 50.0, 100.0);
        let running = vec![
            r"C:\Program Files\Google\Chrome\chrome.exe".to_string(),
            r"D:\Portable\chrome.exe".to_string(),
        ];
        let resolution = resolve_zone_paths(&zone, &running);
        assert_eq!(
            resolution,
            ZoneResolution::Ambiguous {
                paths: running.clone(),
            }
        );
        assert!(
            !zone_needs_launch(&zone, &resolution, true, &running),
            "an ambiguous assignment must never be launched"
        );
    }

    #[test]
    fn ambiguous_zone_reports_failure_with_both_paths_without_spawning() {
        let zone = Zone::new(1, "Chrome", "chrome.exe", 0.0, 0.0, 50.0, 100.0);
        let cfg = crate::config::AppConfig::default();
        let a = r"C:\Program Files\Google\Chrome\chrome.exe".to_string();
        let b = r"D:\Portable\chrome.exe".to_string();
        let execution = execute_zone_with_context(
            &zone,
            Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
            &cfg,
            true, // Launch & Arrange — still must not pick an arbitrary install
            &[a.clone(), b.clone()],
        );
        assert_eq!(
            execution,
            ZoneExecution::Failed {
                failure: ZoneFailure::AmbiguousPath,
                message: format!(
                    "chrome.exe matches multiple installations; re-assign with an exact path: {a}, {b}"
                ),
                launched: false,
            }
        );
    }

    /* ---------------- BAT/CMD launcher execution outcomes ---------------- */

    fn launcher_zone(script: &str) -> Zone {
        let mut z = Zone::new(1, "Script app", "app.exe", 0.0, 0.0, 50.0, 100.0);
        z.path = r"C:\App\app.exe".into();
        z.launcher_path = script.into();
        z
    }

    fn candidate(hwnd: isize, pid: u32) -> CandidateWindow {
        CandidateWindow {
            hwnd,
            pid,
            class: format!("class-{hwnd}"),
            process_path: format!(r"C:\App\app{hwnd}.exe"),
            title: format!("App {hwnd}"),
        }
    }

    /* ---------------- WP7: ambiguity settle timer resets on each new window ---------------- */

    #[test]
    fn delayed_second_window_resets_quiet_period_and_yields_ambiguous() {
        let t0 = std::time::Instant::now();
        let mut tracker = AttributionTracker::new(AMBIGUITY_SETTLE_MS);

        // First unique window appears at t0.
        tracker.observe(1, t0);
        assert!(!tracker.is_settled(t0));

        // Second unique window appears at t0+300ms -> quiet period resets.
        let t_second = t0 + std::time::Duration::from_millis(300);
        tracker.observe(1, t_second);

        // The old first_seen logic would have broken at t0+400ms. After the
        // reset, that point is no longer a settle point: the tracker waits a
        // full quiet period from the SECOND window instead.
        assert!(
            !tracker.is_settled(t0 + std::time::Duration::from_millis(AMBIGUITY_SETTLE_MS)),
            "the old break point must not be a settle point once a later window was seen"
        );
        assert!(!tracker.is_settled(t_second));
        assert!(!tracker.is_settled(t_second + std::time::Duration::from_millis(399)));
        assert!(tracker.is_settled(t_second + std::time::Duration::from_millis(400)));

        // Final attribution sees both windows -> Ambiguous (never snaps first).
        let a = candidate(1, 100);
        let b = candidate(2, 101);
        assert_eq!(
            decide_attribution(vec![a.clone(), b.clone()]),
            AttributionDecision::Ambiguous(vec![a, b])
        );
    }

    #[test]
    fn repeated_enumeration_of_same_hwnd_does_not_reset_quiet_period() {
        let t0 = std::time::Instant::now();
        let mut tracker = AttributionTracker::new(AMBIGUITY_SETTLE_MS);
        tracker.observe(1, t0); // first discovery

        // Repeated polls see the same HWND -> new_count == 0 -> no reset.
        for ms in [100, 200, 300] {
            tracker.observe(0, t0 + std::time::Duration::from_millis(ms));
        }
        assert!(!tracker.is_settled(t0 + std::time::Duration::from_millis(399)));
        assert!(tracker.is_settled(t0 + std::time::Duration::from_millis(400)));

        // A single window (even enumerated repeatedly) decides as Single.
        let w = candidate(7, 200);
        assert_eq!(
            decide_attribution(vec![w.clone(), w.clone()]),
            AttributionDecision::Single(w)
        );
    }

    #[test]
    fn single_window_settles_after_quiet_period_to_single() {
        let t0 = std::time::Instant::now();
        let mut tracker = AttributionTracker::new(AMBIGUITY_SETTLE_MS);
        tracker.observe(1, t0);
        assert!(!tracker.is_settled(t0 + std::time::Duration::from_millis(399)));
        let settled_at = t0 + std::time::Duration::from_millis(400);
        assert!(tracker.is_settled(settled_at));
        // The settled branch of the loop-exit decision (deadline still ahead).
        assert!(should_stop_attribution(
            settled_at,
            settled_at + std::time::Duration::from_secs(1),
            &tracker
        ));

        let w = candidate(42, 300);
        assert_eq!(
            decide_attribution(vec![w.clone()]),
            AttributionDecision::Single(w)
        );
    }

    #[test]
    fn multiple_windows_decide_as_ambiguous() {
        let a = candidate(10, 400);
        let b = candidate(11, 401);
        let c = candidate(12, 402);
        assert_eq!(
            decide_attribution(vec![a.clone(), b.clone(), c.clone()]),
            AttributionDecision::Ambiguous(vec![a, b, c])
        );
    }

    #[test]
    fn deadline_forces_decision_on_what_was_seen_without_false_single() {
        let t0 = std::time::Instant::now();
        let deadline = t0 + std::time::Duration::from_secs(LAUNCH_WINDOW_POLL_SECS);
        let mut tracker = AttributionTracker::new(AMBIGUITY_SETTLE_MS);
        tracker.observe(2, t0); // two windows seen, quiet period not elapsed

        // Not settled yet, but the deadline still stops the loop (no hang).
        assert!(!tracker.is_settled(t0 + std::time::Duration::from_millis(100)));
        assert!(should_stop_attribution(deadline, deadline, &tracker));

        // Decision is made on what was seen: >1 window is Ambiguous, not Single.
        let a = candidate(20, 500);
        let b = candidate(21, 501);
        assert_eq!(
            decide_attribution(vec![a.clone(), b.clone()]),
            AttributionDecision::Ambiguous(vec![a, b])
        );
    }

    /* ---------------- WP7: failed Job PID query is a truthful error ---------------- */

    #[test]
    fn job_query_failure_is_failed_outcome_and_never_proceeds() {
        let outcome = JobQueryOutcome::from_query_result(Err("access denied".to_string()));
        assert_eq!(
            outcome,
            JobQueryOutcome::Failed("access denied".to_string())
        );
        // A failure must NOT be a Proceed(empty) that would use the EXE override.
        assert!(!matches!(outcome, JobQueryOutcome::Proceed(_)));
    }

    #[test]
    fn job_query_success_empty_proceeds_to_existing_fallback_path() {
        let outcome = JobQueryOutcome::from_query_result(Ok(vec![]));
        assert_eq!(outcome, JobQueryOutcome::Proceed(vec![]));
    }

    #[test]
    fn job_query_success_with_pids_proceeds() {
        let outcome = JobQueryOutcome::from_query_result(Ok(vec![100, 200, 300]));
        assert_eq!(outcome, JobQueryOutcome::Proceed(vec![100, 200, 300]));
    }

    #[test]
    fn launcher_zone_with_metacharacter_reports_launcher_failed_and_never_launches() {
        // A metacharacter in the script path is rejected BEFORE any process is
        // created. The configured EXE must not be launched as a substitute.
        let zone = launcher_zone(r"C:\Scripts\run&evil.bat");
        let cfg = crate::config::AppConfig::default();
        let execution = execute_zone_with_context(
            &zone,
            Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
            &cfg,
            true, // Launch & Arrange — would launch if the EXE were substituted
            &[],
        );
        assert_eq!(
            execution,
            ZoneExecution::Failed {
                failure: ZoneFailure::LauncherFailed,
                message: format!(
                    "invalid launcher script '{}': path contains CMD metacharacter '&'",
                    r"C:\Scripts\run&evil.bat"
                ),
                launched: false,
            }
        );
    }

    #[test]
    fn launcher_zone_with_bad_extension_reports_launcher_failed() {
        let zone = launcher_zone(r"C:\Scripts\run.exe");
        let cfg = crate::config::AppConfig::default();
        let execution = execute_zone_with_context(
            &zone,
            Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
            &cfg,
            true,
            &[],
        );
        assert_eq!(
            execution.failure_for_test(),
            Some(ZoneFailure::LauncherFailed)
        );
        assert!(!execution.launched());
        assert!(execution.message().contains(".bat or .cmd"));
    }

    #[test]
    fn launcher_zone_in_apply_mode_never_runs_the_script() {
        // D1: Apply arranges running windows only. With no EXE override the
        // launcher cannot be attributed, so we report that honestly instead of
        // running the script or launching the EXE.
        let mut zone = launcher_zone(r"C:\Scripts\run.bat");
        zone.path.clear();
        let cfg = crate::config::AppConfig::default();
        let execution = execute_zone_with_context(
            &zone,
            Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
            &cfg,
            false, // Apply
            &[],
        );
        assert_eq!(
            execution.failure_for_test(),
            Some(ZoneFailure::NoEligibleWindow)
        );
        assert!(!execution.launched());
        assert!(execution.message().contains("was not run (Apply/Attach)"));
    }

    #[test]
    fn launcher_zone_in_attach_mode_never_runs_the_script() {
        let mut zone = launcher_zone(r"C:\Scripts\run.bat");
        zone.path.clear();
        zone.launch_mode = crate::config::LaunchMode::Attach;
        let cfg = crate::config::AppConfig::default();
        let execution = execute_zone_with_context(
            &zone,
            Rect {
                x: 0,
                y: 0,
                w: 100,
                h: 100,
            },
            &cfg,
            true, // Launch & Arrange, but Attach mode forbids launching
            &[],
        );
        assert_eq!(
            execution.failure_for_test(),
            Some(ZoneFailure::NoEligibleWindow)
        );
        assert!(!execution.launched());
    }

    #[test]
    fn exact_path_still_wins_unambiguously_over_running_duplicates() {
        let mut zone = Zone::new(1, "Chrome", "chrome.exe", 0.0, 0.0, 50.0, 100.0);
        zone.path = r"D:\Portable\chrome.exe".into();
        let running = vec![
            r"C:\Program Files\Google\Chrome\chrome.exe".to_string(),
            r"D:\Portable\chrome.exe".to_string(),
        ];
        assert_eq!(
            resolve_zone_paths(&zone, &running),
            ZoneResolution::Resolved {
                candidates: vec![r"D:\Portable\chrome.exe".to_string()],
                source: ResolutionSource::ExactConfig,
            }
        );
    }
}
