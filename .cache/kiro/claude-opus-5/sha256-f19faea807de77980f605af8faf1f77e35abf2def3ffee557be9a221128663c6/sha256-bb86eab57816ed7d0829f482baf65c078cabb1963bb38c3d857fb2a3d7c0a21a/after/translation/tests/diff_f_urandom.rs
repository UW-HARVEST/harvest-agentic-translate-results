//! CONFIGS.md row F2 — the `urandom` feature, i.e. the `randombytes()` from
//! `c_src/app/src/randombytes.c` (CMake target `sphincs_core`) instead of the
//! NIST DRBG from `rng.c` (`sphincs_core_det`).
//!
//! This provider reads `/dev/urandom`, so its output is not reproducible and a
//! byte-for-byte comparison is impossible by construction.  What *is* checkable
//! is the contract: the exported symbol has the `void (unsigned char *,
//! unsigned long long)` signature, it fills exactly `xlen` bytes and no more,
//! `xlen == 0` writes nothing, and it never blocks.  Both `.so`s are held to
//! the same contract.

mod common;

use common::*;

/// `void randombytes(unsigned char *x, unsigned long long xlen)`
type UrandomFn = unsafe extern "C" fn(*mut u8, u64);

#[cfg(rand_urandom)]
#[test]
fn f2_urandom_randombytes_contract() {
    let libs = Libs::load();
    // With the `urandom` feature `Libs::c` resolves `randombytes` in
    // `libsphincs_core.so`, i.e. the same provider the Rust build exports.
    let (c, r) = pair!(libs, "randombytes", UrandomFn);

    for &xlen in &[0u64, 1, 15, 16, 17, 31, 32, 48, 63, 64, 65, 1000, 4096] {
        let n = xlen as usize;
        let mut cb = vec![0xAAu8; n + 16];
        let mut rb = vec![0xAAu8; n + 16];
        unsafe {
            c(cb.as_mut_ptr(), xlen);
            r(rb.as_mut_ptr(), xlen);
        }
        // Guard bytes past `xlen` must be untouched on both sides.
        assert!(
            cb[n..].iter().all(|&b| b == 0xAA),
            "C randombytes wrote past xlen={xlen}"
        );
        assert!(
            rb[n..].iter().all(|&b| b == 0xAA),
            "Rust randombytes wrote past xlen={xlen}"
        );
        if xlen == 0 {
            assert!(cb.iter().all(|&b| b == 0xAA));
            assert!(rb.iter().all(|&b| b == 0xAA));
            continue;
        }
        // With >= 16 bytes of /dev/urandom, an all-0xAA result is
        // astronomically unlikely; this catches a provider that never writes.
        if n >= 16 {
            assert!(
                !cb[..n].iter().all(|&b| b == 0xAA),
                "C randombytes did not fill {xlen} bytes"
            );
            assert!(
                !rb[..n].iter().all(|&b| b == 0xAA),
                "Rust randombytes did not fill {xlen} bytes"
            );
            // Two independent draws must differ.
            let mut cb2 = vec![0xAAu8; n];
            let mut rb2 = vec![0xAAu8; n];
            unsafe {
                c(cb2.as_mut_ptr(), xlen);
                r(rb2.as_mut_ptr(), xlen);
            }
            assert_ne!(&cb[..n], &cb2[..], "C randombytes repeated itself");
            assert_ne!(&rb[..n], &rb2[..], "Rust randombytes repeated itself");
        }
    }
}

/// `crypto_sign_keypair` / `crypto_sign_signature` must still work end to end
/// with the non-deterministic provider, and each side's signature must verify
/// under *both* implementations.
#[cfg(rand_urandom)]
#[test]
fn f2_urandom_sign_verify_interop() {
    use std::os::raw::c_int;
    type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
    type SignatureFn =
        unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
    type VerifyFn = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;

    let libs = Libs::load();
    let (ckp, rkp) = pair!(libs, "crypto_sign_keypair", KeypairFn);
    let (cs, rs) = pair!(libs, "crypto_sign_signature", SignatureFn);
    let (cv, rv) = pair!(libs, "crypto_sign_verify", VerifyFn);
    let mut rng = Rng::new(1001);

    for &mlen in &[0usize, 1, 64, 137, 1000] {
        let m = rng.bytes(mlen.max(1));
        // Each implementation generates its own key pair from /dev/urandom.
        for side in 0..2 {
            let mut pk = vec![0u8; SPX_PK_BYTES];
            let mut sk = vec![0u8; SPX_SK_BYTES];
            let mut sig = vec![0u8; SPX_BYTES];
            let mut l = 0usize;
            unsafe {
                let kr = if side == 0 {
                    ckp(pk.as_mut_ptr(), sk.as_mut_ptr())
                } else {
                    rkp(pk.as_mut_ptr(), sk.as_mut_ptr())
                };
                eq("crypto_sign_keypair ret", kr, 0);
                let sr = if side == 0 {
                    cs(sig.as_mut_ptr(), &mut l, m.as_ptr(), mlen, sk.as_ptr())
                } else {
                    rs(sig.as_mut_ptr(), &mut l, m.as_ptr(), mlen, sk.as_ptr())
                };
                eq("crypto_sign_signature ret", sr, 0);
                eq("siglen", l, SPX_BYTES);
                eq(
                    &format!("C verifies side-{side} signature (mlen={mlen})"),
                    cv(sig.as_ptr(), l, m.as_ptr(), mlen, pk.as_ptr()),
                    0,
                );
                eq(
                    &format!("Rust verifies side-{side} signature (mlen={mlen})"),
                    rv(sig.as_ptr(), l, m.as_ptr(), mlen, pk.as_ptr()),
                    0,
                );
            }
        }
    }
}

/// In the default (DRBG) build the exported `randombytes` is `rng.c`'s, which
/// returns `int`; `diff_e_api.rs::e10_e11_randombytes_drbg` covers it
/// byte-for-byte.  Assert here only that the two providers are distinguishable,
/// so this file is meaningful in both modes.
#[cfg(rand_drbg)]
#[test]
fn f2_drbg_provider_selected() {
    let libs = Libs::load();
    let _: libloading::os::unix::Symbol<UrandomFn> = libs.r("randombytes");
    // rng.c-only symbols must be present in the default build.
    for name in [
        "randombytes_init",
        "seedexpander",
        "seedexpander_init",
        "AES256_ECB",
        "AES256_CTR_DRBG_Update",
    ] {
        let _: libloading::os::unix::Symbol<*const u8> = libs.r(name);
        let _: libloading::os::unix::Symbol<*const u8> = libs.c(name);
    }
}
