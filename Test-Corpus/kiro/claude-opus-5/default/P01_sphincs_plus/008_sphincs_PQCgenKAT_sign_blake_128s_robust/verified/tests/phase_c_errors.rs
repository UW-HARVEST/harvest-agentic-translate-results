//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Every row asserts the *same* sentinel /
//! error code from both implementations, not merely that both failed.

mod common;

use common::*;
use std::ffi::{c_int, c_uint};

// ---------------------------------------------------------------------------
// E1 / B3 — crypto_sign_verify rejects any siglen != SPX_BYTES
// ---------------------------------------------------------------------------

#[test]
fn err_e1_verify_bad_siglen() {
    type Ver = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
    let (cver, rver) = libs().pair::<Ver>("crypto_sign_verify");
    let mut rng = Rng::new(SEED ^ 101);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, _csk, rpk, _rsk) = keypair(&seed);
    let sig = rng.bytes(SPX_BYTES);
    let m = rng.bytes(64);

    for siglen in [
        0usize,
        1,
        SPX_N,
        SPX_BYTES - 1,
        SPX_BYTES + 1,
        SPX_BYTES * 2,
        usize::MAX,
    ] {
        for (label, pk) in [("C pk", &cpk), ("R pk", &rpk)] {
            let cv = unsafe { cver(sig.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()) };
            let rv = unsafe { rver(sig.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()) };
            same_val(
                &format!("crypto_sign_verify(siglen={siglen}, {label})"),
                cv,
                rv,
            );
            same_val(
                &format!("crypto_sign_verify(siglen={siglen}) must be -1"),
                cv,
                -1,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E2 — crypto_sign_verify rejects a root mismatch
// ---------------------------------------------------------------------------

#[test]
fn err_e2_verify_root_mismatch() {
    type Sig = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
    type Ver = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
    let (csig_f, _rsig_f) = libs().pair::<Sig>("crypto_sign_signature");
    let (cver, rver) = libs().pair::<Ver>("crypto_sign_verify");
    let mut rng = Rng::new(SEED ^ 102);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, csk, rpk, _rsk) = keypair(&seed);
    let m = rng.bytes(64);

    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
    seed_both_drbgs(&entropy, None);
    let mut sig = vec![0u8; SPX_BYTES];
    let mut siglen = 0usize;
    let ok = unsafe {
        csig_f(
            sig.as_mut_ptr(),
            &mut siglen,
            m.as_ptr(),
            m.len(),
            csk.as_ptr(),
        )
    };
    same_val("signing succeeded", ok, 0);

    // 1. corrupt one byte of the signature, at several positions
    for pos in [
        0usize,
        SPX_N,
        SPX_N + SPX_FORS_BYTES / 2,
        SPX_N + SPX_FORS_BYTES,
        SPX_BYTES - 1,
    ] {
        let mut bad = sig.clone();
        bad[pos] ^= 0x01;
        for (label, pk) in [("C pk", &cpk), ("R pk", &rpk)] {
            let cv = unsafe { cver(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
            let rv = unsafe { rver(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
            same_val(&format!("verify(corrupt sig byte {pos}, {label})"), cv, rv);
            same_val(&format!("verify(corrupt sig byte {pos}) must be -1"), cv, -1);
        }
    }

    // 2. corrupt the message.
    //
    // NOTE (real behaviour of the C, faithfully reproduced by the Rust):
    // `lib/blake/src/hash_blake.c` calls `blakeX_update(&S, m, mlen)` while
    // `blake256_update`/`blake512_update` take their length in *bits* and
    // `memcpy` only `datalen >> 3` bytes.  With the blake backend the message
    // therefore influences the signature only through its first `mlen / 8` bytes,
    // and only when `mlen` is a multiple of 8 (otherwise `blake*_final` never
    // reaches a compression and the digest degenerates to the IV).  Verified
    // empirically for mlen ∈ {1,16,32,64,100,128,200}; the other three backends
    // absorb every byte.
    //
    // The differential property asserted for *every* position is that C and Rust
    // agree; rejection is asserted only over the range the C actually absorbs.
    let absorbed = if IS_BLAKE {
        if m.len() % 8 == 0 {
            m.len() / 8
        } else {
            0
        }
    } else {
        m.len()
    };
    for pos in 0..m.len() {
        let mut badm = m.clone();
        badm[pos] ^= 0x80;
        let cv = unsafe { cver(sig.as_ptr(), SPX_BYTES, badm.as_ptr(), badm.len(), cpk.as_ptr()) };
        let rv = unsafe { rver(sig.as_ptr(), SPX_BYTES, badm.as_ptr(), badm.len(), cpk.as_ptr()) };
        same_val(&format!("verify(corrupt message byte {pos})"), cv, rv);
        if pos < absorbed {
            same_val(
                &format!("verify(corrupt message byte {pos}) must be -1"),
                cv,
                -1,
            );
        }
    }
    assert!(
        absorbed > 0,
        "the test message length must be one the backend actually absorbs"
    );

    // 2b. a different message length is a different message.  For blake this
    // also changes `buflen`, so it is detected there too, but only agreement is
    // required.
    for shorter in [1usize, 8] {
        let n = m.len() - shorter;
        let cv = unsafe { cver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), n, cpk.as_ptr()) };
        let rv = unsafe { rver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), n, cpk.as_ptr()) };
        same_val(&format!("verify(mlen={n} instead of {})", m.len()), cv, rv);
    }

    // 3. wrong public key (only the root half differs)
    let mut badpk = cpk.clone();
    badpk[SPX_N] ^= 0x01;
    let cv = unsafe { cver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), badpk.as_ptr()) };
    let rv = unsafe { rver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), badpk.as_ptr()) };
    same_val("verify(corrupt pk root)", cv, rv);
    same_val("verify(corrupt pk root) must be -1", cv, -1);

    // 4. wrong pub_seed half of the public key
    let mut badpk2 = cpk.clone();
    badpk2[0] ^= 0x01;
    let cv = unsafe { cver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), badpk2.as_ptr()) };
    let rv = unsafe { rver(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), badpk2.as_ptr()) };
    same_val("verify(corrupt pk seed)", cv, rv);
    same_val("verify(corrupt pk seed) must be -1", cv, -1);

    // 5. a completely random signature
    let junk = rng.bytes(SPX_BYTES);
    let cv = unsafe { cver(junk.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), cpk.as_ptr()) };
    let rv = unsafe { rver(junk.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), cpk.as_ptr()) };
    same_val("verify(random sig)", cv, rv);
    same_val("verify(random sig) must be -1", cv, -1);
}

