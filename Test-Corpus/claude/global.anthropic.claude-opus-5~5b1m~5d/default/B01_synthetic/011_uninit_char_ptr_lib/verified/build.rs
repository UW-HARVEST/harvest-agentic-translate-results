//! Link-time configuration needed for behavioural parity with the C library.
//!
//! `cmake` builds `libdriver.so` with the plain default `gcc` driver, which on
//! this platform produces a shared object with **lazy PLT binding** (no
//! `DF_BIND_NOW`). `rustc`, by contrast, defaults to full RELRO + `-z now`.
//!
//! That difference is normally invisible, but this library's `bad()` reproduces
//! the original's CWE-457 defect: it reads an *indeterminate* stack slot that
//! sits below the frame, i.e. exactly in the region the dynamic linker's lazy
//! symbol resolver (`_dl_runtime_resolve`) scribbles over on the first call
//! through a PLT entry. With `-z now` that resolver never runs and the slot
//! holds different bytes, so the library prints different output than the C for
//! the very same call sequence.
//!
//! Requesting lazy binding restores the C's link-time configuration, and with it
//! byte-identical output (verified by the differential tests in `tests/`).

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "linux" && target_env == "gnu" {
        // Match `gcc`'s default: lazy PLT binding, partial RELRO.
        println!("cargo::rustc-link-arg=-Wl,-z,lazy");
        println!("cargo::rustc-link-arg=-Wl,-z,norelro");
    }
}
