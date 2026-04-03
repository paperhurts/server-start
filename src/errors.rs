//! User-visible error notifications.
//! In release mode (no console), uses MessageBoxW. In debug mode, also prints to stderr.

/// Show an error dialog to the user. Always safe to call — no-ops gracefully on failure.
pub fn show_error(title: &str, message: &str) {
    // Always try stderr for debug builds / log capture
    eprintln!("[{}] {}", title, message);

    #[cfg(windows)]
    {
        show_message_box(title, message, MB_ICONERROR);
    }
}

/// Show a yes/no confirmation dialog. Returns true if user clicks Yes.
pub fn confirm(title: &str, message: &str) -> bool {
    #[cfg(windows)]
    {
        show_message_box(title, message, MB_YESNO | MB_ICONWARNING) == IDYES
    }

    #[cfg(not(windows))]
    {
        let _ = (title, message);
        false
    }
}

#[cfg(windows)]
const MB_ICONERROR: u32 = 0x00000010;
#[cfg(windows)]
const MB_YESNO: u32 = 0x00000004;
#[cfg(windows)]
const MB_ICONWARNING: u32 = 0x00000030;
#[cfg(windows)]
const IDYES: i32 = 6;

#[cfg(windows)]
fn show_message_box(title: &str, message: &str, flags: u32) -> i32 {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    let title_w = to_wide(title);
    let message_w = to_wide(message);

    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            None,
            windows::core::PCWSTR(message_w.as_ptr()),
            windows::core::PCWSTR(title_w.as_ptr()),
            windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE(flags),
        )
        .0
    }
}
