//! Native main-thread lifecycle (architecture §3.4 / §3.6 / §3.7).
//!
//! The main thread owns **all** Win32 windows: the hidden message-only wake-up
//! window, the layered drag overlay, the tray icon and the global hotkey
//! manager. It runs a single `MsgWaitForMultipleObjectsEx` loop with a variable
//! timeout (150 ms idle gesture detection → 16 ms active drag tracking) and a
//! manual-reset shutdown event as handle 0.

use crate::api::commands::CommandPayload;
use crate::api::state::Inner;
use crate::api::worker::Command;
use crate::api::ApiError;
use crate::config::AppConfig;
use crate::engine_core::Rect;
use crate::system::{Action, System};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::{oneshot, watch};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, COLORREF, HANDLE, HWND, LPARAM, LRESULT, POINT, SIZE, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
    AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::SetEvent;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, KillTimer,
    MessageBoxW, MsgWaitForMultipleObjectsEx, PeekMessageW, PostMessageW, PostQuitMessage,
    RegisterClassW, SetTimer, ShowWindow, TranslateMessage, UpdateLayeredWindow, HWND_MESSAGE,
    IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_YESNO, MESSAGEBOX_STYLE, MSG, MWMO_INPUTAVAILABLE,
    PM_REMOVE, QS_ALLINPUT, SW_HIDE, SW_SHOW, ULW_ALPHA, UPDATE_LAYERED_WINDOW_FLAGS,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_DESTROY, WM_QUIT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

const CLASS: PCWSTR = w!("ZoneFlow.MsgHost.7f3a9b7c");
const OVERLAY_CLASS: PCWSTR = w!("ZoneFlow.Overlay.7f3a9b7c");
const WM_WAKE: u32 = WM_APP + 1;
const WM_STOP: u32 = WM_APP + 2;
const TIMER_SAFETY_DRAIN: usize = 1;
const IDLE_MS: u32 = 150;
const ACTIVE_MS: u32 = 16;
const QUEUE_CAP: usize = 256;
const VK_CONTROL: i32 = 0x11;
const VK_LBUTTON: i32 = 0x01;

/* ============================ payload queue ============================== */

/// Messages sent from the worker (or main) to the native thread. Variants are
/// either *replaceable state notifications* (idempotent full-state refreshes)
/// or *must-not-drop events* (`Shutdown`). On overflow, older replaceable
/// notifications of the same kind are coalesced; must-not-drop events are
/// never discarded (architecture §3.4).
#[derive(Clone, Debug)]
pub enum NativeMessage {
    RebuildMenu {
        applied: String,
        selected: String,
        profiles: Vec<String>,
        enabled: bool,
    },
    SetToggleLabel {
        enabled: bool,
    },
    ReregisterHotkeys {
        hotkeys: BTreeMap<String, String>,
    },
    /// Full-state refresh of the cached zone rectangles used by the overlay
    /// and drag-release hit testing (idempotent, coalesces on overflow).
    SetZones {
        zones: Vec<(i32, Rect)>,
    },
    /// Full-state refresh of overlay colours/alpha from Settings (idempotent).
    SetOverlayStyle {
        tints: Vec<(u8, u8, u8)>,
        inactive_alpha: u8,
        active_alpha: u8,
    },
    ShowOverlay,
    HideOverlay,
    /// Must-not-drop: latches shutdown on the native thread.
    #[allow(dead_code)] // constructed by tests and Phase 5 native triggers
    Shutdown,
}

impl NativeMessage {
    fn kind(&self) -> u8 {
        match self {
            Self::RebuildMenu { .. } => 0,
            Self::SetToggleLabel { .. } => 1,
            Self::ReregisterHotkeys { .. } => 2,
            Self::SetZones { .. } => 3,
            Self::SetOverlayStyle { .. } => 4,
            Self::ShowOverlay => 5,
            Self::HideOverlay => 6,
            Self::Shutdown => 255,
        }
    }

    fn is_replaceable(&self) -> bool {
        !matches!(self, Self::Shutdown)
    }
}

struct QueueInner {
    items: VecDeque<NativeMessage>,
    accepting: bool,
}

/// Bounded, coalescing payload queue shared between the worker and main.
pub struct NativeQueue {
    inner: Mutex<QueueInner>,
}

impl NativeQueue {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(QueueInner {
                items: VecDeque::new(),
                accepting: true,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, QueueInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()) // tolerate poisoning; state stays consistent
    }

    /// Push a message. Returns `false` when the queue is no longer accepting
    /// (shutdown step 1), so no producer can post after the window is destroyed.
    pub fn push(&self, msg: NativeMessage) -> bool {
        let mut g = self.lock();
        if !g.accepting {
            return false;
        }
        if msg.is_replaceable() {
            let kind = msg.kind();
            g.items.retain(|m| m.kind() != kind);
        }
        if g.items.len() >= QUEUE_CAP && msg.is_replaceable() {
            if let Some(pos) = g.items.iter().position(|m| m.is_replaceable()) {
                g.items.remove(pos);
            }
        }
        if g.items.len() >= QUEUE_CAP {
            tracing::warn!("native payload queue at capacity; must-not-drop message kept");
        }
        g.items.push_back(msg);
        true
    }

    /// Drain all queued messages in FIFO order.
    pub fn drain(&self) -> Vec<NativeMessage> {
        self.lock().items.drain(..).collect()
    }

    pub fn set_accepting(&self, value: bool) {
        self.lock().accepting = value;
    }

    #[allow(dead_code)] // asserted by queue tests
    pub fn accepting(&self) -> bool {
        self.lock().accepting
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.lock().items.len()
    }
}

impl Default for NativeQueue {
    fn default() -> Self {
        Self::new()
    }
}

/* ============================ send wrappers ============================== */

/// `HWND` is `!Send`/`!Sync` in windows-rs. This newtype is shared only after
/// the hidden window exists and is published through a `OnceLock` gate; the
/// `accepting` flag plus join-before-destroy ordering guarantee no producer
/// posts after `DestroyWindow`. `PostMessageW` marshals by HWND on the OS side.
#[derive(Clone, Copy)]
pub struct HwndSend(pub *mut c_void);

// SAFETY: the raw pointer is a valid HWND created on the main thread. It is
// published only after creation (OnceLock gate) and every producer is joined
// before the window is destroyed (§3.6 steps 7 and 10). Producers only call
// PostMessageW, which the OS marshals safely across threads.
unsafe impl Send for HwndSend {}
// SAFETY: see the Send impl — the handle is immutable and read-only to all
// producers, and its lifetime is bounded by the join-before-destroy ordering.
unsafe impl Sync for HwndSend {}

