//! Shared lazy terminal-image helpers.
//!
//! `Picker::from_query_stdio()` performs terminal capability queries over
//! stdio and can hang when stdio is not a TTY (tests, CI, pipes). Every
//! page that renders images must go through these helpers instead of
//! constructing a `Picker` directly.

use ratatui_image::picker::Picker;
use std::sync::{Arc, OnceLock};

static SHARED: OnceLock<Arc<Picker>> = OnceLock::new();

/// Best-effort TTY detection for the crossterm stdout handle on Windows/Unix.
fn stdio_is_tty() -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
        let handle = std::io::stdout().as_raw_handle();
        if handle.is_null() || handle as isize == INVALID_HANDLE_VALUE as isize {
            return false;
        }
        let mut mode = 0u32;
        // Safe: probing console mode is the standard TTY check on Windows.
        unsafe { windows_sys::Win32::System::Console::GetConsoleMode(handle, &mut mode) != 0 }
    }
    #[cfg(not(windows))]
    {
        // Safe: isatty is a trivial libc call.
        unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
    }
}

/// Process-wide picker, created lazily on first use.
///
/// When stdio is not a TTY (tests, CI, pipes) or the terminal does not
/// support image protocols, falls back to `Picker::halfblocks()` which
/// never performs I/O queries.
pub fn shared_picker() -> Arc<Picker> {
    SHARED
        .get_or_init(|| {
            let picker = if stdio_is_tty() {
                Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
            } else {
                Picker::halfblocks()
            };
            Arc::new(picker)
        })
        .clone()
}

/// Whether the shared picker can render real terminal images (queried TTY).
pub fn picker_supports_images() -> bool {
    stdio_is_tty()
}