// ---------------------------------------------------------------------------
// E3 / B3 — crypto_sign_open with smlen < SPX_BYTES
// ---------------------------------------------------------------------------

#[test]
fn err_e3_open_short_smlen() {
    type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
    let (copen, ropen) = libs().pair::<Open>("crypto_sign_open");
    let mut rng = Rng::new(SEED ^ 103);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, _csk, rpk, _rsk) = keypair(&seed);
    let sm = rng.bytes(SPX_BYTES + 64);

    for smlen in [0u64, 1, SPX_N as u64, (SPX_BYTES - 1) as u64] {
        for (label, pk) in [("C pk", &cpk), ("R pk", &rpk)] {
            // The C memsets `smlen` bytes of `m`, so give it room and a
            // recognizable fill so the zeroing is observable.
            let mut cm = vec![0x5Au8; SPX_BYTES + 128];
            let mut rm = vec![0x5Au8; SPX_BYTES + 128];
            let mut cml = u64::MAX;
            let mut rml = u64::MAX;
            let cv = unsafe { copen(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), smlen, pk.as_ptr()) };
            let rv = unsafe { ropen(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), smlen, pk.as_ptr()) };
            same_val(&format!("crypto_sign_open(smlen={smlen}, {label}) ret"), cv, rv);
            same_val(&format!("crypto_sign_open(smlen={smlen}) must be -1"), cv, -1);
            same_val(&format!("crypto_sign_open(smlen={smlen}) *mlen"), cml, rml);
            same_val(&format!("crypto_sign_open(smlen={smlen}) *mlen == 0"), cml, 0);
            same(&format!("crypto_sign_open(smlen={smlen}) m buffer"), &cm, &rm);
            // The C zeroes exactly `smlen` bytes and nothing beyond.
            assert!(
                cm[..smlen as usize].iter().all(|&b| b == 0),
                "expected the first {smlen} bytes of m to be zeroed"
            );
            assert!(
                cm[smlen as usize..].iter().all(|&b| b == 0x5A),
                "expected bytes past {smlen} to be untouched"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E4 — crypto_sign_open where the inner verify fails
// ---------------------------------------------------------------------------

#[test]
fn err_e4_open_verify_fail() {
    type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
    type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
    let (csign, _rsign) = libs().pair::<Sign>("crypto_sign");
    let (copen, ropen) = libs().pair::<Open>("crypto_sign_open");
    let mut rng = Rng::new(SEED ^ 104);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, csk, rpk, _rsk) = keypair(&seed);
    let mlen = 64u64;
    let m = rng.bytes(mlen as usize);

    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
    seed_both_drbgs(&entropy, None);
    let mut sm = vec![0u8; SPX_BYTES + mlen as usize];
    let mut smlen = 0u64;
    unsafe {
        csign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), mlen, csk.as_ptr());
    }
    same_val("smlen", smlen, SPX_BYTES as u64 + mlen);

    for pos in [0usize, SPX_N, SPX_BYTES - 1, SPX_BYTES, SPX_BYTES + 5] {
        let mut bad = sm.clone();
        bad[pos] ^= 0x01;
        for (label, pk) in [("C pk", &cpk), ("R pk", &rpk)] {
            let mut cm = vec![0x5Au8; SPX_BYTES + 128];
            let mut rm = vec![0x5Au8; SPX_BYTES + 128];
            let mut cml = u64::MAX;
            let mut rml = u64::MAX;
            let cv = unsafe { copen(cm.as_mut_ptr(), &mut cml, bad.as_ptr(), smlen, pk.as_ptr()) };
            let rv = unsafe { ropen(rm.as_mut_ptr(), &mut rml, bad.as_ptr(), smlen, pk.as_ptr()) };
            same_val(&format!("open(corrupt at {pos}, {label}) ret"), cv, rv);
            same_val(&format!("open(corrupt at {pos}) *mlen"), cml, rml);
            same(&format!("open(corrupt at {pos}) m buffer"), &cm, &rm);
            // mlen = 64 makes every backend absorb the whole message, so both a
            // signature-byte and a message-byte flip must be rejected.
            same_val(&format!("open(corrupt at {pos}) must be -1"), cv, -1);
            same_val(&format!("open(corrupt at {pos}) *mlen == 0"), cml, 0);
            assert!(
                cm[..smlen as usize].iter().all(|&b| b == 0),
                "expected the first smlen bytes of m to be zeroed"
            );
            assert!(
                cm[smlen as usize..].iter().all(|&b| b == 0x5A),
                "expected bytes past smlen to be untouched"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B2 — crypto_sign_open with smlen exactly SPX_BYTES (zero-length message)
// ---------------------------------------------------------------------------

#[test]
fn err_b2_open_exact_smlen() {
    type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
    type Open = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
    let (csign, _rsign) = libs().pair::<Sign>("crypto_sign");
    let (copen, ropen) = libs().pair::<Open>("crypto_sign_open");
    let mut rng = Rng::new(SEED ^ 105);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, csk, rpk, _rsk) = keypair(&seed);
    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
    seed_both_drbgs(&entropy, None);

    let mut sm = vec![0u8; SPX_BYTES];
    let mut smlen = 0u64;
    let m: [u8; 0] = [];
    unsafe {
        csign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), 0, csk.as_ptr());
    }
    same_val("smlen == SPX_BYTES", smlen, SPX_BYTES as u64);

    for (label, pk) in [("C pk", &cpk), ("R pk", &rpk)] {
        let mut cm = vec![0x5Au8; 64];
        let mut rm = vec![0x5Au8; 64];
        let mut cml = u64::MAX;
        let mut rml = u64::MAX;
        let cv = unsafe { copen(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), smlen, pk.as_ptr()) };
        let rv = unsafe { ropen(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), smlen, pk.as_ptr()) };
        same_val(&format!("open(smlen==SPX_BYTES, {label}) ret"), cv, rv);
        same_val("open(smlen==SPX_BYTES) accepts", cv, 0);
        same_val("open(smlen==SPX_BYTES) *mlen", cml, rml);
        same_val("open(smlen==SPX_BYTES) *mlen == 0", cml, 0);
        same("open(smlen==SPX_BYTES) m untouched", &cm, &rm);
        assert!(cm.iter().all(|&b| b == 0x5A), "m must not be written");
    }
}

