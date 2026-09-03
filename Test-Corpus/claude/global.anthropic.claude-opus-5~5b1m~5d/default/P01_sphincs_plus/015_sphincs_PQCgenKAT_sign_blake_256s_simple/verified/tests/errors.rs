//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (runtime rejections 1-7 and boundary rows
//! B1-B18).  Each constructs the exact invalid/boundary condition, calls BOTH
//! the C `.so` and the Rust `.so` through their exported symbols, and asserts
//! the SAME return code / sentinel *and* the same output-buffer side effects.

mod common;

use common::*;
use libloading::os::unix::Symbol;

type Keypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type SignSig = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type CSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type COpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;

/// Identical key pair in both libraries.
fn keypair(p: &Pair, rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let fc: Symbol<Keypair> = p.c.sym("crypto_sign_seed_keypair");
    let fr: Symbol<Keypair> = p.r.sym("crypto_sign_seed_keypair");
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut cpk = vec![0u8; SPX_PK_BYTES];
    let mut csk = vec![0u8; SPX_SK_BYTES];
    let mut rpk = vec![0u8; SPX_PK_BYTES];
    let mut rsk = vec![0u8; SPX_SK_BYTES];
    unsafe { fc(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
    unsafe { fr(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()) };
    eq_bytes("keypair pk", &cpk, &rpk);
    eq_bytes("keypair sk", &csk, &rsk);
    (cpk, csk)
}

/// A valid detached signature over `m`, produced by the C library.
fn sign(p: &Pair, sk: &[u8], m: &[u8], rng: &mut Rng) -> Vec<u8> {
    let f: Symbol<SignSig> = p.c.sym("crypto_sign_signature");
    let mut e = [0u8; 48];
    rng.fill(&mut e);
    seed_drbg(p, &e, None);
    let mut sig = vec![0u8; SPX_BYTES];
    let mut sl: usize = 0;
    unsafe { f(sig.as_mut_ptr(), &mut sl, m.as_ptr(), m.len(), sk.as_ptr()) };
    assert_eq!(sl, SPX_BYTES);
    sig
}

// ===========================================================================
// ERRORS.md row 1 — crypto_sign_verify: siglen != SPX_BYTES
// ===========================================================================

#[test]
fn err01_verify_wrong_siglen() {
    let _g = drbg_guard();
    let p = load();
    let fc: Symbol<Verify> = p.c.sym("crypto_sign_verify");
    let fr: Symbol<Verify> = p.r.sym("crypto_sign_verify");

    let mut rng = Rng::new(101);
    let (pk, sk) = keypair(&p, &mut rng);
    let m = rng.bytes(37);
    let sig = sign(&p, &sk, &m, &mut rng);

    // Sanity: with the right length the signature verifies in both.
    eq(
        "verify(correct siglen)",
        unsafe { fc(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) },
        unsafe { fr(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) },
    );

    for &bad in &[
        0usize,
        1,
        SPX_BYTES - 1,
        SPX_BYTES + 1,
        SPX_BYTES * 2,
        usize::MAX,
    ] {
        let c = unsafe { fc(sig.as_ptr(), bad, m.as_ptr(), m.len(), pk.as_ptr()) };
        let r = unsafe { fr(sig.as_ptr(), bad, m.as_ptr(), m.len(), pk.as_ptr()) };
        eq(&format!("verify(siglen={bad})"), c, r);
        assert_eq!(c, -1, "C must reject siglen={bad} with -1");
    }
}

// ===========================================================================
// ERRORS.md row 2 — crypto_sign_verify: recomputed root != pk root
// ===========================================================================

#[test]
fn err02_verify_root_mismatch() {
    let _g = drbg_guard();
    let p = load();
    let fc: Symbol<Verify> = p.c.sym("crypto_sign_verify");
    let fr: Symbol<Verify> = p.r.sym("crypto_sign_verify");

    let mut rng = Rng::new(102);
    let (pk, sk) = keypair(&p, &mut rng);
    let m = rng.bytes(64);
    let sig = sign(&p, &sk, &m, &mut rng);

    // one probe in every distinct region of the signature
    let mut probes: Vec<usize> = vec![
        0,                              // R
        SPX_N - 1,
        SPX_N,                          // FORS
        SPX_N + SPX_FORS_BYTES / 2,
        SPX_N + SPX_FORS_BYTES - 1,
        SPX_N + SPX_FORS_BYTES,         // first WOTS
        SPX_N + SPX_FORS_BYTES + SPX_WOTS_BYTES - 1,
        SPX_N + SPX_FORS_BYTES + SPX_WOTS_BYTES, // first auth path
        SPX_BYTES - 1,                  // last auth path node
    ];
    for _ in 0..8 {
        probes.push(rng.below(SPX_BYTES as u32) as usize);
    }

    for &pos in &probes {
        let mut bad = sig.clone();
        bad[pos] ^= 1 << (rng.byte() & 7);
        let c = unsafe { fc(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
        let r = unsafe { fr(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
        eq(&format!("verify(corrupt sig byte {pos})"), c, r);
        assert_eq!(c, -1, "C must reject a corrupted signature");
    }

    // Corrupt the message.
    //
    // NOTE: `lib/blake/src/hash_blake.c` passes BYTE counts to
    // `blake256_update`, whose `datalen` parameter is a BIT count
    // (`blake256_update(S, data, datalen)` does `memcpy(..., datalen >> 3)`).
    // Consequently the blake backend only absorbs the first `mlen / 8` bytes of
    // the message into the message digest, and flipping a bit past that point
    // leaves the signature valid.  That is the C's behaviour and the Rust must
    // reproduce it, so only AGREEMENT is asserted for every position; rejection
    // is asserted for byte 0, which every backend does absorb.
    let mut rejected_any_msg = false;
    for pos in 0..m.len() {
        let mut bad = m.clone();
        bad[pos] ^= 0x40;
        let c = unsafe { fc(sig.as_ptr(), SPX_BYTES, bad.as_ptr(), bad.len(), pk.as_ptr()) };
        let r = unsafe { fr(sig.as_ptr(), SPX_BYTES, bad.as_ptr(), bad.len(), pk.as_ptr()) };
        eq(&format!("verify(corrupt msg byte {pos})"), c, r);
        if c == -1 {
            rejected_any_msg = true;
        }
        if pos == 0 {
            assert_eq!(c, -1, "corrupting message byte 0 must be rejected");
        }
    }
    assert!(rejected_any_msg, "no message corruption was ever rejected");

    // corrupt the public key (both the pub_seed half and the root half)
    for pos in [0usize, SPX_N - 1, SPX_N, SPX_PK_BYTES - 1] {
        let mut bad = pk.clone();
        bad[pos] ^= 0x20;
        let c = unsafe { fc(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), bad.as_ptr()) };
        let r = unsafe { fr(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), bad.as_ptr()) };
        eq(&format!("verify(corrupt pk byte {pos})"), c, r);
        assert_eq!(c, -1);
    }

    // an entirely random signature
    for _ in 0..4 {
        let bad = rng.bytes(SPX_BYTES);
        let c = unsafe { fc(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
        let r = unsafe { fr(bad.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()) };
        eq("verify(random signature)", c, r);
        assert_eq!(c, -1);
    }
}

// ===========================================================================
// ERRORS.md row 3 — crypto_sign_open: smlen < SPX_BYTES
// ===========================================================================

#[test]
fn err03_open_smlen_too_short() {
    let _g = drbg_guard();
    let p = load();
    let fc: Symbol<COpen> = p.c.sym("crypto_sign_open");
    let fr: Symbol<COpen> = p.r.sym("crypto_sign_open");

    let mut rng = Rng::new(103);
    let (pk, _sk) = keypair(&p, &mut rng);

    for &smlen in &[0u64, 1, 16, (SPX_BYTES / 2) as u64, (SPX_BYTES - 1) as u64] {
        let sm = rng.bytes(smlen as usize);
        // the C memsets `smlen` bytes of `m`, so `m` must be `smlen` long
        let mut cm = vec![0xAAu8; smlen as usize];
        let mut rm = vec![0xAAu8; smlen as usize];
        let mut cl: u64 = 0xdead;
        let mut rl: u64 = 0xdead;
        let c = unsafe { fc(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, pk.as_ptr()) };
        let r = unsafe { fr(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, pk.as_ptr()) };
        eq(&format!("open(smlen={smlen}) ret"), c, r);
        eq(&format!("open(smlen={smlen}) *mlen"), cl, rl);
        eq_bytes(&format!("open(smlen={smlen}) m"), &cm, &rm);
        assert_eq!(c, -1, "C must reject smlen={smlen}");
        assert_eq!(cl, 0, "C must set *mlen = 0");
        assert!(
            cm.iter().all(|&b| b == 0),
            "C must memset all {smlen} bytes of m"
        );
    }
}

// ===========================================================================
// ERRORS.md row 4 — crypto_sign_open: inner verify fails
// ===========================================================================

#[test]
fn err04_open_verify_fails() {
    let _g = drbg_guard();
    let p = load();
    let fsign: Symbol<CSign> = p.c.sym("crypto_sign");
    let fc: Symbol<COpen> = p.c.sym("crypto_sign_open");
    let fr: Symbol<COpen> = p.r.sym("crypto_sign_open");

    let mut rng = Rng::new(104);
    let (pk, sk) = keypair(&p, &mut rng);

    for &mlen in &[0usize, 1, 64, 500] {
        let m = rng.bytes(mlen);
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        seed_drbg(&p, &e, None);
        let mut sm = vec![0u8; SPX_BYTES + mlen];
        let mut smlen: u64 = 0;
        unsafe { fsign(sm.as_mut_ptr(), &mut smlen, m.as_ptr(), mlen as u64, sk.as_ptr()) };

        // Positions inside the signature always change the outcome; positions
        // inside the appended message do so only if that byte is actually
        // absorbed by `hash_message` (see the note in err02_verify_root_mismatch
        // about the blake backend's bit/byte mix-up).  Rejection is therefore
        // only asserted for the signature region.
        let mut probes: Vec<(usize, bool)> =
            vec![(0, true), (SPX_N, true), (SPX_BYTES - 1, true)];
        if mlen > 0 {
            probes.push((SPX_BYTES, false)); // corrupt the appended message
            probes.push((SPX_BYTES + mlen - 1, false));
        }
        for &(pos, must_reject) in &probes {
            let mut bad = sm.clone();
            bad[pos] ^= 0x01;
            let mut cm = vec![0xAAu8; smlen as usize];
            let mut rm = vec![0xAAu8; smlen as usize];
            let mut cl: u64 = 0xdead;
            let mut rl: u64 = 0xdead;
            let c = unsafe { fc(cm.as_mut_ptr(), &mut cl, bad.as_ptr(), smlen, pk.as_ptr()) };
            let r = unsafe { fr(rm.as_mut_ptr(), &mut rl, bad.as_ptr(), smlen, pk.as_ptr()) };
            eq(&format!("open(corrupt byte {pos}) ret"), c, r);
            eq(&format!("open(corrupt byte {pos}) *mlen"), cl, rl);
            eq_bytes(&format!("open(corrupt byte {pos}) m"), &cm, &rm);
            if must_reject {
                assert_eq!(c, -1, "corrupting signature byte {pos} must be rejected");
            }
            if c == -1 {
                assert_eq!(cl, 0);
                assert!(
                    cm.iter().all(|&b| b == 0),
                    "C must memset all smlen={smlen} bytes of m on the failure path"
                );
            }
        }

        // a wrong public key
        let mut bad_pk = pk.clone();
        bad_pk[SPX_PK_BYTES - 1] ^= 0x80;
        let mut cm = vec![0xAAu8; smlen as usize];
        let mut rm = vec![0xAAu8; smlen as usize];
        let mut cl: u64 = 0xdead;
        let mut rl: u64 = 0xdead;
        let c = unsafe { fc(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, bad_pk.as_ptr()) };
        let r = unsafe { fr(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, bad_pk.as_ptr()) };
        eq("open(wrong pk) ret", c, r);
        eq("open(wrong pk) *mlen", cl, rl);
        eq_bytes("open(wrong pk) m", &cm, &rm);
        assert_eq!(c, -1);
    }
}

// ===========================================================================
// ERRORS.md row 5 / B15 — seedexpander_init maxlen bound
// ===========================================================================

type SeedInit = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, u64) -> i32;
type SeedExp = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, u64) -> i32;

#[test]
fn err05_seedexpander_init_maxlen() {
    let p = load();
    let ic: Symbol<SeedInit> = p.c.sym("seedexpander_init");
    let ir: Symbol<SeedInit> = p.r.sym("seedexpander_init");

    let mut rng = Rng::new(105);
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let mut div = [0u8; 8];
    rng.fill(&mut div);

    for &maxlen in &[
        0x1_0000_0000u64,
        0x1_0000_0001,
        0xFFFF_FFFF_FFFF_FFFF,
        1u64 << 40,
    ] {
        // The C leaves `ctx` untouched on this path, so start from a *marked*
        // context and verify neither implementation writes into it.
        let mut cctx = AesXofStruct::zeroed();
        cctx.buffer = [0x5a; 16];
        cctx.buffer_pos = 0xdeadbeef;
        cctx.length_remaining = 0xcafebabe;
        cctx.key = [0x3c; 32];
        cctx.ctr = [0xc3; 16];
        let mut rctx = cctx;
        let mut s = seed;
        let mut d = div;
        let c = unsafe { ic(&mut cctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        let r = unsafe { ir(&mut rctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ret"), c, r);
        eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ctx"), cctx, rctx);
        assert_eq!(c, -1, "RNG_BAD_MAXLEN");
        let _ = &mut d;
    }
}

#[test]
fn err_b15_seedexpander_init_max_valid() {
    let p = load();
    let ic: Symbol<SeedInit> = p.c.sym("seedexpander_init");
    let ir: Symbol<SeedInit> = p.r.sym("seedexpander_init");

    let mut rng = Rng::new(115);
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let mut div = [0u8; 8];
    rng.fill(&mut div);
    for &maxlen in &[0u64, 1, 0xFFFF_FFFE, 0xFFFF_FFFF] {
        let mut cctx = AesXofStruct::zeroed();
        let mut rctx = AesXofStruct::zeroed();
        let mut s = seed;
        let mut d = div;
        let c = unsafe { ic(&mut cctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        let r = unsafe { ir(&mut rctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ret"), c, r);
        eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ctx"), cctx, rctx);
        assert_eq!(c, 0, "RNG_SUCCESS expected one step below the limit");
        let _ = &mut d;
    }
}

// ===========================================================================
// ERRORS.md row 6 — seedexpander: x == NULL  (checked before the length check)
// ===========================================================================

#[test]
fn err06_seedexpander_null_outbuf() {
    let p = load();
    let ic: Symbol<SeedInit> = p.c.sym("seedexpander_init");
    let ir: Symbol<SeedInit> = p.r.sym("seedexpander_init");
    let sc: Symbol<SeedExp> = p.c.sym("seedexpander");
    let sr: Symbol<SeedExp> = p.r.sym("seedexpander");

    let mut rng = Rng::new(106);
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let mut div = [0u8; 8];
    rng.fill(&mut div);

    for &maxlen in &[0u64, 1, 256, 0xFFFF_FFFF] {
        let mut cctx = AesXofStruct::zeroed();
        let mut rctx = AesXofStruct::zeroed();
        let mut s = seed;
        let mut d = div;
        unsafe { ic(&mut cctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        unsafe { ir(&mut rctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        let before_c = cctx;
        let before_r = rctx;

        // NULL wins over an also-invalid xlen (the null check comes first).
        for &xlen in &[0u64, 1, maxlen, maxlen + 1, u64::MAX] {
            let c = unsafe { sc(&mut cctx, core::ptr::null_mut(), xlen) };
            let r = unsafe { sr(&mut rctx, core::ptr::null_mut(), xlen) };
            eq(&format!("seedexpander(NULL, xlen={xlen}) ret"), c, r);
            assert_eq!(c, -2, "RNG_BAD_OUTBUF");
            eq("seedexpander(NULL) must not touch ctx (C)", before_c, cctx);
            eq("seedexpander(NULL) must not touch ctx (Rust)", before_r, rctx);
            eq("seedexpander(NULL) ctx", cctx, rctx);
        }
        let _ = &mut d;
    }
}

// ===========================================================================
// ERRORS.md row 7 / B14 — seedexpander: xlen >= ctx->length_remaining
// ===========================================================================

#[test]
fn err07_seedexpander_req_len() {
    let p = load();
    let ic: Symbol<SeedInit> = p.c.sym("seedexpander_init");
    let ir: Symbol<SeedInit> = p.r.sym("seedexpander_init");
    let sc: Symbol<SeedExp> = p.c.sym("seedexpander");
    let sr: Symbol<SeedExp> = p.r.sym("seedexpander");

    let mut rng = Rng::new(107);
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let mut div = [0u8; 8];
    rng.fill(&mut div);

    for &maxlen in &[0u64, 1, 16, 17, 256] {
        let mut cctx = AesXofStruct::zeroed();
        let mut rctx = AesXofStruct::zeroed();
        let mut s = seed;
        let mut d = div;
        unsafe { ic(&mut cctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        unsafe { ir(&mut rctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        let before_c = cctx;
        let before_r = rctx;

        // `>=`, so requesting exactly `length_remaining` already errors, and
        // xlen == 0 errors when length_remaining == 0.
        for &xlen in &[maxlen, maxlen + 1, maxlen + 1000, u64::MAX] {
            let mut cb = vec![0xAAu8; 8];
            let mut rb = vec![0xAAu8; 8];
            let c = unsafe { sc(&mut cctx, cb.as_mut_ptr(), xlen) };
            let r = unsafe { sr(&mut rctx, rb.as_mut_ptr(), xlen) };
            eq(&format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ret"), c, r);
            assert_eq!(c, -3, "RNG_BAD_REQ_LEN");
            eq_bytes("seedexpander error path must not write x", &cb, &rb);
            assert!(cb.iter().all(|&b| b == 0xAA), "C must not write x");
            eq("seedexpander error path ctx (C untouched)", before_c, cctx);
            eq("seedexpander error path ctx (Rust untouched)", before_r, rctx);
        }
        let _ = &mut d;
    }
}

#[test]
fn err_b14_seedexpander_max_valid_len() {
    let p = load();
    let ic: Symbol<SeedInit> = p.c.sym("seedexpander_init");
    let ir: Symbol<SeedInit> = p.r.sym("seedexpander_init");
    let sc: Symbol<SeedExp> = p.c.sym("seedexpander");
    let sr: Symbol<SeedExp> = p.r.sym("seedexpander");

    let mut rng = Rng::new(114);
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    let mut div = [0u8; 8];
    rng.fill(&mut div);

    for &maxlen in &[1u64, 2, 16, 17, 33, 256] {
        let mut cctx = AesXofStruct::zeroed();
        let mut rctx = AesXofStruct::zeroed();
        let mut s = seed;
        let mut d = div;
        unsafe { ic(&mut cctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        unsafe { ir(&mut rctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
        let xlen = maxlen - 1; // exactly one step inside the valid range
        let mut cb = vec![0xAAu8; xlen as usize + 8];
        let mut rb = vec![0xAAu8; xlen as usize + 8];
        let c = unsafe { sc(&mut cctx, cb.as_mut_ptr(), xlen) };
        let r = unsafe { sr(&mut rctx, rb.as_mut_ptr(), xlen) };
        eq(&format!("seedexpander(xlen={xlen} of {maxlen}) ret"), c, r);
        assert_eq!(c, 0, "RNG_SUCCESS expected");
        eq_bytes(&format!("seedexpander(xlen={xlen}) out"), &cb, &rb);
        eq(&format!("seedexpander(xlen={xlen}) ctx"), cctx, rctx);
        let _ = &mut d;
    }
}

// ===========================================================================
// ERRORS.md B1/B2/B3/B4 — out-of-range "enum" and setter arguments
// ===========================================================================

#[test]
fn err_b1_set_type_out_of_range() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u32, u32);
    let fc: Symbol<F> = p.c.sym("SPX_set_type");
    let fr: Symbol<F> = p.r.sym("SPX_set_type");

    // 0..=6 are the documented SPX_ADDR_TYPE_* values; everything else is an
    // out-of-range enum value, which C accepts (a C enum is just an int).
    let mut values: Vec<u32> = (0u32..=8).collect();
    values.extend_from_slice(&[
        9, 15, 16, 127, 128, 255, 256, 257, 0x1_00ff, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff,
        (-1i32) as u32,
        (-7i32) as u32,
    ]);
    let mut rng = Rng::new(201);
    for _ in 0..64 {
        values.push(rng.next_u32());
    }

    for &v in &values {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe { fc(ca.as_mut_ptr(), v) };
        unsafe { fr(ra.as_mut_ptr(), v) };
        eq_u32s(&format!("SPX_set_type({v:#x})"), &ca, &ra);
        // and confirm the documented truncation semantics
        let cb = unsafe { *(ca.as_ptr() as *const [u8; 32]) };
        assert_eq!(
            cb[sphincs_core_det::params::SPX_OFFSET_TYPE],
            (v & 0xff) as u8,
            "set_type must truncate to the low byte"
        );
    }
}

#[test]
fn err_b2_single_byte_setters_truncate() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u32, u32);
    let names = [
        "SPX_set_layer_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
    ];
    let mut rng = Rng::new(202);
    let values: Vec<u32> = vec![
        0,
        1,
        (SPX_D as u32) - 1,
        SPX_D as u32,
        SPX_WOTS_W as u32,
        (SPX_WOTS_W as u32) - 1,
        SPX_FULL_HEIGHT as u32,
        255,
        256,
        257,
        0xffff_ffff,
    ];
    for name in names {
        let fc: Symbol<F> = p.c.sym(name);
        let fr: Symbol<F> = p.r.sym(name);
        for &v in &values {
            for _ in 0..4 {
                let base = rng.addr();
                let mut ca = base;
                let mut ra = base;
                unsafe { fc(ca.as_mut_ptr(), v) };
                unsafe { fr(ra.as_mut_ptr(), v) };
                eq_u32s(&format!("{name}({v:#x})"), &ca, &ra);
            }
        }
    }
}

#[test]
fn err_b3_u32_setters_full_range() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u32, u32);
    let mut rng = Rng::new(203);
    for name in ["SPX_set_keypair_addr", "SPX_set_tree_index"] {
        let fc: Symbol<F> = p.c.sym(name);
        let fr: Symbol<F> = p.r.sym(name);
        let mut values: Vec<u32> = vec![0, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];
        for _ in 0..64 {
            values.push(rng.next_u32());
        }
        for &v in &values {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe { fc(ca.as_mut_ptr(), v) };
            unsafe { fr(ra.as_mut_ptr(), v) };
            eq_u32s(&format!("{name}({v:#x})"), &ca, &ra);
        }
    }
}

#[test]
fn err_b4_set_tree_addr_full_range() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u32, u64);
    let fc: Symbol<F> = p.c.sym("SPX_set_tree_addr");
    let fr: Symbol<F> = p.r.sym("SPX_set_tree_addr");
    let mut rng = Rng::new(204);
    let mut values: Vec<u64> = vec![
        0,
        1,
        u64::MAX,
        u64::MAX - 1,
        1u64 << 63,
        // one past the largest legal tree index for this parameter set
        if SPX_TREE_BITS >= 64 {
            u64::MAX
        } else {
            1u64 << SPX_TREE_BITS
        },
    ];
    for _ in 0..64 {
        values.push(rng.next_u64());
    }
    for &v in &values {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe { fc(ca.as_mut_ptr(), v) };
        unsafe { fr(ra.as_mut_ptr(), v) };
        eq_u32s(&format!("SPX_set_tree_addr({v:#x})"), &ca, &ra);
    }
}

// ===========================================================================
// ERRORS.md B5-B8 — ull_to_bytes / bytes_to_ull degenerate lengths
// ===========================================================================

#[test]
fn err_b5_ull_to_bytes_zero_len() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, core::ffi::c_uint, u64);
    let fc: Symbol<F> = p.c.sym("SPX_ull_to_bytes");
    let fr: Symbol<F> = p.r.sym("SPX_ull_to_bytes");
    let mut rng = Rng::new(205);
    for _ in 0..64 {
        let v = rng.next_u64();
        let mut cb = [0xAAu8; 32];
        let mut rb = [0xAAu8; 32];
        unsafe { fc(cb.as_mut_ptr(), 0, v) };
        unsafe { fr(rb.as_mut_ptr(), 0, v) };
        eq_bytes("ull_to_bytes(outlen=0)", &cb, &rb);
        assert!(cb.iter().all(|&b| b == 0xAA), "outlen=0 must write nothing");
    }
}

#[test]
fn err_b6_ull_to_bytes_oversized() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, core::ffi::c_uint, u64);
    let fc: Symbol<F> = p.c.sym("SPX_ull_to_bytes");
    let fr: Symbol<F> = p.r.sym("SPX_ull_to_bytes");
    let mut rng = Rng::new(206);
    for &outlen in &[9u32, 10, 16, 32, 64] {
        for _ in 0..16 {
            let v = rng.next_u64();
            let mut cb = vec![0xAAu8; 128];
            let mut rb = vec![0xAAu8; 128];
            unsafe { fc(cb.as_mut_ptr(), outlen, v) };
            unsafe { fr(rb.as_mut_ptr(), outlen, v) };
            eq_bytes(&format!("ull_to_bytes(outlen={outlen})"), &cb, &rb);
        }
    }
}

#[test]
fn err_b7_bytes_to_ull_zero_len() {
    let p = load();
    type F = unsafe extern "C" fn(*const u8, core::ffi::c_uint) -> u64;
    let fc: Symbol<F> = p.c.sym("SPX_bytes_to_ull");
    let fr: Symbol<F> = p.r.sym("SPX_bytes_to_ull");
    let mut rng = Rng::new(207);
    for _ in 0..16 {
        let inp = rng.bytes(16);
        let c = unsafe { fc(inp.as_ptr(), 0) };
        let r = unsafe { fr(inp.as_ptr(), 0) };
        eq("bytes_to_ull(inlen=0)", c, r);
        assert_eq!(c, 0);
    }
    // inlen = 0 with a NULL pointer: the C loop body never runs, so it never
    // dereferences `in`.
    let c = unsafe { fc(core::ptr::null(), 0) };
    let r = unsafe { fr(core::ptr::null(), 0) };
    eq("bytes_to_ull(NULL, 0)", c, r);
}

#[test]
fn err_b8_bytes_to_ull_oversized() {
    let p = load();
    type F = unsafe extern "C" fn(*const u8, core::ffi::c_uint) -> u64;
    let fc: Symbol<F> = p.c.sym("SPX_bytes_to_ull");
    let fr: Symbol<F> = p.r.sym("SPX_bytes_to_ull");
    let mut rng = Rng::new(208);
    // inlen > 8 makes the C shift count exceed 63 (UB in C); whatever the
    // compiled C does, the Rust must reproduce it bit-for-bit.
    for &inlen in &[9u32, 10, 12, 16] {
        for it in 0..8 {
            let inp = match it {
                0 => vec![0u8; 32],
                1 => vec![0xffu8; 32],
                _ => rng.bytes(32),
            };
            let c = unsafe { fc(inp.as_ptr(), inlen) };
            let r = unsafe { fr(inp.as_ptr(), inlen) };
            eq(&format!("bytes_to_ull(inlen={inlen})"), c, r);
        }
    }
}

// ===========================================================================
// ERRORS.md B9 — thash with inblocks == 0
// ===========================================================================

#[test]
fn err_b9_thash_zero_inblocks() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, core::ffi::c_uint, *const u8, *mut u32);
    let fc: Symbol<F> = p.c.sym("SPX_thash");
    let fr: Symbol<F> = p.r.sym("SPX_thash");
    let mut rng = Rng::new(209);
    for _ in 0..16 {
        let (cc, rc) = init_ctx_pair(&p, &mut rng);
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut co = vec![0xAAu8; SPX_N + 16];
        let mut ro = vec![0xAAu8; SPX_N + 16];
        // `in` is never read for inblocks == 0, so pass a dangling-but-aligned
        // non-null pointer (the C reads 0 bytes from it).
        let dummy = [0u8; 1];
        unsafe { fc(co.as_mut_ptr(), dummy.as_ptr(), 0, cc.as_ptr(), ca.as_mut_ptr()) };
        unsafe { fr(ro.as_mut_ptr(), dummy.as_ptr(), 0, rc.as_ptr(), ra.as_mut_ptr()) };
        eq_bytes("thash(inblocks=0) out", &co, &ro);
        eq_u32s("thash(inblocks=0) addr", &ca, &ra);
    }
}

// ===========================================================================
// ERRORS.md B10/B11 — backend hash and MGF1 degenerate lengths
// ===========================================================================

#[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
#[test]
fn err_b10_backend_hash_empty() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
    for (name, outlen) in [("blake256", 32usize), ("blake512", 64)] {
        let fc: Symbol<F> = p.c.sym(name);
        let fr: Symbol<F> = p.r.sym(name);
        let mut co = vec![0xAAu8; outlen + 16];
        let mut ro = vec![0xAAu8; outlen + 16];
        let dummy = [0u8; 1];
        let c = unsafe { fc(co.as_mut_ptr(), dummy.as_ptr(), 0) };
        let r = unsafe { fr(ro.as_mut_ptr(), dummy.as_ptr(), 0) };
        eq(&format!("{name}(inlen=0) ret"), c, r);
        eq_bytes(&format!("{name}(inlen=0)"), &co, &ro);
    }
}

#[cfg(feature = "sha2")]
#[test]
fn err_b10_backend_hash_empty() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *const u8, usize);
    for (name, outlen) in [("sha256", 32usize), ("sha512", 64)] {
        let fc: Symbol<F> = p.c.sym(name);
        let fr: Symbol<F> = p.r.sym(name);
        let mut co = vec![0xAAu8; outlen + 16];
        let mut ro = vec![0xAAu8; outlen + 16];
        let dummy = [0u8; 1];
        unsafe { fc(co.as_mut_ptr(), dummy.as_ptr(), 0) };
        unsafe { fr(ro.as_mut_ptr(), dummy.as_ptr(), 0) };
        eq_bytes(&format!("{name}(inlen=0)"), &co, &ro);
    }
}

#[cfg(all(feature = "shake", not(feature = "sha2")))]
#[test]
fn err_b10_backend_hash_empty() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    let fc: Symbol<F> = p.c.sym("shake256");
    let fr: Symbol<F> = p.r.sym("shake256");
    let dummy = [0u8; 1];
    for &outlen in &[0usize, 1, 32, 136, 137] {
        let mut co = vec![0xAAu8; outlen + 16];
        let mut ro = vec![0xAAu8; outlen + 16];
        unsafe { fc(co.as_mut_ptr(), outlen, dummy.as_ptr(), 0) };
        unsafe { fr(ro.as_mut_ptr(), outlen, dummy.as_ptr(), 0) };
        eq_bytes(&format!("shake256(inlen=0, outlen={outlen})"), &co, &ro);
    }
}

#[cfg(not(any(feature = "sha2", feature = "shake", feature = "blake")))]
#[test]
fn err_b10_backend_hash_empty() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, u64, *const u8, u64, *const u8);
    let fc: Symbol<F> = p.c.sym("SPX_haraka_S");
    let fr: Symbol<F> = p.r.sym("SPX_haraka_S");
    let mut rng = Rng::new(210);
    let (cc, rc) = init_ctx_pair(&p, &mut rng);
    let dummy = [0u8; 1];
    for &outlen in &[0u64, 1, 32, 33, 64] {
        let mut co = vec![0xAAu8; outlen as usize + 16];
        let mut ro = vec![0xAAu8; outlen as usize + 16];
        unsafe { fc(co.as_mut_ptr(), outlen, dummy.as_ptr(), 0, cc.as_ptr()) };
        unsafe { fr(ro.as_mut_ptr(), outlen, dummy.as_ptr(), 0, rc.as_ptr()) };
        eq_bytes(&format!("haraka_S(inlen=0, outlen={outlen})"), &co, &ro);
    }
}

/// MGF1 with `outlen == 0` and with `outlen` not a multiple of the block size.
/// Only the blake and sha2 backends export an MGF1; shake/haraka use the XOF
/// directly, and those are covered by `err_b10_backend_hash_empty`.
#[test]
fn err_b11_mgf1_boundary_outlen() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, u64, *const u8, u64);

    #[cfg(all(feature = "blake", not(any(feature = "sha2", feature = "shake"))))]
    let names: &[&str] = &["SPX_blake256_mgf1", "SPX_blake512_mgf1"];
    #[cfg(feature = "sha2")]
    let names: &[&str] = &["SPX_mgf1_256", "SPX_mgf1_512"];
    #[cfg(any(
        all(feature = "shake", not(feature = "sha2")),
        not(any(feature = "sha2", feature = "shake", feature = "blake"))
    ))]
    let names: &[&str] = &[];

    let mut rng = Rng::new(211);
    for name in names {
        let fc: Symbol<F> = p.c.sym(name);
        let fr: Symbol<F> = p.r.sym(name);
        for &outlen in &[0u64, 1, 2, 31, 32, 33, 63, 64, 65, 96, 97, 127, 128, 129, 1000] {
            for &inlen in &[0u64, 1, 32, 192, 256, 257, 1000] {
                let input = rng.bytes(inlen as usize);
                let mut cb = vec![0xAAu8; outlen as usize + 32];
                let mut rb = vec![0xAAu8; outlen as usize + 32];
                unsafe { fc(cb.as_mut_ptr(), outlen, input.as_ptr(), inlen) };
                unsafe { fr(rb.as_mut_ptr(), outlen, input.as_ptr(), inlen) };
                eq_bytes(&format!("{name}(outlen={outlen}, inlen={inlen})"), &cb, &rb);
                if outlen == 0 {
                    assert!(cb.iter().all(|&b| b == 0xAA), "outlen=0 must write nothing");
                }
            }
        }
    }
}

