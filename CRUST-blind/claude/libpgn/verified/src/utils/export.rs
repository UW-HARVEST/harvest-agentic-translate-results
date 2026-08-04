#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // Windows-specific export: nothing to do beyond declaration.
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // Other platforms (Linux, macOS, etc.): nothing to do.
}
