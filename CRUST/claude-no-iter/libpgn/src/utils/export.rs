#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // Windows-specific export marker — no runtime behavior in Rust.
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // Other platforms — no runtime behavior in Rust.
}
