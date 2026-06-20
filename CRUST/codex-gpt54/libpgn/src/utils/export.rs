#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // No-op in the Rust port.
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // No-op in the Rust port.
}