/// `HANDLE` is `!Send`/`!Sync` in windows-rs 0.62. The event is process-wide,
/// created by main, shared read-only, and closed by main only after every
/// producer has been joined (§3.6 step 12). Producers call only `SetEvent`.
#[derive(Clone, Copy)]
pub struct SharedEvent(pub HANDLE);

// SAFETY: the event handle is created by main before any clone is shared, is
// never reset or closed by a producer, and is closed exactly once by main after
// all joins. SetEvent is thread-safe and idempotent for a valid handle.
unsafe impl Send for SharedEvent {}
// SAFETY: see the Send impl — the handle is immutable and all access is a
// read-only SetEvent call.
unsafe impl Sync for SharedEvent {}

impl SharedEvent {
    /// Test-only handle. Tests never call SetEvent on it; `trigger()` skips a
    /// null/invalid handle so no Win32 call is made with it.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self(HANDLE::default())
    }
}

/* ============================== notifier ================================= */

/// Worker → native side-effect channel. Pushes a payload then posts `WM_WAKE`
/// as a wake-up signal only (no pointer in LPARAM).
#[derive(Clone)]
pub struct NativeNotifier {
    queue: Arc<NativeQueue>,
    hwnd: Arc<OnceLock<HwndSend>>,
}

impl NativeNotifier {
    pub fn new(queue: Arc<NativeQueue>, hwnd: Arc<OnceLock<HwndSend>>) -> Self {
        Self { queue, hwnd }
    }

    fn post_wake(&self) {
        if let Some(h) = self.hwnd.get() {
            // SAFETY: `h` is a valid HWND published after creation; PostMessageW
            // takes no caller pointers and the OS marshals by HWND. Failure is
            // logged and the always-on safety timer covers the lost wake-up.
            if let Err(e) = unsafe { PostMessageW(Some(HWND(h.0)), WM_WAKE, WPARAM(0), LPARAM(0)) }
            {
                tracing::warn!(error = %e, "WM_WAKE post failed; relying on safety timer");
            }
        }
    }

    fn send(&self, msg: NativeMessage) {
        if self.queue.push(msg) {
            self.post_wake();
        }
    }

    /// Full-state tray refresh (applied checkmark, selected bullet, profile
    /// list, Disable/Enable label). Idempotent — coalesces on overflow.
    pub fn rebuild_menu(&self, inner: &Inner) {
        let profiles: Vec<String> = inner.draft.profiles.keys().cloned().collect();
        self.send(NativeMessage::RebuildMenu {
            applied: inner.applied_profile.clone(),
            selected: inner.selected_profile.clone(),
            profiles,
            enabled: inner.layout_enabled,
        });
    }

    pub fn set_toggle_label(&self, enabled: bool) {
        self.send(NativeMessage::SetToggleLabel { enabled });
    }

    pub fn reregister_hotkeys(&self, hotkeys: &BTreeMap<String, String>) {
        self.send(NativeMessage::ReregisterHotkeys {
            hotkeys: hotkeys.clone(),
        });
    }

    /// Push a full-state zone refresh to the native overlay cache.
    pub fn set_zones(&self, zones: Vec<(i32, Rect)>) {
        self.send(NativeMessage::SetZones { zones });
    }

    /// Push a full-state overlay style refresh (Settings changes, §3.3 step 4).
    pub fn set_overlay_style(
        &self,
        tints: Vec<(u8, u8, u8)>,
        inactive_alpha: u8,
        active_alpha: u8,
    ) {
        self.send(NativeMessage::SetOverlayStyle {
            tints,
            inactive_alpha,
            active_alpha,
        });
    }

    /// Derive and push the overlay style from the authoritative Settings.
    pub fn sync_overlay_style(&self, settings: &crate::config::Settings) {
        let tints = settings.zone_tints.iter().map(|s| parse_tint(s)).collect();
        self.set_overlay_style(
            tints,
            settings.overlay_inactive_alpha,
            settings.overlay_active_alpha,
        );
    }

    // Reserved for producers that need to force overlay visibility; normal
    // drag tracking drives the overlay directly from gesture state on main.
    #[allow(dead_code)]
    pub fn show_overlay(&self) {
        self.send(NativeMessage::ShowOverlay);
    }

    #[allow(dead_code)]
    pub fn hide_overlay(&self) {
        self.send(NativeMessage::HideOverlay);
    }
}

/* =========================== shutdown handle ============================= */

/// Idempotent shutdown coordinator. Cloned into main, the worker and the HTTP
/// state so tray Exit and `POST /api/shutdown` converge on the same ordered
/// sequence (architecture §3.6 steps 1–4).
#[derive(Clone)]
pub struct ShutdownHandle {
    guard: Arc<AtomicBool>,
    queue: Arc<NativeQueue>,
    event: SharedEvent,
    hwnd: Arc<OnceLock<HwndSend>>,
    sse_stop: watch::Sender<bool>,
    axum_shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl ShutdownHandle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        queue: Arc<NativeQueue>,
        event: SharedEvent,
        hwnd: Arc<OnceLock<HwndSend>>,
        sse_stop: watch::Sender<bool>,
        axum_shutdown: Option<oneshot::Sender<()>>,
    ) -> Self {
        Self {
            guard: Arc::new(AtomicBool::new(false)),
            queue,
            event,
            hwnd,
            sse_stop,
            axum_shutdown: Arc::new(Mutex::new(axum_shutdown)),
        }
    }

    pub fn is_shutting_down(&self) -> bool {
        self.guard.load(Ordering::SeqCst)
    }

    /// Steps 1–4 of §3.6. Safe to call from any thread; only the first caller
    /// performs the work.
    pub fn trigger(&self) {
        if self.guard.swap(true, Ordering::SeqCst) {
            return;
        }
        tracing::info!("shutdown requested");

        // 1) Stop accepting new native payloads (under the queue lock).
        self.queue.set_accepting(false);

        // 2) Close every SSE stream before Axum's graceful-shutdown await.
        self.sse_stop.send_replace(true);

        // 3) Signal Axum (oneshot can only be fired once).
        if let Ok(mut slot) = self.axum_shutdown.lock() {
            if let Some(tx) = slot.take() {
                if tx.send(()).is_err() {
                    tracing::debug!("axum shutdown receiver already dropped");
                }
            }
        }

        // 4a) Guaranteed wake-up: manual-reset event, not subject to the
        // 10,000-message queue limit. Skipped for the null test handle.
        if !self.event.0.is_invalid() {
            // SAFETY: the event handle is valid until main closes it after all
            // joins; SetEvent is cross-thread, idempotent and takes no pointers.
            if let Err(e) = unsafe { SetEvent(self.event.0) } {
                tracing::warn!(error = %e, "SetEvent failed during shutdown");
            }
        }

        // 4b) Best-effort wake-up for the modal MessageBoxW case. Failure is
        // expected when the queue is full; the event path already covers it.
        if let Some(h) = self.hwnd.get() {
            // SAFETY: valid published HWND; PostMessageW marshals by HWND and
            // takes no caller pointers. WM_STOP only posts WM_QUIT on main.
            if let Err(e) = unsafe { PostMessageW(Some(HWND(h.0)), WM_STOP, WPARAM(0), LPARAM(0)) }
            {
                tracing::debug!(error = %e, "WM_STOP post failed (event path still active)");
            }
        }
    }
}

