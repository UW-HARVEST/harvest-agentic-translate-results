#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // Windows-specific export logic — declared in C as a no-op for DLL export
    // visibility. In Rust there is nothing to do.
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // Other platforms (Linux, macOS, etc.) — no-op.
}
