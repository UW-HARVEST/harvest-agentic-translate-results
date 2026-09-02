//! Link configuration for the `driver` cdylib.
//!
//! The C library is linked by CMake with the toolchain defaults: partial RELRO
//! and **lazy** PLT binding (`R_X86_64_JUMP_SLOT` entries for `printLine`,
//! `bad`, `good`, `puts`). rustc instead defaults to `-z relro -z now`, which
//! resolves everything at load time.
//!
//! That difference is observable here: `bad()` prints an uninitialized stack
//! word (CWE-457), and with lazy binding the first PLT call inside `driver`
//! runs `_dl_runtime_resolve`, which overwrites exactly that word. Requesting
//! `-z lazy` reproduces the C's binding behaviour, and hence its output.
//!
//! These are emitted from `build.rs` rather than `.cargo/config.toml` so they
//! survive a `RUSTFLAGS` override in the environment.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo::rustc-cdylib-link-arg=-Wl,-z,lazy");
    }
}