/* ============================ D2 confirmation ============================ */

/// Abstraction over the native dirty-draft confirmation so the tray-switch
/// flow is unit-testable without a real modal dialog (architecture §4 / D2).
pub trait DirtyConfirm {
    /// `true` = Discard (retry with `discard_dirty=true`), `false` = Cancel.
    fn confirm_discard(&self, profile: &str) -> bool;
}

/// Production implementation: a native `MessageBoxW` with Yes/No buttons whose
/// text spells out the Discard/Cancel semantics explicitly. `None` is the owner
/// on purpose — `HWND_MESSAGE` is not a valid owner (architecture §3.4).
pub struct NativeDirtyConfirm;

impl DirtyConfirm for NativeDirtyConfirm {
    fn confirm_discard(&self, profile: &str) -> bool {
        let text = format!(
            "“{profile}” has unsaved changes.\n\nDiscard them and switch profiles?\n\nYes = Discard · No = Cancel"
        );
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);
        // SAFETY: `wide` is a local, null-terminated UTF-16 buffer that lives
        // for the whole call; MessageBoxW copies its text. The modal loop still
        // dispatches posted window messages, so WM_WAKE/WM_TIMER keep arriving.
        unsafe {
            MessageBoxW(
                None,
                windows::core::PCWSTR::from_raw(wide.as_ptr()),
                w!("ZoneFlow — unsaved changes"),
                MESSAGEBOX_STYLE(MB_YESNO.0 | MB_ICONWARNING.0 | MB_DEFBUTTON2.0),
            ) == IDYES
        }
    }
}

/// Handle the oneshot reply of a tray profile switch. Only `DirtyState` opens
/// the native dialog: Discard re-sends with `discard_dirty=true`, Cancel leaves
/// authoritative state untouched — never a silent discard (D2).
fn handle_tray_apply_reply(
    result: Result<Value, ApiError>,
    name: &str,
    cmd_tx: &mpsc::Sender<Command>,
    confirm: &dyn DirtyConfirm,
) {
    match result {
        Ok(_) => {}
        Err(ApiError::DirtyState) => {
            if confirm.confirm_discard(name) {
                let (reply, _) = oneshot::channel();
                if let Err(e) = cmd_tx.send(Command {
                    payload: CommandPayload::ProfileApply {
                        name: name.to_string(),
                        discard_dirty: true,
                    },
                    reply,
                }) {
                    tracing::warn!(error = %e, "failed to send discard-and-apply command");
                }
            } else {
                tracing::info!(profile = %name, "tray profile switch cancelled (dirty draft)");
            }
        }
        Err(e) => {
            tracing::warn!(error = ?e, profile = %name, "tray profile apply failed");
        }
    }
}

/* ============================== WndProc ================================== */

thread_local! {
    /// Messages drained by WndProc, applied by the main loop where the
    /// non-Send `System` lives. `try_borrow_mut` keeps the callback panic-free.
    static PENDING: RefCell<Vec<NativeMessage>> = const { RefCell::new(Vec::new()) };
}

/// The single process-wide queue used by WndProc. Set once at startup before
/// the hidden window is created, so no message can arrive before it is ready.
static WND_QUEUE: OnceLock<Arc<NativeQueue>> = OnceLock::new();

fn drain_into_pending() {
    if let Some(queue) = WND_QUEUE.get() {
        let messages = queue.drain();
        if messages.is_empty() {
            return;
        }
        PENDING.with(|cell| match cell.try_borrow_mut() {
            Ok(mut pending) => pending.extend(messages),
            Err(_) => tracing::warn!("native pending buffer busy; messages dropped"),
        });
    }
}

/// Hidden message-only window callback. Only non-blocking work: drain the
/// queue into the thread-local buffer, or post WM_QUIT on WM_STOP.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_WAKE => {
            drain_into_pending();
            LRESULT(0)
        }
        WM_TIMER if wp.0 == TIMER_SAFETY_DRAIN => {
            drain_into_pending();
            LRESULT(0)
        }
        WM_STOP => {
            // Owner thread only. NO DestroyWindow here (§3.6 step 4).
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: `hwnd` is the window being destroyed; KillTimer takes no
            // caller pointers and tolerates an already-killed timer.
            let _ = unsafe { KillTimer(Some(hwnd), TIMER_SAFETY_DRAIN) };
            LRESULT(0)
        }
        _ => {
            // SAFETY: all arguments are the ones Windows passed to this callback.
            unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
        }
    }
}

/// Overlay window callback: no state, just default handling. The overlay is
/// click-through (WS_EX_TRANSPARENT) and never receives input.
unsafe extern "system" fn overlay_wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: all arguments are the ones Windows passed to this callback.
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

/* ============================== native loop ============================== */

/// Owns every Win32 object created on the main thread. Dropped on main so
/// `System`'s `Drop` destroys its tray/hotkey windows on the creating thread.
pub struct NativeLoop {
    hidden: HWND,
    overlay: HWND,
    event: HANDLE,
    system: System,
    work: Rect,
    zones: Vec<(i32, Rect)>,
    hover_zone: i32,
    tints: Vec<(u8, u8, u8)>,
    inactive_alpha: u8,
    active_alpha: u8,
}