// ===========================================================================
// ERRORS.md B12/B13 — empty message and the smlen == SPX_BYTES boundary
// ===========================================================================

#[test]
fn err_b12_empty_message_roundtrip() {
    let _g = drbg_guard();
    let p = load();
    let fsign_c: Symbol<CSign> = p.c.sym("crypto_sign");
    let fsign_r: Symbol<CSign> = p.r.sym("crypto_sign");
    let fopen_c: Symbol<COpen> = p.c.sym("crypto_sign_open");
    let fopen_r: Symbol<COpen> = p.r.sym("crypto_sign_open");

    let mut rng = Rng::new(212);
    let (pk, sk) = keypair(&p, &mut rng);
    let mut e = [0u8; 48];
    rng.fill(&mut e);

    let dummy = [0u8; 1];
    seed_drbg(&p, &e, None);
    let mut c_sm = vec![0xAAu8; SPX_BYTES];
    let mut c_sl: u64 = 0;
    let c1 = unsafe { fsign_c(c_sm.as_mut_ptr(), &mut c_sl, dummy.as_ptr(), 0, sk.as_ptr()) };
    seed_drbg(&p, &e, None);
    let mut r_sm = vec![0xAAu8; SPX_BYTES];
    let mut r_sl: u64 = 0;
    let r1 = unsafe { fsign_r(r_sm.as_mut_ptr(), &mut r_sl, dummy.as_ptr(), 0, sk.as_ptr()) };
    eq("crypto_sign(mlen=0) ret", c1, r1);
    eq("crypto_sign(mlen=0) smlen", c_sl, r_sl);
    eq_bytes("crypto_sign(mlen=0) sm", &c_sm, &r_sm);
    assert_eq!(c_sl as usize, SPX_BYTES);

    let mut cm = vec![0xAAu8; SPX_BYTES];
    let mut rm = vec![0xAAu8; SPX_BYTES];
    let mut cl: u64 = 0xdead;
    let mut rl: u64 = 0xdead;
    let c2 = unsafe { fopen_c(cm.as_mut_ptr(), &mut cl, c_sm.as_ptr(), c_sl, pk.as_ptr()) };
    let r2 = unsafe { fopen_r(rm.as_mut_ptr(), &mut rl, r_sm.as_ptr(), r_sl, pk.as_ptr()) };
    eq("crypto_sign_open(mlen=0) ret", c2, r2);
    eq("crypto_sign_open(mlen=0) *mlen", cl, rl);
    eq_bytes("crypto_sign_open(mlen=0) m", &cm, &rm);
    assert_eq!(c2, 0);
    assert_eq!(cl, 0);
}

