#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // Windows-specific export logic - placeholder, no logic in C either.
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // Other platforms (Linux, macOS, etc.) - placeholder, no logic in C either.
}