impl NativeLoop {
    /// Create the hidden window, overlay, tray/hotkeys and shutdown event.
    /// Must be called on the main thread after the HTTP port is known.
    pub fn new(cfg: &AppConfig, queue: Arc<NativeQueue>, event: HANDLE) -> Result<Self, String> {
        // SAFETY: GetModuleHandleW(None) returns the current module's HINSTANCE
        // or an error; no caller pointers are involved.
        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;

        // Register the hidden-window class once; tolerate a duplicate class
        // (ERROR_CLASS_ALREADY_EXISTS = 1410) from a prior instance in-process.
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance.into(),
            lpszClassName: CLASS,
            ..Default::default()
        };
        // SAFETY: `wc` is a local, correctly initialized WNDCLASSW whose
        // lpszClassName points to a 'static wide literal. The pointer is valid
        // for the duration of the call and RegisterClassW copies the class data.
        let atom = unsafe { RegisterClassW(&wc) };
        if atom == 0 {
            // SAFETY: GetLastError takes no arguments and reads thread-local
            // error state only.
            let err = unsafe { windows::Win32::Foundation::GetLastError() };
            if err.0 != 1410 {
                return Err(format!("RegisterClassW failed: {err:?}"));
            }
        }

        let oc = WNDCLASSW {
            lpfnWndProc: Some(overlay_wndproc),
            hInstance: hinstance.into(),
            lpszClassName: OVERLAY_CLASS,
            ..Default::default()
        };
        // SAFETY: same contract as the hidden class registration above.
        let oatom = unsafe { RegisterClassW(&oc) };
        if oatom == 0 {
            // SAFETY: GetLastError takes no arguments.
            let err = unsafe { windows::Win32::Foundation::GetLastError() };
            if err.0 != 1410 {
                return Err(format!("overlay RegisterClassW failed: {err:?}"));
            }
        }

        // Publish the queue before creating the window so WndProc can never
        // observe an uninitialized queue.
        let _ = WND_QUEUE.set(queue.clone());

