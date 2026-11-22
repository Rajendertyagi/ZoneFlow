//! Minimal Win32 clipboard helper for the tray "Copy editor URL" action.
//!
//! Uses only the `windows` crate already in the dependency tree — no new
//! high-level clipboard dependency. The text is placed on the clipboard as
//! `CF_UNICODETEXT` using `OpenClipboard` / `EmptyClipboard` /
//! `SetClipboardData`.

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// Encode `text` as a null-terminated UTF-16 buffer (the `CF_UNICODETEXT`
/// wire format). Factored out so the encoding is unit-testable without
/// touching the real clipboard.
fn utf16_with_nul(text: &str) -> Vec<u16> {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    wide
}

/// Copy `text` to the system clipboard as `CF_UNICODETEXT`.
///
/// Ownership of the global memory block transfers to the system on a
/// successful `SetClipboardData`; every failure path frees it ourselves. The
/// clipboard is always closed before returning.
pub fn copy_text(text: &str) -> Result<(), String> {
    // SAFETY: every handle below is created and validated locally. `wide` is a
    // null-terminated UTF-16 buffer that outlives the copy into global memory;
    // the global block is either handed to the system or freed by us, never
    // both. `OpenClipboard(None)` uses no caller-owned window.
    unsafe {
        let wide = utf16_with_nul(text);
        let byte_len = wide.len() * std::mem::size_of::<u16>();

        let hglobal: HGLOBAL =
            GlobalAlloc(GMEM_MOVEABLE, byte_len).map_err(|e| format!("GlobalAlloc failed: {e}"))?;

        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            let _ = GlobalFree(Some(hglobal));
            return Err("GlobalLock failed".to_string());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr.cast::<u16>(), wide.len());
        // A failed unlock means the lock count was already zero — the block is
        // unlocked either way and remains valid for SetClipboardData.
        let _ = GlobalUnlock(hglobal);

        if let Err(e) = OpenClipboard(None) {
            let _ = GlobalFree(Some(hglobal));
            return Err(format!("OpenClipboard failed: {e}"));
        }

        if let Err(e) = EmptyClipboard() {
            let _ = CloseClipboard();
            let _ = GlobalFree(Some(hglobal));
            return Err(format!("EmptyClipboard failed: {e}"));
        }

        // On success the system owns `hglobal`; on failure we still own it.
        if let Err(e) = SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(hglobal.0))) {
            let _ = CloseClipboard();
            let _ = GlobalFree(Some(hglobal));
            return Err(format!("SetClipboardData failed: {e}"));
        }

        let _ = CloseClipboard();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_buffer_is_null_terminated() {
        let wide = utf16_with_nul("abc");
        assert_eq!(wide, vec![b'a' as u16, b'b' as u16, b'c' as u16, 0]);
    }

    #[test]
    fn utf16_buffer_handles_non_ascii() {
        let wide = utf16_with_nul("é");
        assert_eq!(wide, vec![0x00E9, 0]);
    }
}