#[test]
fn err_b13_open_smlen_exactly_spx_bytes() {
    let _g = drbg_guard();
    let p = load();
    let fc: Symbol<COpen> = p.c.sym("crypto_sign_open");
    let fr: Symbol<COpen> = p.r.sym("crypto_sign_open");

    let mut rng = Rng::new(213);
    let (pk, sk) = keypair(&p, &mut rng);
    let dummy: [u8; 0] = [];
    let sig = sign(&p, &sk, &dummy, &mut rng);

    // smlen == SPX_BYTES is *not* rejected by the `smlen < SPX_BYTES` check.
    let mut cm = vec![0xAAu8; SPX_BYTES];
    let mut rm = vec![0xAAu8; SPX_BYTES];
    let mut cl: u64 = 0xdead;
    let mut rl: u64 = 0xdead;
    let c = unsafe { fc(cm.as_mut_ptr(), &mut cl, sig.as_ptr(), SPX_BYTES as u64, pk.as_ptr()) };
    let r = unsafe { fr(rm.as_mut_ptr(), &mut rl, sig.as_ptr(), SPX_BYTES as u64, pk.as_ptr()) };
    eq("open(smlen == SPX_BYTES) ret", c, r);
    eq("open(smlen == SPX_BYTES) *mlen", cl, rl);
    eq_bytes("open(smlen == SPX_BYTES) m", &cm, &rm);
    assert_eq!(c, 0, "the boundary value must be accepted");
    assert_eq!(cl, 0);

    // and a garbage signature of exactly that length must fail identically
    let bad = rng.bytes(SPX_BYTES);
    let mut cm = vec![0xAAu8; SPX_BYTES];
    let mut rm = vec![0xAAu8; SPX_BYTES];
    let mut cl: u64 = 0xdead;
    let mut rl: u64 = 0xdead;
    let c = unsafe { fc(cm.as_mut_ptr(), &mut cl, bad.as_ptr(), SPX_BYTES as u64, pk.as_ptr()) };
    let r = unsafe { fr(rm.as_mut_ptr(), &mut rl, bad.as_ptr(), SPX_BYTES as u64, pk.as_ptr()) };
    eq("open(garbage, smlen == SPX_BYTES) ret", c, r);
    eq("open(garbage) *mlen", cl, rl);
    eq_bytes("open(garbage) m", &cm, &rm);
    assert_eq!(c, -1);
}

