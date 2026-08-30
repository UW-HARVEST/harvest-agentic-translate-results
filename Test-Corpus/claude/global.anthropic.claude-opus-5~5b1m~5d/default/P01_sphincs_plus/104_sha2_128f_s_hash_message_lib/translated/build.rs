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

    // Backend priority (default: haraka).
    let backend = if feature("haraka") {
        "haraka"
    } else if feature("sha2") {
        "sha2"
    } else if feature("shake") {
        "shake"
    } else if feature("blake") {
        "blake"
    } else {
        "haraka"
    };

    // thash variant priority (default: robust).
    let thash = if feature("robust") {
        "robust"
    } else if feature("simple") {
        "simple"
    } else {
        "robust"
    };

    // Security parameter priority (default: 128s).
    let secpar = if feature("128s") {
        "128s"
    } else if feature("128f") {
        "128f"
    } else if feature("192s") {
        "192s"
    } else if feature("192f") {
        "192f"
    } else if feature("256s") {
        "256s"
    } else if feature("256f") {
        "256f"
    } else {
        "128s"
    };

    println!("cargo:rustc-cfg=spx_backend=\"{}\"", backend);
    println!("cargo:rustc-cfg=spx_thash=\"{}\"", thash);
    println!("cargo:rustc-cfg=spx_secpar=\"{}\"", secpar);

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