        // SAFETY: CLASS is a registered 'static wide string; HWND_MESSAGE is a
        // valid parent constant; all other arguments are by value. The returned
        // HWND is checked by CreateWindowExW's Result.
        let hidden = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                CLASS,
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| format!("hidden window creation failed: {e}"))?;

        let work = crate::engine::work_area();
        // SAFETY: OVERLAY_CLASS is registered; the layered/ex-transparent/
        // topmost/no-activate/toolwindow styles are valid flags; all coordinates
        // are by value. WS_EX_TRANSPARENT makes it click-through, WS_EX_TOPMOST
        // keeps it above every app, WS_EX_TOOLWINDOW keeps it off the taskbar.
        let overlay = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                OVERLAY_CLASS,
                w!(""),
                WS_POPUP,
                work.x,
                work.y,
                work.w,
                work.h,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| format!("overlay window creation failed: {e}"))?;

        // Always-on 100 ms safety drain (§3.4). WM_TIMER is low-priority and
        // survives modal loops, so no payload can be stranded.
        // SAFETY: `hidden` is a valid window created above; no callback pointer
        // is supplied (TIMERPROC(None)), so messages go to its WndProc.
        let timer = unsafe { SetTimer(Some(hidden), TIMER_SAFETY_DRAIN, 100, None) };
        if timer == 0 {
            tracing::warn!("SetTimer failed; native queue safety drain unavailable");
        }

        let system =
            System::with_config(cfg).map_err(|e| format!("tray/hotkey init failed: {e}"))?;

        // Send an initial full-state refresh so the tray reflects applied and
        // selected profiles from the very first frame.
        let notifier = NativeNotifier::new(queue.clone(), Arc::new(OnceLock::new()));
        let mut initial = Inner::new(cfg.clone());
        initial.applied_profile = cfg.active_profile.clone();
        initial.selected_profile = cfg.active_profile.clone();
        notifier.rebuild_menu(&initial);
        notifier.set_toggle_label(true);
        // Drain the initial payloads now (before the loop starts) so they are
        // applied on the first iteration.
        drain_into_pending();

        let tints = cfg
            .settings
            .zone_tints
            .iter()
            .map(|s| parse_tint(s))
            .collect();

        // The caller publishes the HWND slot after construction (startup step 4).

        Ok(Self {
            hidden,
            overlay,
            event,
            system,
            work,
            zones: crate::engine_core::compute_zones(cfg.active(), work, cfg.gap),
            hover_zone: -1,
            tints,
            inactive_alpha: cfg.settings.overlay_inactive_alpha,
            active_alpha: cfg.settings.overlay_active_alpha,
        })
    }

    /// Publish the hidden HWND to producers (startup step 4 gate).
    pub fn publish_hwnd(&self, slot: &Arc<OnceLock<HwndSend>>) {
        let _ = slot.set(HwndSend(self.hidden.0));
    }

    /// Refresh the locally cached zone rectangles used by the overlay and
    /// drag-release hit testing. Driven by `NativeMessage::SetZones` from the
    /// worker after every state change.
    pub fn set_zones(&mut self, zones: Vec<(i32, Rect)>) {
        self.zones = zones;
    }

    /// Refresh cached overlay colours/alpha from a Settings change so the next
    /// drag paints with the new values without a restart (§3.3 step 4).
    pub fn set_overlay_style(
        &mut self,
        tints: Vec<(u8, u8, u8)>,
        inactive_alpha: u8,
        active_alpha: u8,
    ) {
        if !tints.is_empty() {
            self.tints = tints;
        }
        self.inactive_alpha = inactive_alpha;
        self.active_alpha = active_alpha;
    }

    /// Run the message loop. Returns when the shutdown event is signalled or
    /// WM_QUIT is retrieved. Window destruction happens in `finish`, after all
    /// producers have been joined (§3.6 steps 10–12).
    pub fn run(
        &mut self,
        cmd_tx: mpsc::Sender<Command>,
        shutdown: &ShutdownHandle,
        confirm: &dyn DirtyConfirm,
    ) {
        let mut active = false;
        // Tray profile switches are sent as guarded commands; their oneshot
        // replies are polled here so a 409 dirty_state can open the native D2
        // dialog without blocking the message loop on a busy worker.
        let mut pending_applies: Vec<(String, oneshot::Receiver<Result<Value, ApiError>>)> =
            Vec::new();
        loop {
            let timeout = if active { ACTIVE_MS } else { IDLE_MS };
            // SAFETY: `self.event` is a valid manual-reset event created before
            // the loop and closed only after it exits. The handle slice lives
            // for the duration of the call; QS_ALLINPUT + MWMO_INPUTAVAILABLE
            // ensure queued messages are never missed.
            let wait = unsafe {
                MsgWaitForMultipleObjectsEx(
                    Some(&[self.event]),
                    timeout,
                    QS_ALLINPUT,
                    MWMO_INPUTAVAILABLE,
                )
            };
            if wait == WAIT_OBJECT_0 {
                break; // shutdown event (index 0)
            }

            // Drain all queued messages. Never filter by HWND: WM_QUIT has a
            // NULL hwnd and must not be hidden.
            loop {
                let mut msg = MSG::default();
                // SAFETY: `msg` is a local, aligned MSG that lives for the call;
                // PM_REMOVE consumes the message; no HWND filter is applied.
                let got = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) };
                if !got.as_bool() {
                    break;
                }
                if msg.message == WM_QUIT {
                    // Defensive: WM_QUIT normally arrives only after trigger()
                    // posted WM_STOP, but any other source (e.g. an external
                    // quit request) must still run the full §3.6 sequence so
                    // SSE and Axum are closed before main joins them. trigger()
                    // is idempotent, so this is a no-op on the normal path.
                    shutdown.trigger();
                    return;
                }
                // SAFETY: `msg` was just filled by PeekMessageW and is valid.
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // Apply payloads drained by WndProc (and the safety timer).
            self.apply_pending(&cmd_tx, shutdown);

            // Native → worker: hotkeys and tray clicks. The clipboard action
            // is handled here on the UI thread; everything else goes to the
            // worker as before.
            for action in self.system.poll() {
                match action {
                    Action::CopyEditorUrl => {
                        // Win32 clipboard access belongs on the thread that
                        // owns the message loop. The URL is the fixed constant
                        // used by the "Open editor…" action too. The overlay is
                        // a real WS_POPUP owned by this thread and is therefore
                        // a valid clipboard owner (the HWND_MESSAGE window is
                        // not — see MSDN on OpenClipboard/EmptyClipboard).
                        match crate::clipboard::copy_text(self.overlay, crate::browser::EDITOR_URL)
                        {
                            Ok(()) => tracing::info!("editor URL copied to clipboard"),
                            Err(e) => {
                                tracing::warn!(error = %e, "failed to copy editor URL to clipboard")
                            }
                        }
                    }
                    Action::ProfileApply(name) => {
                        // D6/D2: route tray profile clicks through the guarded
                        // ProfileApply command so a dirty draft yields 409
                        // instead of being silently overwritten.
                        let (reply, rx) = oneshot::channel();
                        if let Err(e) = cmd_tx.send(Command {
                            payload: CommandPayload::ProfileApply {
                                name: name.clone(),
                                discard_dirty: false,
                            },
                            reply,
                        }) {
                            tracing::warn!(error = %e, "failed to forward tray profile apply");
                        } else {
                            pending_applies.push((name, rx));
                        }
                    }
                    other => {
                        let (reply, _) = oneshot::channel();
                        if let Err(e) = cmd_tx.send(Command {
                            payload: CommandPayload::Action(other),
                            reply,
                        }) {
                            tracing::warn!(error = %e, "failed to forward native action to worker");
                        }
                    }
                }
            }

            // Poll tray-apply replies without blocking. A DirtyState reply
            // opens the (mockable) native dialog; Discard re-sends with the
            // discard flag, Cancel leaves state untouched.
            let mut i = 0;
            while i < pending_applies.len() {
                match pending_applies[i].1.try_recv() {
                    Ok(result) => {
                        let (name, _) = pending_applies.remove(i);
                        handle_tray_apply_reply(result, &name, &cmd_tx, confirm);
                    }
                    Err(oneshot::error::TryRecvError::Empty) => i += 1,
                    Err(oneshot::error::TryRecvError::Closed) => {
                        tracing::warn!("tray profile apply reply channel closed");
                        pending_applies.remove(i);
                    }
                }
            }

            // Gesture detection (§3.7): global key state, focus-independent.
            let down = gesture_down();
            if down && !active {
                active = true;
                self.show_overlay();
                self.update_overlay();
            } else if !down && active {
                active = false;
                self.hide_overlay();
                self.on_drag_release(&cmd_tx);
            } else if down && active {
                self.update_overlay();
            }

            if shutdown.is_shutting_down() {
                break;
            }
        }
    }

    fn apply_pending(&mut self, cmd_tx: &mpsc::Sender<Command>, shutdown: &ShutdownHandle) {
        let messages = PENDING.with(|cell| cell.replace(Vec::new()));
        for msg in messages {
            match msg {
                NativeMessage::RebuildMenu {
                    applied,
                    selected,
                    profiles,
                    enabled,
                } => self
                    .system
                    .rebuild_profiles(&applied, &selected, &profiles, enabled),
                NativeMessage::SetToggleLabel { enabled } => self.system.set_toggle_label(enabled),
                NativeMessage::ReregisterHotkeys { hotkeys } => {
                    self.system.reregister_hotkeys(&hotkeys)
                }
                NativeMessage::SetZones { zones } => self.set_zones(zones),
                NativeMessage::SetOverlayStyle {
                    tints,
                    inactive_alpha,
                    active_alpha,
                } => self.set_overlay_style(tints, inactive_alpha, active_alpha),
                NativeMessage::ShowOverlay => self.show_overlay(),
                NativeMessage::HideOverlay => self.hide_overlay(),
                NativeMessage::Shutdown => shutdown.trigger(),
            }
        }
        let _ = cmd_tx; // reserved for future native→worker payloads
    }

    fn show_overlay(&self) {
        // SAFETY: `self.overlay` is a valid window created on this thread.
        unsafe {
            let _ = ShowWindow(self.overlay, SW_SHOW);
        }
    }

    fn hide_overlay(&self) {
        // SAFETY: `self.overlay` is a valid window created on this thread.
        unsafe {
            let _ = ShowWindow(self.overlay, SW_HIDE);
        }
    }

    fn update_overlay(&mut self) {
        let (x, y) = cursor_pos();
        self.hover_zone = crate::engine_core::zone_at(&self.zones, x, y).unwrap_or(-1);
        self.paint_overlay();
    }

    fn on_drag_release(&self, cmd_tx: &mpsc::Sender<Command>) {
        let (x, y) = cursor_pos();
        let Some(zone_id) = crate::engine_core::zone_at(&self.zones, x, y) else {
            return;
        };
        // `window_at` wraps its own Win32 calls with documented SAFETY notes;
        // the overlay is click-through so the underlying app is returned.
        let hwnd = crate::engine::window_at(x, y).map(|h| h.0 as isize);
        if let Some(hwnd) = hwnd {
            let (reply, _) = oneshot::channel();
            if let Err(e) = cmd_tx.send(Command {
                payload: CommandPayload::MoveWindow { hwnd, zone_id },
                reply,
            }) {
                tracing::warn!(error = %e, "failed to forward drag release to worker");
            }
        }
    }

    /// Paint the layered overlay with per-zone alpha via `UpdateLayeredWindow`.
    /// Each zone is filled with its tint at the configured alpha and outlined;
    /// the hovered zone gets a thicker, fully opaque highlight border. The DIB
    /// is created and released per frame so no GDI handle is retained across
    /// drag sessions.
    fn paint_overlay(&self) {
        if self.zones.is_empty() || self.tints.is_empty() {
            return;
        }
        // SAFETY: all GDI handles below are local, checked for validity, and
        // released exactly once before the function returns. The DIB bits
        // pointer is used only while the bitmap is selected into the DC.
        unsafe {
            let screen_dc = windows::Win32::Graphics::Gdi::GetDC(None);
            if screen_dc.is_invalid() {
                return;
            }
            let mem_dc = CreateCompatibleDC(Some(screen_dc));
            if mem_dc.is_invalid() {
                let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
                return;
            }

            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: self.work.w,
                biHeight: -self.work.h, // top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            let bitmap =
                match CreateDIBSection(Some(mem_dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!(error = %e, "CreateDIBSection failed");
                        let _ = DeleteDC(mem_dc);
                        let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
                        return;
                    }
                };
            if bits.is_null() {
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(mem_dc);
                let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
                return;
            }
            let old = SelectObject(mem_dc, bitmap.into());

            let pixels = std::slice::from_raw_parts_mut(
                bits as *mut u8,
                (self.work.w as usize) * (self.work.h as usize) * 4,
            );
            pixels.fill(0);
            let stride = self.work.w as usize;
            for (idx, (zid, z)) in self.zones.iter().enumerate() {
                let (r, g, b) = self.tints[idx % self.tints.len()];
                let hovered = *zid == self.hover_zone;
                let alpha = if hovered {
                    self.active_alpha
                } else {
                    self.inactive_alpha
                };
                let x0 = (z.x - self.work.x).max(0) as usize;
                let y0 = (z.y - self.work.y).max(0) as usize;
                let x1 = ((z.x - self.work.x + z.w) as usize).min(self.work.w as usize);
                let y1 = ((z.y - self.work.y + z.h) as usize).min(self.work.h as usize);
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }
                // Zone body at the configured alpha.
                blit_rect(pixels, stride, (x0, y0, x1, y1), (r, g, b, alpha));
                // Outline: thicker and fully opaque on the hovered zone, a
                // subtle brighter edge elsewhere so every drop target is clear.
                let border = if hovered { 3usize } else { 1usize };
                let edge_alpha = if hovered {
                    255
                } else {
                    alpha.saturating_add(40).min(200)
                };
                let edge = (r, g, b, edge_alpha);
                blit_rect(pixels, stride, (x0, y0, x1, (y0 + border).min(y1)), edge);
                blit_rect(
                    pixels,
                    stride,
                    (x0, y1.saturating_sub(border).max(y0), x1, y1),
                    edge,
                );
                blit_rect(pixels, stride, (x0, y0, (x0 + border).min(x1), y1), edge);
                blit_rect(
                    pixels,
                    stride,
                    (x1.saturating_sub(border).max(x0), y0, x1, y1),
                    edge,
                );
            }

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let pt_src = POINT::default();
            let size = SIZE {
                cx: self.work.w,
                cy: self.work.h,
            };
            let pt_dst = POINT {
                x: self.work.x,
                y: self.work.y,
            };
            if let Err(e) = UpdateLayeredWindow(
                self.overlay,
                None,
                Some(&pt_dst),
                Some(&size),
                Some(mem_dc),
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                UPDATE_LAYERED_WINDOW_FLAGS(ULW_ALPHA.0),
            ) {
                tracing::warn!(error = %e, "UpdateLayeredWindow failed");
            }

            if !old.is_invalid() {
                let _ = SelectObject(mem_dc, old);
            }
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(mem_dc);
            let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, screen_dc);
        }
    }

    /// §3.6 steps 9–12: kill the timer, destroy windows, drop `System`, close
    /// the event. Called on main after Tokio and the worker have been joined.
    pub fn finish(mut self, queue: &NativeQueue) {
        // 8) Final drain — no producer can add more (all joined, accepting=false).
        for msg in queue.drain() {
            match msg {
                NativeMessage::RebuildMenu {
                    applied,
                    selected,
                    profiles,
                    enabled,
                } => self
                    .system
                    .rebuild_profiles(&applied, &selected, &profiles, enabled),
                NativeMessage::SetToggleLabel { enabled } => self.system.set_toggle_label(enabled),
                NativeMessage::ReregisterHotkeys { hotkeys } => {
                    self.system.reregister_hotkeys(&hotkeys)
                }
                NativeMessage::SetZones { zones } => self.set_zones(zones),
                NativeMessage::SetOverlayStyle {
                    tints,
                    inactive_alpha,
                    active_alpha,
                } => self.set_overlay_style(tints, inactive_alpha, active_alpha),
                NativeMessage::ShowOverlay => self.show_overlay(),
                NativeMessage::HideOverlay => self.hide_overlay(),
                NativeMessage::Shutdown => {}
            }
        }

        // 9) Kill the safety timer explicitly (DestroyWindow also kills it).
        // SAFETY: `self.hidden` is still valid; KillTimer takes no pointers.
        unsafe {
            if let Err(e) = KillTimer(Some(self.hidden), TIMER_SAFETY_DRAIN) {
                tracing::debug!(error = %e, "KillTimer failed (timer may already be gone)");
            }
        }

        // 10) Destroy overlay then hidden window. All producers are joined.
        // SAFETY: both handles are valid windows created on this thread and no
        // producer can post to them any more.
        unsafe {
            if let Err(e) = DestroyWindow(self.overlay) {
                tracing::warn!(error = %e, "DestroyWindow(overlay) failed");
            }
            if let Err(e) = DestroyWindow(self.hidden) {
                tracing::warn!(error = %e, "DestroyWindow(hidden) failed");
            }
        }

        // 11) `self.system` drops here on main (tray/hotkey Drop destroys their
        // own HWNDs on the creating thread).

        // 12) Close the event last, after the wait loop exited and all producers
        // joined. SAFETY: `self.event` is valid and no other handle user remains.
        unsafe {
            if let Err(e) = CloseHandle(self.event) {
                tracing::warn!(error = %e, "CloseHandle(shutdown event) failed");
            }
        }

        // Unregistering classes at process exit is optional; skip it to avoid
        // racing any other in-process instance.
    }
}

