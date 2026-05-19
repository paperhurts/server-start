//! "Start with Windows" toggle, backed by the per-user HKCU Run key.
//!
//! The registry is the single source of truth — no config-file shadow,
//! no in-memory cache. `is_enabled()` reads the key fresh on every call;
//! `enable()` overwrites the path each time so re-toggling after moving
//! the .exe fixes a stale entry automatically.
//!
//! All public functions are Windows-only because the Run key only exists
//! on Windows. The module is gated behind `#[cfg(windows)]` at the use
//! site in main.rs — no stubs needed.

use std::env;

use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "ServerStart";

/// True if our value is present under HKCU Run. We don't compare the stored
/// path against the current executable — "value exists" means the user has
/// asked for autostart, and `enable()` keeps the path current.
pub fn is_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match hkcu.open_subkey(RUN_KEY) {
        Ok(key) => key.get_value::<String, _>(VALUE_NAME).is_ok(),
        Err(_) => false,
    }
}

/// Write the current executable's path (quoted, to survive spaces in any
/// parent directory) into HKCU Run. NTFS rejects `"` in path components,
/// so we also reject it explicitly here — anything that slips through
/// would break `CreateProcess` parsing of the Run-key value silently.
pub fn enable() -> Result<(), String> {
    let exe = env::current_exe()
        .map_err(|e| format!("Failed to resolve current executable: {}", e))?;
    let raw = exe.to_string_lossy();
    if raw.contains('"') {
        return Err(format!(
            "Executable path contains a double-quote character and cannot be safely registered: {}",
            raw
        ));
    }
    let value = format!("\"{}\"", raw);

    // Open the Run key with the minimum privilege we need (KEY_SET_VALUE).
    // Fall back to create_subkey only if the key is missing — a stripped
    // Windows image edge case; on a real install the key always exists.
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
        Ok(k) => k,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            hkcu.create_subkey(RUN_KEY)
                .map_err(|e| format!("Failed to create Run key: {}", e))?
                .0
        }
        Err(e) => return Err(format!("Failed to open Run key: {}", e)),
    };
    key.set_value(VALUE_NAME, &value)
        .map_err(|e| format!("Failed to write registry value: {}", e))
}

/// Delete our value from HKCU Run. Idempotent — returns Ok if the value
/// was never there to begin with.
pub fn disable() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE) {
        Ok(k) => k,
        Err(_) => return Ok(()),
    };
    match key.delete_value(VALUE_NAME) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to delete registry value: {}", e)),
    }
}
