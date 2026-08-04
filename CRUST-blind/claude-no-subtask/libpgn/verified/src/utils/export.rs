#[cfg(target_os = "windows")]
pub fn __pgn_export() {
    // Windows-specific export logic. The C export.c is essentially empty
    // (it only re-declares an extern void function), so this is a no-op.
}

#[cfg(not(target_os = "windows"))]
pub fn __pgn_export() {
    // Other platforms (Linux, macOS, etc.) — no-op, matches the C implementation.
}
