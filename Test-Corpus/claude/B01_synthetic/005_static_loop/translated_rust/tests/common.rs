// Common helpers for the C-vs-Rust FFI comparison tests.
//
// These tests load BOTH the C .so and the Rust .so via libloading and
// compare their externally observable behavior through their C ABI exports.

#![allow(dead_code)]

use std::path::PathBuf;

pub fn c_so_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = manifest.join("c_src").join("build").join("libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built at {:?}; run cmake first",
        p
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    // build.rs builds the cdylib (with `export_main`) into a sibling
    // target directory and emits its absolute path via this env var.
    // We MUST use that build, not target/debug/libdriver.so, because
    // the regular cargo test build of the cdylib does NOT enable the
    // `export_main` feature and therefore would not export `main`.
    let path = env!("DRIVER_RUST_CDYLIB_PATH");
    let p = PathBuf::from(path);
    assert!(
        p.exists(),
        "Rust shared library not found at {:?} (expected from build.rs)",
        p
    );
    p
}

// Serialize tests that touch the static state in `static_sum`.
//
// The C library uses a process-global `static int sum`, while the Rust
// translation uses a `thread_local!` cell. To compare them apples-to-
// apples, every test runs single-threaded under this lock and runs both
// the C and Rust call sequence on the same OS thread.
//
// `cargo test` would otherwise run tests across multiple threads,
// which would expose the C/Rust storage-class difference.
pub static SUM_LOCK: Mutex<()> = Mutex::new(());
