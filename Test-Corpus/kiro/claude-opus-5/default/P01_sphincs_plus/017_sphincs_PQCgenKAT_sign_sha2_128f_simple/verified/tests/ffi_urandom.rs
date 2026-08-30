//! Tests for the `urandom` feature, i.e. the configuration where the exported
//! `randombytes` is the `/dev/urandom` reader from `app/src/randombytes.c`
//! instead of the `rng.c` DRBG.  On the C side that is the difference between
//! the `sphincs_core` and `sphincs_core_det` shared libraries.
//!
//! The output is genuinely random, so it cannot be compared byte for byte.
//! What *is* compared is the observable contract: how many bytes get written,
//! that nothing outside the requested range is touched, and that the signature
//! API still works end to end and interoperates with the C build.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::os::raw::c_int;

/// `void randombytes(unsigned char *x, unsigned long long xlen)` in
/// `randombytes.c`; `int randombytes(...)` in `rng.c`.  The return value is
/// only inspected in the DRBG configuration (tests/ffi_high.rs), so the void
/// shape is used here.
type FnRandombytesVoid = unsafe extern "C" fn(*mut u8, u64);

#[test]
fn urandom_randombytes_writes_exactly_xlen() {
    if !URANDOM {
        return;
    }
    let l = libs();
    let c = unsafe { l.c_urandom::<FnRandombytesVoid>("randombytes") };
    let r = unsafe { l.r::<FnRandombytesVoid>("randombytes") };

    const PAD: usize = 16;
    for xlen in [0u64, 1, 15, 16, 17, 48, 100, 1000] {
        for (who, f) in [("C", &c), ("Rust", &r)] {
            let n = xlen as usize;
            let mut buf = vec![0xAAu8; n + PAD];
            unsafe { f(buf.as_mut_ptr(), xlen) };
            assert!(
                buf[n..].iter().all(|&b| b == 0xAA),
                "{who} randombytes wrote past xlen={xlen}"
            );
            if n >= 32 {
                // Filling with real entropy: an all-0xAA result would mean the
                // buffer was not written at all.
                assert!(
                    !buf[..n].iter().all(|&b| b == 0xAA),
                    "{who} randombytes did not fill xlen={xlen}"
                );
            }
        }
    }

    // Successive calls must differ (both implementations read fresh entropy).
    for (who, f) in [("C", &c), ("Rust", &r)] {
        let mut a = vec![0u8; 64];
        let mut b = vec![0u8; 64];
        unsafe {
            f(a.as_mut_ptr(), 64);
            f(b.as_mut_ptr(), 64);
        }
        assert_ne!(a, b, "{who} randombytes repeated itself");
    }
}

/// The deterministic half of the API is unaffected by the random source, so it
/// must still match the C build exactly.
#[test]
fn urandom_seed_keypair_still_matches() {
    if !URANDOM {
        return;
    }
    let l = libs();
    type FnSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
    let c = unsafe { l.c_urandom::<FnSeedKeypair>("crypto_sign_seed_keypair") };
    let r = unsafe { l.r::<FnSeedKeypair>("crypto_sign_seed_keypair") };
    let mut rng = Rng::new(0x2001);
    for round in 0..2 {
        let seed = if round == 0 {
            vec![0u8; 3 * SPX_N]
        } else {
            rng.vec(3 * SPX_N)
        };
        let mut cpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut csk = vec![0xAAu8; SPX_SK_BYTES + 8];
        let mut rpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut rsk = vec![0xAAu8; SPX_SK_BYTES + 8];
        let (cr, rr) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        assert_eq!(cr, rr, "crypto_sign_seed_keypair return value");
        assert_bytes_eq(&format!("seed_keypair pk (round={round})"), &cpk, &rpk);
        assert_bytes_eq(&format!("seed_keypair sk (round={round})"), &csk, &rsk);
    }
}

/// Signatures differ (random `optrand`), but each build must accept the other's.
#[test]
fn urandom_signatures_interoperate() {
    if !URANDOM {
        return;
    }
    let l = libs();
    type FnSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
    type FnSignature =
        unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
    type FnVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;

    let ckp = unsafe { l.c_urandom::<FnSeedKeypair>("crypto_sign_seed_keypair") };
    let rkp = unsafe { l.r::<FnSeedKeypair>("crypto_sign_seed_keypair") };
    let csig = unsafe { l.c_urandom::<FnSignature>("crypto_sign_signature") };
    let rsig = unsafe { l.r::<FnSignature>("crypto_sign_signature") };
    let cver = unsafe { l.c_urandom::<FnVerify>("crypto_sign_verify") };
    let rver = unsafe { l.r::<FnVerify>("crypto_sign_verify") };

    let mut rng = Rng::new(0x2002);
    let seed = rng.vec(3 * SPX_N);
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
    }
    assert_eq!(cpk, rpk, "public keys must be identical");
    assert_eq!(csk, rsk, "secret keys must be identical");

    for mlen in [0usize, 33] {
        let m = rng.vec(mlen);
        let mut cs = vec![0u8; SPX_BYTES];
        let mut rs = vec![0u8; SPX_BYTES];
        let mut cl = 0usize;
        let mut rl = 0usize;
        unsafe {
            csig(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, csk.as_ptr());
            rsig(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, rsk.as_ptr());
        }
        assert_eq!(cl, SPX_BYTES);
        assert_eq!(rl, SPX_BYTES);
        for (name, sig) in [("C sig", &cs), ("Rust sig", &rs)] {
            let (cv, rv) = unsafe {
                (
                    cver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, cpk.as_ptr()),
                    rver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, rpk.as_ptr()),
                )
            };
            assert_eq!(cv, 0, "C verify rejected {name} (mlen={mlen})");
            assert_eq!(rv, 0, "Rust verify rejected {name} (mlen={mlen})");
        }
    }
}