// ===========================================================================
// ERRORS.md B16/B17/B18 — rng.c degenerate / NULL arguments
// ===========================================================================

#[test]
fn err_b16_randombytes_zero_len() {
    let _g = drbg_guard();
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, u64) -> i32;
    let fc: Symbol<F> = p.c.sym("randombytes");
    let fr: Symbol<F> = p.r.sym("randombytes");

    let mut rng = Rng::new(216);
    let mut e = [0u8; 48];
    rng.fill(&mut e);
    seed_drbg(&p, &e, None);
    let before = read_drbg(&p.c);

    let mut cb = [0xAAu8; 16];
    let mut rb = [0xAAu8; 16];
    let c = unsafe { fc(cb.as_mut_ptr(), 0) };
    let r = unsafe { fr(rb.as_mut_ptr(), 0) };
    eq("randombytes(0) ret", c, r);
    assert_eq!(c, 0, "RNG_SUCCESS");
    eq_bytes("randombytes(0) must write nothing", &cb, &rb);
    assert!(cb.iter().all(|&b| b == 0xAA));
    // xlen == 0 still runs AES256_CTR_DRBG_Update and bumps reseed_counter.
    eq_drbg(&p, "DRBG_ctx after randombytes(0)");
    let after = read_drbg(&p.c);
    assert_ne!(before, after, "randombytes(0) must still reseed");
    assert_eq!(after.reseed_counter, before.reseed_counter + 1);

    // NULL output buffer with xlen == 0: the C `while (xlen > 0)` body never
    // runs, so `x` is never dereferenced.
    let c = unsafe { fc(core::ptr::null_mut(), 0) };
    let r = unsafe { fr(core::ptr::null_mut(), 0) };
    eq("randombytes(NULL, 0) ret", c, r);
    eq_drbg(&p, "DRBG_ctx after randombytes(NULL, 0)");
}