/// Global Ctrl+LeftButton state (§3.7). `GetAsyncKeyState` reads physical key
/// state at call time, is focus-independent, and tests the 0x8000 high bit.
fn gesture_down() -> bool {
    // SAFETY: GetAsyncKeyState takes an i32 virtual-key code by value, reads no
    // caller memory, and has no preconditions beyond a valid VK constant.
    unsafe {
        let ctrl = (GetAsyncKeyState(VK_CONTROL) as u16 & 0x8000) != 0;
        let lbtn = (GetAsyncKeyState(VK_LBUTTON) as u16 & 0x8000) != 0;
        ctrl && lbtn
    }
}

fn cursor_pos() -> (i32, i32) {
    // SAFETY: `p` is a local, aligned POINT valid for the call; GetCursorPos
    // writes only to it and no pointer escapes.
    unsafe {
        let mut p = POINT::default();
        if let Err(e) = GetCursorPos(&mut p) {
            tracing::warn!(error = %e, "GetCursorPos failed; using default position");
        }
        (p.x, p.y)
    }
}

/// Fill a rectangle in a top-down 32bpp BGRA DIB. Coordinates are clipped by
/// the caller; out-of-range pixels are ignored rather than wrapping rows.
fn blit_rect(
    pixels: &mut [u8],
    stride: usize,
    (x0, y0, x1, y1): (usize, usize, usize, usize),
    (r, g, b, a): (u8, u8, u8, u8),
) {
    for y in y0..y1 {
        for x in x0..x1 {
            let off = (y * stride + x) * 4;
            if off + 3 < pixels.len() {
                pixels[off] = b; // DIB byte order is BGRA.
                pixels[off + 1] = g;
                pixels[off + 2] = r;
                pixels[off + 3] = a;
            }
        }
    }
}

