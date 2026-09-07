//! Harness smoke test: both `.so`s load and agree on the size constants.

mod common;
use common::*;

#[test]
fn smoke_00_libraries_load() {
    let p = libs();
    assert!(p.c.has("crypto_sign_bytes"), "C lib missing crypto_sign_bytes");
    assert!(p.r.has("crypto_sign_bytes"), "Rust lib missing crypto_sign_bytes");
    eprintln!("config {} loaded, CTX_BYTES={}", cfg_name(), CTX_BYTES);
}

#[test]
fn smoke_01_ctx_size_matches_c_layout() {
    // Independently recompute sizeof(spx_ctx) from context.h instead of
    // trusting the Rust struct.
    let base = 2 * SPX_N;
    let expected = if BACKEND == "sha2" {
        base + 40 + if SPX_N >= 24 { 72 } else { 0 }
    } else if BACKEND == "haraka" {
        // uint64_t[10][8] + uint32_t[10][8], struct align 8
        let raw = base + 640 + 320;
        (raw + 7) / 8 * 8
    } else {
        base
    };
    assert_eq!(CTX_BYTES, expected, "spx_ctx size mismatch for {}", cfg_name());
}

#[test]
fn smoke_02_size_getters_agree() {
    let p = libs();
    for name in [
        "crypto_sign_secretkeybytes",
        "crypto_sign_publickeybytes",
        "crypto_sign_bytes",
        "crypto_sign_seedbytes",
    ] {
        let c: FnU64Void = p.c.f(name);
        let r: FnU64Void = p.r.f(name);
        unsafe { eq(name, 0, c(), r()) };
    }
}