#[test]
fn err_b17_randombytes_init_null_pers() {
    let _g = drbg_guard();
    let p = load();
    type FInit = unsafe extern "C" fn(*mut u8, *mut u8);
    type FRb = unsafe extern "C" fn(*mut u8, u64) -> i32;
    let ic: Symbol<FInit> = p.c.sym("randombytes_init");
    let ir: Symbol<FInit> = p.r.sym("randombytes_init");
    let rbc: Symbol<FRb> = p.c.sym("randombytes");
    let rbr: Symbol<FRb> = p.r.sym("randombytes");

    let mut rng = Rng::new(217);
    for pers_all_zero in [false, true] {
        let mut e = [0u8; 48];
        rng.fill(&mut e);
        let mut zero = [0u8; 48];

        // NULL personalization string
        let mut e1 = e;
        unsafe { ic(e1.as_mut_ptr(), core::ptr::null_mut()) };
        let c_null = read_drbg(&p.c);
        let mut e1 = e;
        unsafe { ir(e1.as_mut_ptr(), core::ptr::null_mut()) };
        let r_null = read_drbg(&p.r);
        eq("randombytes_init(NULL pers) DRBG_ctx", c_null, r_null);

        // an all-zero personalization string must be equivalent to NULL
        if pers_all_zero {
            let mut e2 = e;
            unsafe { ic(e2.as_mut_ptr(), zero.as_mut_ptr()) };
            let c_zero = read_drbg(&p.c);
            let mut e2 = e;
            unsafe { ir(e2.as_mut_ptr(), zero.as_mut_ptr()) };
            let r_zero = read_drbg(&p.r);
            eq("randombytes_init(zero pers) DRBG_ctx", c_zero, r_zero);
            eq("zero pers == NULL pers (C)", c_null, c_zero);
        }

        // and the stream that follows must match too
        let mut cb = vec![0xAAu8; 100];
        let mut rb = vec![0xAAu8; 100];
        unsafe { rbc(cb.as_mut_ptr(), 100) };
        unsafe { rbr(rb.as_mut_ptr(), 100) };
        eq_bytes("randombytes after init", &cb, &rb);
        eq_drbg(&p, "DRBG_ctx after randombytes");
    }
}

