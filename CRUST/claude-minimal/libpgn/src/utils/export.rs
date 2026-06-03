#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // Windows-specific export logic; matches the C `__pgn_export()` no-op.
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // Other platforms (Linux, macOS, etc.); matches the C `__pgn_export()` no-op.
}
