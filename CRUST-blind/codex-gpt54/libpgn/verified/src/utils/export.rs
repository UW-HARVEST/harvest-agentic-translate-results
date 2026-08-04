#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // Windows-specific export logic
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // Other platforms (Linux, macOS, etc.)
}