#[test]
fn err_b18_drbg_update_null_provided_data() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    let fc: Symbol<F> = p.c.sym("AES256_CTR_DRBG_Update");
    let fr: Symbol<F> = p.r.sym("AES256_CTR_DRBG_Update");

    let mut rng = Rng::new(218);
    for _ in 0..32 {
        let mut key = [0u8; 32];
        rng.fill(&mut key);
        let mut v = [0u8; 16];
        rng.fill(&mut v);

        let mut kc = key;
        let mut kr = key;
        let mut vc = v;
        let mut vr = v;
        unsafe { fc(core::ptr::null_mut(), kc.as_mut_ptr(), vc.as_mut_ptr()) };
        unsafe { fr(core::ptr::null_mut(), kr.as_mut_ptr(), vr.as_mut_ptr()) };
        eq_bytes("DRBG_Update(NULL) Key", &kc, &kr);
        eq_bytes("DRBG_Update(NULL) V", &vc, &vr);

        // an all-zero provided_data must be equivalent to NULL (the XOR is a
        // no-op), which pins down that the null check only skips the XOR
        let mut zero = [0u8; 48];
        let mut kz = key;
        let mut vz = v;
        unsafe { fc(zero.as_mut_ptr(), kz.as_mut_ptr(), vz.as_mut_ptr()) };
        eq_bytes("DRBG_Update(zeros) == DRBG_Update(NULL) Key", &kc, &kz);
        eq_bytes("DRBG_Update(zeros) == DRBG_Update(NULL) V", &vc, &vz);
    }
}