// ---------------------------------------------------------------------------
// E5 / B4 / B5 — seedexpander_init maxlen bounds
// ---------------------------------------------------------------------------

#[test]
fn err_e5_seedexpander_init_maxlen() {
    type Init = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, u64) -> c_int;
    let (cinit, rinit) = libs().pair::<Init>("seedexpander_init");
    let mut rng = Rng::new(SEED ^ 106);
    let mut seed = rng.bytes(32);
    let mut div = rng.bytes(8);

    for (maxlen, expect) in [
        (0u64, 0i32),
        (1, 0),
        (0xFFFF_FFFEu64, 0),
        (0xFFFF_FFFFu64, 0),
        (0x1_0000_0000u64, -1), // RNG_BAD_MAXLEN, first rejected value
        (0x1_0000_0001u64, -1),
        (u64::MAX, -1),
    ] {
        // Pre-fill with a recognizable pattern so "ctx untouched" is observable.
        let mut cs = AesXof::zeroed();
        cs.buffer = [0x11; 16];
        cs.buffer_pos = 0x2222_2222;
        cs.length_remaining = 0x3333_3333;
        cs.key = [0x44; 32];
        cs.ctr = [0x55; 16];
        let mut rs = cs;

        let cv = unsafe { cinit(&mut cs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
        let rv = unsafe { rinit(&mut rs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen) };
        same_val(&format!("seedexpander_init(maxlen={maxlen:#x}) ret"), cv, rv);
        same_val(
            &format!("seedexpander_init(maxlen={maxlen:#x}) expected code"),
            cv,
            expect,
        );
        same_val(&format!("seedexpander_init(maxlen={maxlen:#x}) ctx"), cs, rs);
        if expect == -1 {
            assert_eq!(cs.key, [0x44; 32], "ctx must be untouched on rejection");
            assert_eq!(cs.ctr, [0x55; 16], "ctx must be untouched on rejection");
            assert_eq!(cs.buffer_pos, 0x2222_2222);
        }
    }
}

#[test]
fn err_b5_seedexpander_init_zero() {
    type Init = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, u64) -> c_int;
    type Exp = unsafe extern "C" fn(*mut AesXof, *mut u8, u64) -> c_int;
    let (cinit, rinit) = libs().pair::<Init>("seedexpander_init");
    let (cexp, rexp) = libs().pair::<Exp>("seedexpander");
    let mut rng = Rng::new(SEED ^ 107);
    let mut seed = rng.bytes(32);
    let mut div = rng.bytes(8);

    let mut cs = AesXof::zeroed();
    let mut rs = AesXof::zeroed();
    let cv = unsafe { cinit(&mut cs, seed.as_mut_ptr(), div.as_mut_ptr(), 0) };
    let rv = unsafe { rinit(&mut rs, seed.as_mut_ptr(), div.as_mut_ptr(), 0) };
    same_val("seedexpander_init(0) ret", cv, rv);
    same_val("seedexpander_init(0) accepted", cv, 0);
    same_val("seedexpander_init(0) ctx", cs, rs);

    // length_remaining == 0, so *every* request (including 0) is rejected.
    for xlen in [0u64, 1, 16] {
        let mut cb = vec![0xAAu8; 32];
        let mut rb = vec![0xAAu8; 32];
        let cr = unsafe { cexp(&mut cs, cb.as_mut_ptr(), xlen) };
        let rr = unsafe { rexp(&mut rs, rb.as_mut_ptr(), xlen) };
        same_val(&format!("seedexpander(len_rem=0, xlen={xlen}) ret"), cr, rr);
        same_val(
            &format!("seedexpander(len_rem=0, xlen={xlen}) == RNG_BAD_REQ_LEN"),
            cr,
            -3,
        );
        same(&format!("seedexpander(len_rem=0, xlen={xlen}) out"), &cb, &rb);
        same_val(&format!("seedexpander(len_rem=0, xlen={xlen}) ctx"), cs, rs);
    }
}

