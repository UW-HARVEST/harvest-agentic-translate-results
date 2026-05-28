//! Shared helpers for the integration tests.
//!
//! Each test loads BOTH the C and the Rust shared libraries via `libloading`
//! and compares their exported symbols byte-for-byte.

use libloading::{Library, Symbol};
use std::path::PathBuf;

/// Locate the freshly built C `.so` (built by `cmake`).
pub fn c_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("c_src/build/libdriver.so")
}

/// Locate the Rust `cdylib` produced by `cargo build`.
pub fn rust_lib_path() -> PathBuf {
    // Tests run with `cargo test`, which puts artefacts in target/debug.
    // We still need the *cdylib* artefact, which `cargo test --lib` does
    // build into target/debug/. Prefer release if present (faster).
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let release = manifest.join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest.join("target/debug/libdriver.so")
}

/// Convenience: open both libraries.
pub fn open_libs() -> (Library, Library) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("failed to load C .so");
        let rust_lib = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        (c_lib, rust_lib)
    }
}

/// Look up a symbol with the given prototype in a library.
pub unsafe fn sym<'a, T>(lib: &'a Library, name: &[u8]) -> Symbol<'a, T> {
    lib.get(name).unwrap_or_else(|e| {
        panic!(
            "symbol {:?} not found: {}",
            std::str::from_utf8(name).unwrap_or("?"),
            e
        )
    })
}

/// Compare two slices of `f32` byte-for-byte (same bit pattern, NaN-safe).
pub fn assert_eq_bits(a: &[f32], b: &[f32], ctx: &str) {
    assert_eq!(a.len(), b.len(), "len mismatch in {}", ctx);
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x.to_bits(),
            y.to_bits(),
            "bits differ at index {} in {}: c={:?} ({:#x}) vs rust={:?} ({:#x})",
            i, ctx, x, x.to_bits(), y, y.to_bits()
        );
    }
}

pub fn assert_eq_bits_one(a: f32, b: f32, ctx: &str) {
    assert_eq!(
        a.to_bits(),
        b.to_bits(),
        "bits differ in {}: c={:?} ({:#x}) vs rust={:?} ({:#x})",
        ctx, a, a.to_bits(), b, b.to_bits()
    );
}