// ===========================================================================
// Extra generic boundaries: the V / ctr carry cascades and an all-0xff DRBG
// ===========================================================================

#[test]
fn err_extra_drbg_carry_cascade() {
    let p = load();
    type F = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
    let fc: Symbol<F> = p.c.sym("AES256_CTR_DRBG_Update");
    let fr: Symbol<F> = p.r.sym("AES256_CTR_DRBG_Update");

    let mut rng = Rng::new(219);
    // V values that force every stage of the 16-byte increment carry chain
    let mut vs: Vec<[u8; 16]> = vec![[0xff; 16]];
    for k in 0..16usize {
        let mut v = [0u8; 16];
        for i in (16 - k)..16 {
            v[i] = 0xff;
        }
        vs.push(v);
    }
    for v in &vs {
        let mut key = [0u8; 32];
        rng.fill(&mut key);
        let mut kc = key;
        let mut kr = key;
        let mut vc = *v;
        let mut vr = *v;
        unsafe { fc(core::ptr::null_mut(), kc.as_mut_ptr(), vc.as_mut_ptr()) };
        unsafe { fr(core::ptr::null_mut(), kr.as_mut_ptr(), vr.as_mut_ptr()) };
        eq_bytes("carry: Key", &kc, &kr);
        eq_bytes("carry: V", &vc, &vr);
    }
}