// ---------------------------------------------------------------------------
// E6 — seedexpander with a NULL output buffer
// ---------------------------------------------------------------------------

#[test]
fn err_e6_seedexpander_null_out() {
    type Init = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, u64) -> c_int;
    type Exp = unsafe extern "C" fn(*mut AesXof, *mut u8, u64) -> c_int;
    let (cinit, rinit) = libs().pair::<Init>("seedexpander_init");
    let (cexp, rexp) = libs().pair::<Exp>("seedexpander");
    let mut rng = Rng::new(SEED ^ 108);
    let mut seed = rng.bytes(32);
    let mut div = rng.bytes(8);

    // maxlen = 4096, so the length check would pass for small xlen: the NULL
    // check must be the one that fires.  Also try xlen values that would
    // *additionally* fail the length check, to confirm NULL wins.
    for (maxlen, xlen) in [
        (4096u64, 0u64),
        (4096, 1),
        (4096, 100),
        (4096, 4096),  // would also be RNG_BAD_REQ_LEN
        (4096, 99999), // would also be RNG_BAD_REQ_LEN
        (0, 0),        // would also be RNG_BAD_REQ_LEN
    ] {
        let mut cs = AesXof::zeroed();
        let mut rs = AesXof::zeroed();
        unsafe {
            cinit(&mut cs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            rinit(&mut rs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
        }
        let before = cs;
        let cr = unsafe { cexp(&mut cs, std::ptr::null_mut(), xlen) };
        let rr = unsafe { rexp(&mut rs, std::ptr::null_mut(), xlen) };
        same_val(
            &format!("seedexpander(NULL, maxlen={maxlen}, xlen={xlen}) ret"),
            cr,
            rr,
        );
        same_val(
            &format!("seedexpander(NULL, maxlen={maxlen}, xlen={xlen}) == RNG_BAD_OUTBUF"),
            cr,
            -2,
        );
        same_val("seedexpander(NULL) ctx", cs, rs);
        same_val("seedexpander(NULL) ctx untouched", cs, before);
    }
}

// ---------------------------------------------------------------------------
// E7 / B7 — seedexpander length budget (note the `>=`)
// ---------------------------------------------------------------------------

#[test]
fn err_e7_seedexpander_req_len() {
    type Init = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, u64) -> c_int;
    type Exp = unsafe extern "C" fn(*mut AesXof, *mut u8, u64) -> c_int;
    let (cinit, rinit) = libs().pair::<Init>("seedexpander_init");
    let (cexp, rexp) = libs().pair::<Exp>("seedexpander");
    let mut rng = Rng::new(SEED ^ 109);
    let mut seed = rng.bytes(32);
    let mut div = rng.bytes(8);

    for maxlen in [1u64, 16, 100, 4096] {
        for (xlen, expect) in [
            (maxlen - 1, 0i32),  // largest accepted request
            (maxlen, -3),        // `>=` makes the exact budget an error
            (maxlen + 1, -3),
            (u64::MAX, -3),
        ] {
            let mut cs = AesXof::zeroed();
            let mut rs = AesXof::zeroed();
            unsafe {
                cinit(&mut cs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
                rinit(&mut rs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            }
            let before = cs;
            // Only allocate for the accepted case; a rejected call must not write.
            let cap = if expect == 0 { xlen as usize + 8 } else { 8 };
            let mut cb = vec![0xAAu8; cap];
            let mut rb = vec![0xAAu8; cap];
            let cr = unsafe { cexp(&mut cs, cb.as_mut_ptr(), xlen) };
            let rr = unsafe { rexp(&mut rs, rb.as_mut_ptr(), xlen) };
            same_val(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ret"),
                cr,
                rr,
            );
            same_val(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) expected"),
                cr,
                expect,
            );
            same(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) out"),
                &cb,
                &rb,
            );
            same_val(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ctx"),
                cs,
                rs,
            );
            if expect != 0 {
                same_val("rejected seedexpander leaves ctx untouched", cs, before);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B6 — seedexpander(xlen == 0) with budget left: accepted, writes nothing
// ---------------------------------------------------------------------------

#[test]
fn err_b6_seedexpander_zero_len() {
    type Init = unsafe extern "C" fn(*mut AesXof, *mut u8, *mut u8, u64) -> c_int;
    type Exp = unsafe extern "C" fn(*mut AesXof, *mut u8, u64) -> c_int;
    let (cinit, rinit) = libs().pair::<Init>("seedexpander_init");
    let (cexp, rexp) = libs().pair::<Exp>("seedexpander");
    let mut rng = Rng::new(SEED ^ 116);
    let mut seed = rng.bytes(32);
    let mut div = rng.bytes(8);

    for maxlen in [1u64, 16, 100, 4096] {
        let mut cs = AesXof::zeroed();
        let mut rs = AesXof::zeroed();
        unsafe {
            cinit(&mut cs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            rinit(&mut rs, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
        }
        let before = cs;
        let mut cb = [0xAAu8; 16];
        let mut rb = [0xAAu8; 16];
        let cr = unsafe { cexp(&mut cs, cb.as_mut_ptr(), 0) };
        let rr = unsafe { rexp(&mut rs, rb.as_mut_ptr(), 0) };
        same_val(&format!("seedexpander(maxlen={maxlen}, xlen=0) ret"), cr, rr);
        same_val(
            &format!("seedexpander(maxlen={maxlen}, xlen=0) == RNG_SUCCESS"),
            cr,
            0,
        );
        same(&format!("seedexpander(maxlen={maxlen}, xlen=0) out"), &cb, &rb);
        assert_eq!(cb, [0xAAu8; 16], "xlen == 0 must not write");
        same_val(&format!("seedexpander(maxlen={maxlen}, xlen=0) ctx"), cs, rs);
        // The `while (xlen > 0)` loop body never runs, so only
        // `length_remaining -= 0` happens: the state is otherwise unchanged.
        same_val("seedexpander(xlen=0) leaves the ctx unchanged", cs, before);
    }
}

// ---------------------------------------------------------------------------
// B8 — randombytes(xlen == 0) still advances the DRBG
// ---------------------------------------------------------------------------

#[test]
fn err_b8_randombytes_zero() {
    if URANDOM {
        return;
    }
    type F = unsafe extern "C" fn(*mut u8, u64) -> c_int;
    let (c, r) = libs().pair::<F>("randombytes");
    let (cdrbg, rdrbg) = data_pair::<DrbgCtx>("DRBG_ctx");
    let mut rng = Rng::new(SEED ^ 110);
    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
    seed_both_drbgs(&entropy, None);

    let before_c = unsafe { *cdrbg };
    let mut cb = [0xAAu8; 8];
    let mut rb = [0xAAu8; 8];
    let cv = unsafe { c(cb.as_mut_ptr(), 0) };
    let rv = unsafe { r(rb.as_mut_ptr(), 0) };
    same_val("randombytes(0) ret", cv, rv);
    same_val("randombytes(0) == RNG_SUCCESS", cv, 0);
    same("randombytes(0) writes nothing", &cb, &rb);
    assert_eq!(cb, [0xAAu8; 8], "randombytes(0) must not write");
    let after_c = unsafe { *cdrbg };
    let after_r = unsafe { *rdrbg };
    same_val("DRBG_ctx after randombytes(0)", after_c, after_r);
    // The trailing update still runs, so the state must have moved on.
    assert_ne!(
        before_c.key, after_c.key,
        "the C reference still reseeds on a zero-length request"
    );
    same_val("reseed_counter advanced", after_c.reseed_counter, 2);
}

// ---------------------------------------------------------------------------
// B10 — AES256_CTR_DRBG_Update with NULL provided_data
// ---------------------------------------------------------------------------

#[test]
fn err_b10_drbg_update_null() {
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    let (c, r) = libs().pair::<F>("AES256_CTR_DRBG_Update");
    let mut rng = Rng::new(SEED ^ 111);
    for v_fill in [None, Some(0xffu8), Some(0x00u8)] {
        let key = rng.bytes(32);
        let v = match v_fill {
            Some(f) => vec![f; 16],
            None => rng.bytes(16),
        };
        let mut ck = key.clone();
        let mut rk = key.clone();
        let mut cv = v.clone();
        let mut rv = v.clone();
        unsafe {
            c(std::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
            r(std::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        same("DRBG_Update(NULL) Key", &ck, &rk);
        same("DRBG_Update(NULL) V", &cv, &rv);
    }
}

// ---------------------------------------------------------------------------
// B11 — set_type with values outside the documented SPX_ADDR_TYPE_* range
// ---------------------------------------------------------------------------

#[test]
fn err_b11_set_type_out_of_range() {
    type F = unsafe extern "C" fn(*mut u32, u32);
    let (c, r) = libs().pair::<F>("SPX_set_type");
    let mut rng = Rng::new(SEED ^ 112);
    // 0..=6 are the valid variants; everything else is an out-of-range enum
    // value that C accepts (enums are just ints across the FFI boundary).
    let vals: Vec<u32> = vec![
        0, 1, 2, 3, 4, 5, 6, 7, 8, 100, 255, 256, 257, 511, 0x0000_FF00, 0x7FFF_FFFF, 0x8000_0000,
        0xFFFF_FF00, 0xFFFF_FFFF,
    ];
    for v in vals {
        let base = rand_addr(&mut rng);
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr() as *mut u32, v);
            r(ra.as_mut_ptr() as *mut u32, v);
        }
        same(&format!("set_type({v:#x})"), &ca, &ra);
        // Only the single type byte may change, and it holds the low 8 bits.
        let mut expect = base;
        expect[OFF_TYPE] = v as u8;
        same(&format!("set_type({v:#x}) truncates to the low byte"), &ca, &expect);
    }
}

// ---------------------------------------------------------------------------
// B12 — the other truncating single-byte address setters
// ---------------------------------------------------------------------------

#[test]
fn err_b12_addr_setters_truncate() {
    type F = unsafe extern "C" fn(*mut u32, u32);
    let mut rng = Rng::new(SEED ^ 113);
    for (name, off) in [
        ("SPX_set_layer_addr", OFF_LAYER),
        ("SPX_set_chain_addr", OFF_CHAIN_ADDR),
        ("SPX_set_hash_addr", OFF_HASH_ADDR),
        ("SPX_set_tree_height", OFF_TREE_HGT),
    ] {
        let (c, r) = libs().pair::<F>(name);
        for v in [
            0u32,
            1,
            255,
            256,
            257,
            0xFFFF,
            0x1_0000,
            0x7FFF_FFFF,
            0xFFFF_FFFF,
        ] {
            let base = rand_addr(&mut rng);
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr() as *mut u32, v);
                r(ra.as_mut_ptr() as *mut u32, v);
            }
            same(&format!("{name}({v:#x})"), &ca, &ra);
            let mut expect = base;
            expect[off] = v as u8;
            same(&format!("{name}({v:#x}) truncates"), &ca, &expect);
        }
    }
}

// ---------------------------------------------------------------------------
// B13 — ull_to_bytes at outlen 0 and beyond 8
// ---------------------------------------------------------------------------

#[test]
fn err_b13_ull_to_bytes_edge() {
    type F = unsafe extern "C" fn(*mut u8, c_uint, u64);
    let (c, r) = libs().pair::<F>("SPX_ull_to_bytes");
    for outlen in [0u32, 9, 10, 12, 16] {
        for v in [0u64, 1, u64::MAX, 0xDEAD_BEEF_CAFE_BABE] {
            let mut cb = vec![0xAAu8; 32];
            let mut rb = vec![0xAAu8; 32];
            unsafe {
                c(cb.as_mut_ptr(), outlen, v);
                r(rb.as_mut_ptr(), outlen, v);
            }
            same(&format!("ull_to_bytes(outlen={outlen}, {v:#x})"), &cb, &rb);
            if outlen == 0 {
                assert_eq!(cb, vec![0xAAu8; 32], "outlen == 0 must not write");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B14 — bytes_to_ull at inlen 0 and 8 (the documented range) and 9+
// ---------------------------------------------------------------------------

#[test]
fn err_b14_bytes_to_ull_edge() {
    type F = unsafe extern "C" fn(*const u8, c_uint) -> u64;
    let (c, r) = libs().pair::<F>("SPX_bytes_to_ull");
    let mut rng = Rng::new(SEED ^ 114);

    // inlen 0 and 8 are the documented boundaries and must agree exactly.
    for inlen in [0u32, 1, 7, 8] {
        for _ in 0..8 {
            let buf = rng.bytes(32);
            let cv = unsafe { c(buf.as_ptr(), inlen) };
            let rv = unsafe { r(buf.as_ptr(), inlen) };
            same_val(&format!("bytes_to_ull(inlen={inlen})"), cv, rv);
        }
        let z = vec![0u8; 32];
        same_val(
            &format!("bytes_to_ull(inlen={inlen}, zeros)"),
            unsafe { c(z.as_ptr(), inlen) },
            unsafe { r(z.as_ptr(), inlen) },
        );
        if inlen == 0 {
            same_val("bytes_to_ull(0) == 0", unsafe { c(z.as_ptr(), 0) }, 0u64);
        }
    }

    // inlen > 8 makes the C shift by >= 64, which is undefined behaviour in
    // both languages; not asserted.  Documented in ERRORS.md row B14.
}

// ---------------------------------------------------------------------------
// Generic FFI boundaries: zero-length messages through the whole API
// ---------------------------------------------------------------------------

#[test]
fn err_zero_length_message_paths() {
    type Sig = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
    type Ver = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
    let (csig_f, rsig_f) = libs().pair::<Sig>("crypto_sign_signature");
    let (cver, rver) = libs().pair::<Ver>("crypto_sign_verify");
    let mut rng = Rng::new(SEED ^ 115);

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let (cpk, csk, rpk, rsk) = keypair(&seed);
    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();

    // mlen == 0 with a *dangling but non-null* message pointer, which is what a
    // caller with an empty Vec produces.
    let empty: Vec<u8> = Vec::new();
    seed_both_drbgs(&entropy, None);
    let mut cs = vec![0u8; SPX_BYTES];
    let mut cl = 0usize;
    let cv = unsafe { csig_f(cs.as_mut_ptr(), &mut cl, empty.as_ptr(), 0, csk.as_ptr()) };
    if !URANDOM {
        seed_both_drbgs(&entropy, None);
    }
    let mut rs = vec![0u8; SPX_BYTES];
    let mut rl = 0usize;
    let rv = unsafe { rsig_f(rs.as_mut_ptr(), &mut rl, empty.as_ptr(), 0, rsk.as_ptr()) };
    same_val("sign(mlen=0) ret", cv, rv);
    same_val("sign(mlen=0) siglen", cl, rl);
    if !URANDOM {
        same("sign(mlen=0) sig", &cs, &rs);
    }
    for (label, sig, pk) in [
        ("C/C", &cs, &cpk),
        ("C/R", &cs, &rpk),
        ("R/C", &rs, &cpk),
        ("R/R", &rs, &rpk),
    ] {
        let a = unsafe { cver(sig.as_ptr(), SPX_BYTES, empty.as_ptr(), 0, pk.as_ptr()) };
        let b = unsafe { rver(sig.as_ptr(), SPX_BYTES, empty.as_ptr(), 0, pk.as_ptr()) };
        same_val(&format!("verify(mlen=0, {label})"), a, b);
        same_val(&format!("verify(mlen=0, {label}) accepts"), a, 0);
    }
}

// ---------------------------------------------------------------------------
// helper
// ---------------------------------------------------------------------------

fn keypair(seed: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
    let (c, r) = libs().pair::<F>("crypto_sign_seed_keypair");
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    let cv = unsafe { c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
    let rv = unsafe { r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()) };
    same_val("crypto_sign_seed_keypair return", cv, rv);
    same("crypto_sign_seed_keypair pk", &cpk, &rpk);
    same("crypto_sign_seed_keypair sk", &csk, &rsk);
    (cpk, csk, rpk, rsk)
}
