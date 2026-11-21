use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegDeleteValueW, RegOpenKeyW, RegQueryValueW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, REG_SZ,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_NAME: &str = "ZoneFlowStudio";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Enable or disable launching the app at Windows login by writing/removing the
/// Run registry value. The registry key is unchanged from the original code.
pub fn set_auto_start(enable: bool) {
    // Resolve the executable path before touching the registry so a failure can
    // be reported and handled without leaving an open key behind. When disabling,
    // no path is needed (the registry value is simply deleted).
    let exe = if enable {
        match std::env::current_exe() {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to resolve current exe path for auto-start");
                return;
            }
        }
    } else {
        None
    };

    // SAFETY: `key_path`, `name` and `v` are UTF-16 vectors kept alive for the
    // whole block, so their `PCWSTR`/byte pointers stay valid for every registry
    // call. `hkey` starts as a null handle and is only used after
    // `RegCreateKeyW` succeeds; `RegCloseKey` is called on it exactly once on
    // every path after that. `bytes` is a byte view over the live `v` buffer
    // with the correct length (`v.len() * 2` bytes for UTF-16 code units). No
    // pointer escapes this block and all calls are synchronous.
    unsafe {
        let key_path = wide(RUN_KEY);
        let mut hkey: HKEY = HKEY::default();
        if RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(key_path.as_ptr()), &mut hkey).is_err() {
            return;
        }
        let name = wide(APP_NAME);
        if let Some(exe) = exe {
            let value = format!("\"{}\"", exe.to_string_lossy());
            let v = wide(&value);
            let bytes: &[u8] = std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2);
            if let Err(e) =
                RegSetValueExW(hkey, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes)).ok()
            {
                tracing::warn!(error = %e, "RegSetValueExW failed while enabling auto-start");
            }
        } else if let Err(e) = RegDeleteValueW(hkey, PCWSTR(name.as_ptr())).ok() {
            tracing::warn!(error = %e, "RegDeleteValueW failed while disabling auto-start");
        }
        if let Err(e) = RegCloseKey(hkey).ok() {
            tracing::warn!(error = %e, "RegCloseKey failed in set_auto_start");
        }
    }
}

/// Returns `true` when the Run registry value for this app is present, i.e. the
/// app is configured to start at Windows login. Used to initialise the Settings
/// toggle from the actual system state (the config value is the authoritative
/// source that `set_auto_start` applies at startup).
#[allow(dead_code)] // consumed by the rebuilt Settings UI (Phase 4/5)
pub fn is_auto_start() -> bool {
    // SAFETY: `key_path` and `name` are UTF-16 vectors kept alive for the whole
    // block, so their `PCWSTR` pointers stay valid for every registry call.
    // `hkey` starts as a null handle and is only used after `RegOpenKeyW`
    // succeeds; `RegCloseKey` is called on it exactly once afterwards. `len` is
    // a local `i32` whose address and size match the DWORD `RegQueryValueW`
    // expects. No pointer escapes this block and all calls are synchronous.
    unsafe {
        let key_path = wide(RUN_KEY);
        let mut hkey: HKEY = HKEY::default();
        if RegOpenKeyW(HKEY_CURRENT_USER, PCWSTR(key_path.as_ptr()), &mut hkey).is_err() {
            return false;
        }
        let name = wide(APP_NAME);
        // Passing `None` for the data buffer makes RegQueryValueW only report the
        // value's size in `len`, which is enough to test for existence.
        let mut len: i32 = 0;
        let status = RegQueryValueW(hkey, PCWSTR(name.as_ptr()), None, Some(&mut len));
        if let Err(e) = RegCloseKey(hkey).ok() {
            tracing::warn!(error = %e, "RegCloseKey failed in is_auto_start");
        }
        status.is_ok()
    }
}