#[test]
fn err_extra_seedexpander_ctr_carry() {
    let p = load();
    let ic: Symbol<SeedInit> = p.c.sym("seedexpander_init");
    let ir: Symbol<SeedInit> = p.r.sym("seedexpander_init");
    let sc: Symbol<SeedExp> = p.c.sym("seedexpander");
    let sr: Symbol<SeedExp> = p.r.sym("seedexpander");

    let mut rng = Rng::new(220);
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);

    // `seedexpander_init` zeroes ctr[12..16], and `seedexpander` increments
    // ctr[15] downwards through ctr[12]; drive enough blocks to exercise it.
    let mut div = [0u8; 8];
    rng.fill(&mut div);
    let maxlen = 0xFFFF_FFFFu64;
    let mut cctx = AesXofStruct::zeroed();
    let mut rctx = AesXofStruct::zeroed();
    let mut s = seed;
    let mut d = div;
    unsafe { ic(&mut cctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };
    unsafe { ir(&mut rctx, s.as_mut_ptr(), d.as_mut_ptr(), maxlen) };

    // Pre-load the counter so the very first increment carries.
    cctx.ctr[12..16].copy_from_slice(&[0x00, 0xff, 0xff, 0xff]);
    rctx.ctr[12..16].copy_from_slice(&[0x00, 0xff, 0xff, 0xff]);

    for &xlen in &[17u64, 33, 49, 1, 16, 100] {
        let mut cb = vec![0xAAu8; xlen as usize + 8];
        let mut rb = vec![0xAAu8; xlen as usize + 8];
        let c = unsafe { sc(&mut cctx, cb.as_mut_ptr(), xlen) };
        let r = unsafe { sr(&mut rctx, rb.as_mut_ptr(), xlen) };
        eq(&format!("seedexpander carry({xlen}) ret"), c, r);
        eq_bytes(&format!("seedexpander carry({xlen}) out"), &cb, &rb);
        eq(&format!("seedexpander carry({xlen}) ctx"), cctx, rctx);
    }
    let _ = &mut d;
}

// ===========================================================================
// Extra: caller-supplied `AES_XOF_struct` state outside the documented range
// ===========================================================================

/// `ctx->buffer_pos` lives in a caller-allocated struct, so it can legitimately
/// arrive at `seedexpander` with any `unsigned long` value.  For `buffer_pos`
/// past the end of the 16-byte `buffer`, `16 - ctx->buffer_pos` wraps, the
/// `xlen <= avail` branch is taken, and the C `memcpy(x, ctx->buffer +
/// buffer_pos, xlen)` reads the FOLLOWING struct fields and returns
/// `RNG_SUCCESS`.  The Rust must return the same code and the same bytes.
#[test]
fn err_extra_seedexpander_buffer_pos_out_of_range() {
    let p = load();
    let sc: Symbol<SeedExp> = p.c.sym("seedexpander");
    let sr: Symbol<SeedExp> = p.r.sym("seedexpander");

    let mut rng = Rng::new(221);
    for &bp in &[16u64, 17, 20, 24, 32, 47, 63] {
        for &xlen in &[1u64, 2, 4, 8] {
            let mut cctx = AesXofStruct::zeroed();
            rng.fill(&mut cctx.buffer);
            rng.fill(&mut cctx.key);
            rng.fill(&mut cctx.ctr);
            cctx.buffer_pos = bp;
            cctx.length_remaining = 100_000;
            let mut rctx = cctx;

            let mut cb = vec![0xAAu8; xlen as usize + 8];
            let mut rb = vec![0xAAu8; xlen as usize + 8];
            let c = unsafe { sc(&mut cctx, cb.as_mut_ptr(), xlen) };
            let r = unsafe { sr(&mut rctx, rb.as_mut_ptr(), xlen) };
            eq(&format!("seedexpander(buffer_pos={bp}, xlen={xlen}) ret"), c, r);
            eq_bytes(&format!("seedexpander(buffer_pos={bp}, xlen={xlen}) out"), &cb, &rb);
            eq(&format!("seedexpander(buffer_pos={bp}, xlen={xlen}) ctx"), cctx, rctx);
        }
    }
}

// ===========================================================================
// Extra: the in-place (overlapping) crypto_sign / crypto_sign_open idiom.
// `sign.c` uses memmove, NOT memcpy, precisely so that `m` may overlap `sm`.
// ===========================================================================

#[test]
fn err_extra_inplace_overlapping_sign_open() {
    let _g = drbg_guard();
    let p = load();
    let cs: Symbol<CSign> = p.c.sym("crypto_sign");
    let rs: Symbol<CSign> = p.r.sym("crypto_sign");
    let co: Symbol<COpen> = p.c.sym("crypto_sign_open");
    let ro: Symbol<COpen> = p.r.sym("crypto_sign_open");

    let mut rng = Rng::new(222);
    let (pk, sk) = keypair(&p, &mut rng);

    for &mlen in &[1usize, 64, 1000] {
        let m = rng.bytes(mlen);
        let mut e = [0u8; 48];
        rng.fill(&mut e);

        // crypto_sign(sm, &len, sm + SPX_BYTES, mlen, sk): the message already
        // sits where it has to end up, so `memmove` is a no-op move onto itself.
        let build = |buf: &mut Vec<u8>| {
            buf.resize(SPX_BYTES + mlen, 0xAA);
            buf[SPX_BYTES..].copy_from_slice(&m);
        };
        let mut c_sm = Vec::new();
        build(&mut c_sm);
        let mut r_sm = Vec::new();
        build(&mut r_sm);

        seed_drbg(&p, &e, None);
        let mut c_l: u64 = 0;
        let c1 = unsafe {
            cs(
                c_sm.as_mut_ptr(),
                &mut c_l,
                c_sm.as_ptr().add(SPX_BYTES),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        seed_drbg(&p, &e, None);
        let mut r_l: u64 = 0;
        let r1 = unsafe {
            rs(
                r_sm.as_mut_ptr(),
                &mut r_l,
                r_sm.as_ptr().add(SPX_BYTES),
                mlen as u64,
                sk.as_ptr(),
            )
        };
        eq(&format!("in-place crypto_sign(mlen={mlen}) ret"), c1, r1);
        eq(&format!("in-place crypto_sign(mlen={mlen}) smlen"), c_l, r_l);
        eq_bytes(&format!("in-place crypto_sign(mlen={mlen}) sm"), &c_sm, &r_sm);

        // crypto_sign_open(sm, &len, sm, smlen, pk): now the destination and the
        // source genuinely overlap with a non-zero offset.
        let mut c_buf = c_sm.clone();
        let mut r_buf = r_sm.clone();
        let mut c_ml: u64 = 0xdead;
        let mut r_ml: u64 = 0xdead;
        let c2 = unsafe {
            co(
                c_buf.as_mut_ptr(),
                &mut c_ml,
                c_buf.as_ptr(),
                c_l,
                pk.as_ptr(),
            )
        };
        let r2 = unsafe {
            ro(
                r_buf.as_mut_ptr(),
                &mut r_ml,
                r_buf.as_ptr(),
                r_l,
                pk.as_ptr(),
            )
        };
        eq(&format!("in-place crypto_sign_open(mlen={mlen}) ret"), c2, r2);
        eq(&format!("in-place crypto_sign_open(mlen={mlen}) mlen"), c_ml, r_ml);
        eq_bytes(&format!("in-place crypto_sign_open(mlen={mlen}) buf"), &c_buf, &r_buf);
        assert_eq!(c2, 0);
        eq_bytes("in-place recovered message", &c_buf[..mlen], &m);
    }
}
