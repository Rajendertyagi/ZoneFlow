//! Win32 launcher support for `.bat`/`.cmd` scripts.
//!
//! Attribution is reliable because a **Job Object** is the authoritative
//! process tree: `QueryInformationJobObject(JobObjectBasicProcessIdList)`
//! returns every PID in the job, including descendants created after the
//! script exits. There is deliberately **no** before/after HWND snapshot,
//! process-name, window-title or parent-PID heuristic here — the visible
//! cmd.exe console is itself a new eligible window and unrelated windows can
//! appear during the poll, so a snapshot cannot guarantee attribution.
//!
//! Safety choices (per the design constraint):
//! - No `KILL_ON_JOB_CLOSE`: closing our handle must never kill the app when
//!   ZoneFlow exits.
//! - No `BREAKAWAY_OK` / `SILENT_BREAKAWAY_OK`: children must stay in the job.
//! - `CREATE_SUSPENDED` + `AssignProcessToJobObject` **before** `ResumeThread`
//!   so no child can escape the job between spawn and assignment.
//! - All handles are closed on every path (`ScriptLaunch` owns the job handle
//!   via `Drop`; process/thread handles are closed immediately after resume).

use crate::engine_core::is_cmd_executable;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use windows::core::{HRESULT, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, ERROR_MORE_DATA, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicProcessIdList,
    QueryInformationJobObject, JOBOBJECT_BASIC_PROCESS_ID_LIST,
};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::System::Threading::{
    CreateProcessW, ResumeThread, TerminateProcess, CREATE_NO_WINDOW, CREATE_SUSPENDED,
    PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
};

/// A successfully spawned script. Owns the job handle so it is closed exactly
/// once on every path (including early returns and panics via `Drop`). The job
/// object itself may be closed after launch: children remain in it and are NOT
/// killed because `KILL_ON_JOB_CLOSE` is never set.
pub struct ScriptLaunch {
    job: HANDLE,
    /// PID of the spawned `cmd.exe`. Excluded from window attribution because
    /// its console is the launcher's own UI, not the application window.
    pub cmd_pid: u32,
}

impl ScriptLaunch {
    /// Borrow the job handle for `QueryInformationJobObject` during polling.
    pub fn job_handle(&self) -> HANDLE {
        self.job
    }
}

impl Drop for ScriptLaunch {
    fn drop(&mut self) {
        // SAFETY: `job` is a valid handle returned by CreateJobObjectW and owned
        // exclusively by this struct. Drop runs once, so it is closed once.
        if let Err(e) = unsafe { CloseHandle(self.job) } {
            tracing::warn!(error = %e, "CloseHandle failed for launcher job object");
        }
    }
}

/// Resolve the command interpreter from `ComSpec`, validating that it is an
/// existing `cmd.exe`; fall back to `GetSystemDirectoryW()\cmd.exe`.
///
/// An arbitrary environment variable is never trusted blindly: the basename
/// must be `cmd.exe` (case-insensitive) and the file must exist.
pub fn resolve_comspec() -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("ComSpec") {
        let candidate = PathBuf::from(&value);
        if is_cmd_executable(&value) && candidate.is_file() {
            return Ok(candidate);
        }
        tracing::warn!(
            value = %value,
            "ComSpec is not a valid existing cmd.exe; using the system directory"
        );
    }

    // SAFETY: `buf` is a local, correctly aligned `u16` array passed as a slice
    // to GetSystemDirectoryW, which writes at most `buf.len()` units plus a NUL
    // terminator and returns the number of units written (excluding NUL). If the
    // return value exceeds the buffer length the call failed (buffer too small)
    // and we bail out. No pointer escapes this block.
    let mut buf = [0u16; 260]; // MAX_PATH
    let len = unsafe { GetSystemDirectoryW(Some(&mut buf)) } as usize;
    if len == 0 || len > buf.len() {
        return Err("could not determine the Windows system directory for cmd.exe".to_string());
    }
    let dir = OsString::from_wide(&buf[..len]);
    let mut path = PathBuf::from(dir);
    path.push("cmd.exe");
    if !path.is_file() {
        return Err(format!("cmd.exe not found at {}", path.display()));
    }
    Ok(path)
}

