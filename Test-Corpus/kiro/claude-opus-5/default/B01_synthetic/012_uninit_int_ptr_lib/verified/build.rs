//! Match the link-time configuration of the original C shared library.
//!
//! `c_src/CMakeLists.txt` builds `libdriver.so` with the system compiler's
//! defaults, which on this toolchain means **partial RELRO with lazy PLT
//! binding** (`readelf -d` shows a `GNU_RELRO` segment but no `BIND_NOW`).
//! rustc, by contrast, defaults to full RELRO / `-z now`.
//!
//! For an ordinary library that difference is invisible. It is not invisible
//! here: `bad()` prints whatever happens to be in an uninitialized stack slot,
//! and with lazy binding the *first* call through a given PLT entry detours into
//! `_dl_runtime_resolve`, which consumes a large amount of stack below the
//! caller's frame and therefore changes exactly the bytes `bad()` goes on to
//! read.
//!
//! Measured before this flag was added (`driver(0)` called three times in one
//! process, same harness, C `.so` vs Rust `.so`):
//!
//!     C:    1137197056   608471368   608471368
//!     Rust:  608471368   608471368   608471368
//!
//! The two agree from the second call onwards and disagree only on the first —
//! the call that runs the resolver in the lazily-bound C library. Requesting
//! lazy binding for the Rust `cdylib` removes the divergence.
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo::rustc-link-arg-cdylib=-Wl,-z,lazy");
        // The C `.so` records `NEEDED libc.so.6` (it calls `printf`). A `no_std`
        // cdylib leaves `printf` undefined with no recorded dependency, which
        // would make resolution depend on libc happening to be in the loader's
        // global scope. Record the same dependency the C library does.
        println!("cargo::rustc-link-arg-cdylib=-Wl,--no-as-needed");
        println!("cargo::rustc-link-arg-cdylib=-lc");
    }
}
