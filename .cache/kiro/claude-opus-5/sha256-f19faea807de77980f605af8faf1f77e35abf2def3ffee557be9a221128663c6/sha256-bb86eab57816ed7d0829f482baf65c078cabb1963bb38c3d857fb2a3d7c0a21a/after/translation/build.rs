//! Resolves the CMake cache variables (HASH_BACKEND / THASH / SECPAR) that are
//! exposed as additive Cargo features into a single mutually exclusive `cfg`
//! for each choice.
//!
//! Cargo features are additive, so `--features sha2` on top of the default
//! feature set leaves `haraka` enabled as well.  The precedence below makes the
//! non-default value win, so that both
//!   `cargo build --features sha2`
//! and
//!   `cargo build --no-default-features --features "sha2,robust,128s"`
//! select the SHA2 backend.

use std::env;

fn has(feature: &str) -> bool {
    env::var_os(format!(
        "CARGO_FEATURE_{}",
        feature.to_uppercase().replace('-', "_")
    ))
    .is_some()
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    for name in [
        "backend_haraka",
        "backend_sha2",
        "backend_shake",
        "backend_blake",
        "thash_robust",
        "thash_simple",
        "secpar_128s",
        "secpar_128f",
        "secpar_192s",
        "secpar_192f",
        "secpar_256s",
        "secpar_256f",
        "spx_n_ge_24",
        "rand_urandom",
        "rand_drbg",
    ] {
        println!("cargo:rustc-check-cfg=cfg({})", name);
    }

    // ---- HASH_BACKEND (default: haraka) -------------------------------
    let backend = if has("blake") {
        "backend_blake"
    } else if has("shake") {
        "backend_shake"
    } else if has("sha2") {
        "backend_sha2"
    } else {
        "backend_haraka"
    };
    println!("cargo:rustc-cfg={}", backend);

    // ---- THASH (default: robust) -------------------------------------
    let thash = if has("simple") {
        "thash_simple"
    } else {
        "thash_robust"
    };
    println!("cargo:rustc-cfg={}", thash);

    // ---- SECPAR (default: 128s) --------------------------------------
    let (secpar, n_ge_24) = if has("256f") {
        ("secpar_256f", true)
    } else if has("256s") {
        ("secpar_256s", true)
    } else if has("192f") {
        ("secpar_192f", true)
    } else if has("192s") {
        ("secpar_192s", true)
    } else if has("128f") {
        ("secpar_128f", false)
    } else {
        ("secpar_128s", false)
    };
    println!("cargo:rustc-cfg={}", secpar);
    if n_ge_24 {
        println!("cargo:rustc-cfg=spx_n_ge_24");
    }

    // ---- randombytes() provider (default: deterministic DRBG) --------
    if has("urandom") {
        println!("cargo:rustc-cfg=rand_urandom");
    } else {
        println!("cargo:rustc-cfg=rand_drbg");
    }
}
