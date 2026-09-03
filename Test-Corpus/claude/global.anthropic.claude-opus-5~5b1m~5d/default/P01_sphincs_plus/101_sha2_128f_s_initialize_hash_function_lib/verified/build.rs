// Build script that resolves the CMake-style cache variables (HASH_BACKEND,
// THASH, SECPAR) from Cargo features into a single active selection, using a
// deterministic priority order with a fallback to the CMake defaults. This
// guarantees that ALL combinations of features compile (exactly one backend,
// one thash variant and one secpar is ever selected at a time).

use std::env;

fn feature(name: &str) -> bool {
    // Cargo exposes features as CARGO_FEATURE_<NAME> with the name uppercased
    // and any non-alphanumeric character replaced by '_'.
    let key = format!(
        "CARGO_FEATURE_{}",
        name.to_uppercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
    );
    env::var(key).is_ok()
}

fn main() {
    // Declare all cfgs we may emit so the compiler's cfg checking stays quiet.
    println!("cargo:rustc-check-cfg=cfg(spx_backend, values(\"haraka\", \"sha2\", \"shake\", \"blake\"))");
    println!("cargo:rustc-check-cfg=cfg(spx_thash, values(\"robust\", \"simple\"))");
    println!("cargo:rustc-check-cfg=cfg(spx_secpar, values(\"128s\", \"128f\", \"192s\", \"192f\", \"256s\", \"256f\"))");
    println!("cargo:rustc-check-cfg=cfg(spx_sha512)");
    println!("cargo:rustc-check-cfg=cfg(spx_blake512)");
    println!("cargo:rustc-check-cfg=cfg(spx_big_hash)");

    // IMPORTANT: the precedence rules below must match, exactly, the
    // `#[cfg(feature = ...)]` precedence hard-coded in `src/params.rs`,
    // `src/context.rs` and `src/backend/mod.rs`, otherwise a combination that
    // enables more than one backend / thash / secpar feature would select
    // inconsistent constants and struct layouts.

    // Backend priority, matching src/backend/mod.rs (CMake default: haraka).
    let backend = if feature("sha2") {
        "sha2"
    } else if feature("shake") {
        "shake"
    } else if feature("blake") {
        "blake"
    } else {
        "haraka"
    };

    // thash variant priority, matching src/backend/thash_*.rs, which key off
    // `feature = "simple"` / `not(feature = "simple")` (CMake default: robust).
    let thash = if feature("simple") { "simple" } else { "robust" };

    // Security parameter priority, matching the `mod secpar` cascade in
    // src/params.rs (CMake default: 128s).
    let secpar = if feature("256f") {
        "256f"
    } else if feature("256s") {
        "256s"
    } else if feature("192f") {
        "192f"
    } else if feature("192s") {
        "192s"
    } else if feature("128f") {
        "128f"
    } else {
        "128s"
    };

    println!("cargo:rustc-cfg=spx_backend=\"{}\"", backend);
    println!("cargo:rustc-cfg=spx_thash=\"{}\"", thash);
    println!("cargo:rustc-cfg=spx_secpar=\"{}\"", secpar);

    // `SPX_BIG_HASH` / `SPX_SHA512` / `SPX_BLAKE512` is 1 exactly for the
    // 192- and 256-bit security levels, independently of the backend.  This is
    // what guards the `state_seeded_512` field of `spx_ctx` in `context.h`
    // (`# if SPX_SHA512`) and the `thash_512` code paths in `thash_*.c`
    // (`#if SPX_BLAKE512` / `#if SPX_SHA512`).
    if secpar.starts_with("192") || secpar.starts_with("256") {
        println!("cargo:rustc-cfg=spx_big_hash");
    }

    // SPX_SHA512 is only defined by the SHA2 parameter sets, and only for the
    // 192/256-bit security levels.
    if backend == "sha2" && (secpar.starts_with("192") || secpar.starts_with("256")) {
        println!("cargo:rustc-cfg=spx_sha512");
    }

    // SPX_BLAKE512 is defined by the BLAKE parameter sets: 0 for 128-bit,
    // 1 for the 192/256-bit levels (which use BLAKE-512 for multi-block thash
    // and for message hashing).
    if backend == "blake" && (secpar.starts_with("192") || secpar.starts_with("256")) {
        println!("cargo:rustc-cfg=spx_blake512");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