/// Spawn `cmd.exe /c "<script>"` in a fresh Job Object, suspended until the
/// process is assigned. The script path is validated by the caller (pure
/// `validate_script_path`) so wrapping it in quotes makes it a single, safe
/// argv element. The child's working directory is the script's parent.
///
/// `show_console = true` leaves the console visible; `false` sets
/// `CREATE_NO_WINDOW`. This flag never affects EXE launches.
pub fn spawn_script(script_path: &str, show_console: bool) -> Result<ScriptLaunch, String> {
    let comspec = resolve_comspec()?;
    // Resolve to an absolute path and require an existing file. Without this,
    // a missing script would still spawn cmd.exe successfully and we would
    // poll for 8 s before reporting a misleading "no eligible window".
    let script = std::path::absolute(script_path)
        .map_err(|e| format!("could not resolve launcher script path: {e}"))?;
    if !script.is_file() {
        return Err(format!("launcher script not found: {}", script.display()));
    }
    let working_dir = script
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .ok_or_else(|| format!("launcher script '{script_path}' has no parent directory"))?;

    // cmd.exe is the explicit application name (avoids the Program.exe
    // space-in-path trap). The command line's first token repeats it in quotes.
    let cmdline = format!("\"{}\" /c \"{}\"", comspec.display(), script.display());
    let mut cmdline_wide: Vec<u16> = OsString::from(&cmdline)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let app_wide: Vec<u16> = comspec
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let work_wide: Vec<u16> = working_dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let mut flags = CREATE_SUSPENDED;
    if !show_console {
        flags |= CREATE_NO_WINDOW;
    }

    // SAFETY: all buffers are local, NUL-terminated `u16` vectors that live for
    // the whole call. `CreateProcessW` may modify `cmdline_wide`, hence the
    // mutable pointer. `startup` and `pi` are correctly aligned locals whose
    // addresses match the API's expected types. Every handle is closed on all
    // paths below; the job handle is transferred to `ScriptLaunch` on success.
    unsafe {
        let job = CreateJobObjectW(None, PCWSTR::null())
            .map_err(|e| format!("CreateJobObjectW failed: {e}"))?;

        let created = CreateProcessW(
            PCWSTR(app_wide.as_ptr()),
            Some(PWSTR(cmdline_wide.as_mut_ptr())),
            None, // default process attributes
            None, // default thread attributes
            false,
            PROCESS_CREATION_FLAGS(flags.0),
            None, // inherit parent environment
            PCWSTR(work_wide.as_ptr()),
            &startup,
            &mut pi,
        );
        if let Err(e) = created {
            let _ = CloseHandle(job);
            return Err(format!("CreateProcessW failed: {e}"));
        }

        // Assign BEFORE resuming: no child can be created outside the job.
        if let Err(e) = AssignProcessToJobObject(job, pi.hProcess) {
            // The process is still suspended — terminate it so we never leak a
            // stuck cmd.exe, then close every handle we own on this error path.
            let _ = TerminateProcess(pi.hProcess, 1);
            let _ = CloseHandle(pi.hThread);
            let _ = CloseHandle(pi.hProcess);
            let _ = CloseHandle(job);
            return Err(format!("AssignProcessToJobObject failed: {e}"));
        }

        // Resume the primary thread so the script actually runs.
        if ResumeThread(pi.hThread) == u32::MAX {
            let _ = TerminateProcess(pi.hProcess, 1);
            let _ = CloseHandle(pi.hThread);
            let _ = CloseHandle(pi.hProcess);
            let _ = CloseHandle(job);
            return Err("ResumeThread failed".to_string());
        }

        // We do not need the process/thread handles after resume: attribution
        // uses the job, not the cmd.exe PID's process handle.
        let _ = CloseHandle(pi.hThread);
        let _ = CloseHandle(pi.hProcess);

        Ok(ScriptLaunch {
            job,
            cmd_pid: pi.dwProcessId,
        })
    }
}

/// Query the authoritative set of PIDs currently in the job.
///
/// `JOBOBJECT_BASIC_PROCESS_ID_LIST` has a one-element flexible array. Two
/// details are easy to get wrong and would silently return an empty list:
/// 1. `NumberOfAssignedProcesses` is an INPUT capacity as well as an output
///    count — it must be set to the number of PIDs the buffer can hold.
/// 2. The `cbJobObjectInformationLength` argument must be the TRUE byte size of
///    the whole buffer, not just `size_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>()`.
///
/// We allocate a generously sized buffer and grow on `ERROR_MORE_DATA`. The
/// growth uses both the count the API reports and a plain doubling, so it is
/// correct regardless of which of those the API writes on that error path.
pub fn query_job_pids(job: HANDLE) -> Result<Vec<u32>, String> {
    const MAX_PIDS: usize = 4096;
    let mut capacity: usize = 8;

    for _ in 0..12 {
        // Allocate as `usize` words so the buffer has the alignment required by
        // JOBOBJECT_BASIC_PROCESS_ID_LIST (which contains `usize` PIDs).
        let needed_bytes = std::mem::size_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>()
            + (capacity - 1) * std::mem::size_of::<usize>();
        let words = needed_bytes.div_ceil(std::mem::size_of::<usize>());
        let mut buf = vec![0usize; words];
        let buf_bytes = (buf.len() * std::mem::size_of::<usize>()) as u32;

        // INPUT capacity for this query.
        // SAFETY: `buf` is a live, correctly aligned allocation large enough for
        // a JOBOBJECT_BASIC_PROCESS_ID_LIST header.
        unsafe {
            (*buf.as_mut_ptr().cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>())
                .NumberOfAssignedProcesses = capacity as u32;
        }

        // SAFETY: `buf` is a live, correctly aligned `usize` vector; `buf_bytes`
        // is its true byte length. `QueryInformationJobObject` writes at most
        // that many bytes. We read the PID slice only after a successful call
        // and clamp the count to the allocated capacity.
        let result = unsafe {
            QueryInformationJobObject(
                Some(job),
                JobObjectBasicProcessIdList,
                buf.as_mut_ptr() as *mut _,
                buf_bytes,
                None,
            )
        };

        match result {
            Ok(()) => {
                // SAFETY: after a successful call the buffer begins with a valid
                // JOBOBJECT_BASIC_PROCESS_ID_LIST. `n` is clamped to the
                // allocated PID capacity, so `from_raw_parts` stays in bounds.
                return Ok(unsafe {
                    let list = &*buf.as_ptr().cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>();
                    let n = (list.NumberOfProcessIdsInList as usize).min(capacity);
                    std::slice::from_raw_parts(list.ProcessIdList.as_ptr(), n)
                        .iter()
                        .map(|&p| p as u32)
                        .collect()
                });
            }
            Err(e) if e.code() == HRESULT::from_win32(ERROR_MORE_DATA.0) => {
                // Prefer the count the API reported; double as a fallback in
                // case this Windows version leaves the field untouched.
                let reported = unsafe {
                    (*buf.as_ptr().cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>())
                        .NumberOfAssignedProcesses as usize
                };
                capacity = reported.max(capacity.saturating_mul(2)).min(MAX_PIDS);
            }
            Err(e) => return Err(format!("QueryInformationJobObject failed: {e}")),
        }
    }
    Err(format!("job PID list exceeded {MAX_PIDS} entries"))
}