/// Parse `#RRGGBB` into RGB; malformed values fall back to neutral grey.
fn parse_tint(s: &str) -> (u8, u8, u8) {
    let hex = s.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.is_ascii() {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return (r, g, b);
        }
    }
    (128, 128, 128)
}

/* --------------------------------- tests --------------------------------- */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_coalesces_same_kind_keeping_newest() {
        let q = NativeQueue::new();
        q.push(NativeMessage::RebuildMenu {
            applied: "A".into(),
            selected: "A".into(),
            profiles: vec!["A".into()],
            enabled: true,
        });
        q.push(NativeMessage::SetToggleLabel { enabled: false });
        q.push(NativeMessage::RebuildMenu {
            applied: "B".into(),
            selected: "B".into(),
            profiles: vec!["B".into(), "C".into()],
            enabled: false,
        });
        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        match &drained[0] {
            NativeMessage::SetToggleLabel { enabled } => assert!(!enabled),
            other => panic!("unexpected message: {other:?}"),
        }
        match &drained[1] {
            NativeMessage::RebuildMenu {
                applied, profiles, ..
            } => {
                assert_eq!(applied, "B");
                assert_eq!(profiles.len(), 2);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn queue_never_drops_must_not_drop_on_overflow() {
        let q = NativeQueue::new();
        for i in 0..QUEUE_CAP + 10 {
            q.push(NativeMessage::RebuildMenu {
                applied: format!("A{i}"),
                selected: String::new(),
                profiles: Vec::new(),
                enabled: true,
            });
        }
        q.push(NativeMessage::Shutdown);
        let drained = q.drain();
        assert!(drained.iter().any(|m| matches!(m, NativeMessage::Shutdown)));
        // Replaceable notifications are coalesced to a bounded set.
        assert!(drained.len() <= QUEUE_CAP + 1);
    }

    #[test]
    fn queue_stops_accepting_after_shutdown_flag() {
        let q = NativeQueue::new();
        q.set_accepting(false);
        assert!(!q.push(NativeMessage::HideOverlay));
        assert_eq!(q.len(), 0);
        assert!(!q.accepting());
    }

    #[test]
    fn parse_tint_handles_valid_and_invalid_hex() {
        assert_eq!(parse_tint("#010203"), (1, 2, 3));
        assert_eq!(parse_tint("F5B544"), (0xF5, 0xB5, 0x44));
        assert_eq!(parse_tint("nonsense"), (128, 128, 128));
    }

    #[test]
    fn message_kinds_are_distinct() {
        let kinds = [
            NativeMessage::RebuildMenu {
                applied: String::new(),
                selected: String::new(),
                profiles: Vec::new(),
                enabled: true,
            }
            .kind(),
            NativeMessage::SetToggleLabel { enabled: true }.kind(),
            NativeMessage::ReregisterHotkeys {
                hotkeys: BTreeMap::new(),
            }
            .kind(),
            NativeMessage::SetZones { zones: Vec::new() }.kind(),
            NativeMessage::SetOverlayStyle {
                tints: Vec::new(),
                inactive_alpha: 0,
                active_alpha: 0,
            }
            .kind(),
            NativeMessage::ShowOverlay.kind(),
            NativeMessage::HideOverlay.kind(),
            NativeMessage::Shutdown.kind(),
        ];
        let mut unique = kinds.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), kinds.len());
        assert_eq!(unique.len(), 8);
    }

    /* ------------------------- reverse updates ------------------------- */

    #[test]
    fn notifier_rebuild_menu_pushes_full_state() {
        let queue = Arc::new(NativeQueue::new());
        let notifier = NativeNotifier::new(queue.clone(), Arc::new(OnceLock::new()));
        let mut inner = Inner::new(crate::config::default_config());
        inner.applied_profile = "Coding".into();
        inner.selected_profile = "Trading".into();
        inner.layout_enabled = false;
        notifier.rebuild_menu(&inner);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        match &drained[0] {
            NativeMessage::RebuildMenu {
                applied,
                selected,
                profiles,
                enabled,
            } => {
                assert_eq!(applied, "Coding");
                assert_eq!(selected, "Trading");
                assert_eq!(profiles.len(), 3);
                assert!(!enabled);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn notifier_reregister_hotkeys_pushes_bindings() {
        let queue = Arc::new(NativeQueue::new());
        let notifier = NativeNotifier::new(queue.clone(), Arc::new(OnceLock::new()));
        let mut hotkeys = BTreeMap::new();
        hotkeys.insert("clear_layout".to_string(), "Ctrl+Shift+1".to_string());
        notifier.reregister_hotkeys(&hotkeys);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        match &drained[0] {
            NativeMessage::ReregisterHotkeys { hotkeys } => {
                assert_eq!(hotkeys.get("clear_layout").unwrap(), "Ctrl+Shift+1");
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn notifier_sync_overlay_style_pushes_tints_and_alpha() {
        let queue = Arc::new(NativeQueue::new());
        let notifier = NativeNotifier::new(queue.clone(), Arc::new(OnceLock::new()));
        let mut settings = crate::config::default_config().settings;
        settings.zone_tints = vec!["#010203".into()];
        settings.overlay_inactive_alpha = 111;
        settings.overlay_active_alpha = 222;
        notifier.sync_overlay_style(&settings);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        match &drained[0] {
            NativeMessage::SetOverlayStyle {
                tints,
                inactive_alpha,
                active_alpha,
            } => {
                assert_eq!(tints, &vec![(1, 2, 3)]);
                assert_eq!(*inactive_alpha, 111);
                assert_eq!(*active_alpha, 222);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    /* ----------------------------- D2 dialog --------------------------- */

    struct MockConfirm {
        discard: bool,
        calls: std::cell::Cell<usize>,
    }

    impl DirtyConfirm for MockConfirm {
        fn confirm_discard(&self, _profile: &str) -> bool {
            self.calls.set(self.calls.get() + 1);
            self.discard
        }
    }

    #[test]
    fn d2_discard_resends_profile_apply_with_discard_flag() {
        let (tx, rx) = mpsc::channel::<Command>();
        let confirm = MockConfirm {
            discard: true,
            calls: std::cell::Cell::new(0),
        };
        handle_tray_apply_reply(Err(ApiError::DirtyState), "Coding", &tx, &confirm);
        assert_eq!(confirm.calls.get(), 1, "the dialog must be shown once");
        let cmd = rx.try_recv().expect("Discard must re-send the command");
        match cmd.payload {
            CommandPayload::ProfileApply {
                name,
                discard_dirty,
            } => {
                assert_eq!(name, "Coding");
                assert!(discard_dirty, "Discard must set discard_dirty=true");
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[test]
    fn d2_cancel_sends_no_command() {
        let (tx, rx) = mpsc::channel::<Command>();
        let confirm = MockConfirm {
            discard: false,
            calls: std::cell::Cell::new(0),
        };
        handle_tray_apply_reply(Err(ApiError::DirtyState), "Coding", &tx, &confirm);
        assert_eq!(confirm.calls.get(), 1);
        assert!(rx.try_recv().is_err(), "Cancel must not send a command");
    }

    #[test]
    fn d2_non_dirty_error_does_not_open_dialog() {
        let (tx, rx) = mpsc::channel::<Command>();
        let confirm = MockConfirm {
            discard: true,
            calls: std::cell::Cell::new(0),
        };
        handle_tray_apply_reply(
            Err(ApiError::NotFound("profile".into())),
            "Coding",
            &tx,
            &confirm,
        );
        assert_eq!(confirm.calls.get(), 0, "only DirtyState opens the dialog");
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn d2_success_does_not_open_dialog() {
        let (tx, rx) = mpsc::channel::<Command>();
        let confirm = MockConfirm {
            discard: true,
            calls: std::cell::Cell::new(0),
        };
        handle_tray_apply_reply(
            Ok(serde_json::json!({ "ok": true })),
            "Coding",
            &tx,
            &confirm,
        );
        assert_eq!(confirm.calls.get(), 0);
        assert!(rx.try_recv().is_err());
    }

    /* ---------------------------- shutdown ----------------------------- */

    #[test]
    fn shutdown_trigger_is_idempotent_and_stops_producers() {
        let queue = Arc::new(NativeQueue::new());
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let (axum_tx, axum_rx) = oneshot::channel::<()>();
        let shutdown = ShutdownHandle::new(
            queue.clone(),
            SharedEvent::new_test(),
            Arc::new(OnceLock::new()),
            stop_tx,
            Some(axum_tx),
        );
        assert!(!shutdown.is_shutting_down());
        shutdown.trigger();
        shutdown.trigger(); // second call must be a no-op
        assert!(shutdown.is_shutting_down());
        assert!(!queue.accepting(), "no producer may post after shutdown");
        assert!(*stop_rx.borrow_and_update(), "SSE streams must be closed");
        assert!(
            axum_rx.blocking_recv().is_ok(),
            "Axum must be signaled once"
        );
    }

    #[test]
    fn shutdown_event_is_signaled_even_without_hwnd() {
        use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

        // SAFETY: CreateEventW takes no caller pointers; manual-reset, initially
        // non-signaled, unnamed. Returns a valid handle or an error.
        let event = unsafe { CreateEventW(None, true, false, None) }.expect("CreateEventW failed");
        let queue = Arc::new(NativeQueue::new());
        let (stop_tx, _) = watch::channel(false);
        // Publish an invalid HWND so PostMessageW(WM_STOP) is attempted and
        // fails; the guaranteed SetEvent path must still latch (§3.6 step 4a).
        let hwnd_slot = Arc::new(OnceLock::new());
        let _ = hwnd_slot.set(HwndSend(std::ptr::dangling_mut::<c_void>()));
        let shutdown =
            ShutdownHandle::new(queue.clone(), SharedEvent(event), hwnd_slot, stop_tx, None);
        shutdown.trigger();
        // SAFETY: `event` is a valid open handle; a zero timeout only queries
        // its current state without blocking.
        let wait = unsafe { WaitForSingleObject(event, 0) };
        assert_eq!(wait, WAIT_OBJECT_0, "SetEvent must latch without an HWND");
        // SAFETY: close exactly once, after the wait and with no other users.
        unsafe { CloseHandle(event).expect("CloseHandle failed") };
    }
}
