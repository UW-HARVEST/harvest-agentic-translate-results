//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Each constructs the exact invalid input the
//! C rejects, calls BOTH the C `.so` and the Rust `.so`, and asserts they return
//! the SAME error code / sentinel *and* have the same observable side effects on
//! the output buffers — not merely "both failed".

mod common;
use common::*;

/* ------------------------------------------------------------------ */
/* rng.h error constants                                               */
/* ------------------------------------------------------------------ */

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

/* ------------------------------------------------------------------ */
/* Helpers                                                             */
/* ------------------------------------------------------------------ */

fn seed_both_drbgs(entropy: &[u8]) {
    let p = libs();
    let c: FnRandombytesInit = p.c.f("randombytes_init");
    let r: FnRandombytesInit = p.r.f("randombytes_init");
    let mut ec = entropy.to_vec();
    let mut er = entropy.to_vec();
    unsafe {
        c(ec.as_mut_ptr(), core::ptr::null_mut());
        r(er.as_mut_ptr(), core::ptr::null_mut());
    }
}

struct Keys {
    pk_c: Vec<u8>,
    sk_c: Vec<u8>,
    pk_r: Vec<u8>,
    sk_r: Vec<u8>,
}

fn keypair(seed: &[u8]) -> Keys {
    let p = libs();
    let c: FnSeedKeypair = p.c.f("crypto_sign_seed_keypair");
    let r: FnSeedKeypair = p.r.f("crypto_sign_seed_keypair");
    let mut pk_c = vec![0u8; SPX_PK_BYTES];
    let mut sk_c = vec![0u8; SPX_SK_BYTES];
    let mut pk_r = vec![0u8; SPX_PK_BYTES];
    let mut sk_r = vec![0u8; SPX_SK_BYTES];
    unsafe {
        assert_eq!(c(pk_c.as_mut_ptr(), sk_c.as_mut_ptr(), seed.as_ptr()), 0);
        assert_eq!(r(pk_r.as_mut_ptr(), sk_r.as_mut_ptr(), seed.as_ptr()), 0);
    }
    assert_eq!(pk_c, pk_r, "keypair pk diverged (see Phase B)");
    assert_eq!(sk_c, sk_r, "keypair sk diverged (see Phase B)");
    Keys {
        pk_c,
        sk_c,
        pk_r,
        sk_r,
    }
}

/// Produce one valid (sig_c, sig_r) pair for `m`.
fn sign(keys: &Keys, m: &[u8], entropy: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let p = libs();
    let c: FnSignature = p.c.f("crypto_sign_signature");
    let r: FnSignature = p.r.f("crypto_sign_signature");
    seed_both_drbgs(entropy);
    let mut sc = vec![0u8; SPX_BYTES];
    let mut sr = vec![0u8; SPX_BYTES];
    let mut lc = 0usize;
    let mut lr = 0usize;
    unsafe {
        assert_eq!(
            c(sc.as_mut_ptr(), &mut lc, m.as_ptr(), m.len(), keys.sk_c.as_ptr()),
            0
        );
        assert_eq!(
            r(sr.as_mut_ptr(), &mut lr, m.as_ptr(), m.len(), keys.sk_r.as_ptr()),
            0
        );
    }
    assert_eq!(lc, SPX_BYTES);
    assert_eq!(lr, SPX_BYTES);
    (sc, sr)
}

fn verify_pair(keys: &Keys, sig_c: &[u8], sig_r: &[u8], siglen: usize, m: &[u8]) -> (i32, i32) {
    let p = libs();
    let vc: FnVerify = p.c.f("crypto_sign_verify");
    let vr: FnVerify = p.r.f("crypto_sign_verify");
    unsafe {
        (
            vc(sig_c.as_ptr(), siglen, m.as_ptr(), m.len(), keys.pk_c.as_ptr()),
            vr(sig_r.as_ptr(), siglen, m.as_ptr(), m.len(), keys.pk_r.as_ptr()),
        )
    }
}

fn fixture() -> (Keys, Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut rng = Rng::for_row(1000);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let keys = keypair(&seed);
    let m = rng.bytes(64);
    let entropy = rng.bytes(48);
    let (sc, sr) = sign(&keys, &m, &entropy);
    (keys, m, sc, sr)
}

/* ================================================================== */
/* Rows 1-8 — crypto_sign_verify                                      */
/* ================================================================== */

fn verify_bad_siglen(row: u64, siglen: usize, label: &str) {
    let (keys, m, sc, sr) = fixture();
    let (a, b) = verify_pair(&keys, &sc, &sr, siglen, &m);
    eq(&format!("ERRORS row {row}: crypto_sign_verify({label})"), 0, a, b);
    eq(
        &format!("ERRORS row {row}: crypto_sign_verify({label}) == -1"),
        0,
        a,
        -1,
    );
}

#[test]
fn err_01_verify_siglen_zero() {
    verify_bad_siglen(1, 0, "siglen=0");
}

#[test]
fn err_02_verify_siglen_minus_one() {
    verify_bad_siglen(2, SPX_BYTES - 1, "siglen=SPX_BYTES-1");
}

#[test]
fn err_03_verify_siglen_plus_one() {
    verify_bad_siglen(3, SPX_BYTES + 1, "siglen=SPX_BYTES+1");
}

#[test]
fn err_04_verify_siglen_huge() {
    // The `siglen != SPX_BYTES` check runs before any read of `sig`, so an
    // absurd length must be rejected without touching memory.
    verify_bad_siglen(4, usize::MAX, "siglen=SIZE_MAX");
}

#[test]
fn err_05_verify_corrupt_sig() {
    let (keys, m, sc, sr) = fixture();
    let mut rng = Rng::for_row(5);
    // Flip one bit at many different offsets: R, the FORS part, each hypertree
    // layer, and the very last byte.
    let mut offsets: Vec<usize> = vec![
        0,
        SPX_N - 1,
        SPX_N,
        SPX_N + SPX_FORS_BYTES - 1,
        SPX_N + SPX_FORS_BYTES,
        SPX_BYTES - 1,
    ];
    for _ in 0..10 {
        offsets.push((rng.next_u64() % SPX_BYTES as u64) as usize);
    }
    for (i, off) in offsets.into_iter().enumerate() {
        let mut bc = sc.clone();
        let mut br = sr.clone();
        let bit = 1u8 << (rng.next_u32() % 8);
        bc[off] ^= bit;
        br[off] ^= bit;
        let (a, b) = verify_pair(&keys, &bc, &br, SPX_BYTES, &m);
        eq(
            &format!("ERRORS row 5: verify(corrupt sig @{off})"),
            i,
            a,
            b,
        );
        eq(
            &format!("ERRORS row 5: verify(corrupt sig @{off}) == -1"),
            i,
            a,
            -1,
        );
    }
}

#[test]
fn err_06_verify_wrong_message() {
    let (keys, m, sc, sr) = fixture();
    // IMPORTANT C quirk: the BLAKE backend's `hash_message` calls
    // `blakeX_update(&S, m, mlen)`, but `blake256_update`/`blake512_update`
    // take their length in BITS (the one-shot `blake256()` passes `inlen * 8`).
    // So for BLAKE only `mlen / 8` bytes of the message are actually absorbed
    // and mutating a byte past that prefix does NOT change the digest — the C
    // accepts the modified message. That is ground truth and the Rust must
    // behave identically. The requirement asserted here is therefore
    // "C and Rust agree", plus "at least one mutation is rejected" so the test
    // still has teeth.
    let mut rejected = 0usize;
    for (i, off) in [0usize, 1, 2, 7, 31, 63].into_iter().enumerate() {
        let mut m2 = m.clone();
        m2[off] ^= 0x01;
        let (a, b) = verify_pair(&keys, &sc, &sr, SPX_BYTES, &m2);
        eq(&format!("ERRORS row 6: verify(mutated m @{off})"), i, a, b);
        assert!(
            a == 0 || a == -1,
            "row 6: unexpected return value {a} from crypto_sign_verify"
        );
        if a == -1 {
            rejected += 1;
        }
        if off == 0 {
            eq(
                "ERRORS row 6: verify(mutated m @0) == -1 (byte 0 is always absorbed)",
                i,
                a,
                -1,
            );
        }
    }
    assert!(
        rejected > 0,
        "row 6: no message mutation was rejected — the test would be vacuous"
    );
}

#[test]
fn err_07_verify_wrong_pk() {
    let p = libs();
    let vc: FnVerify = p.c.f("crypto_sign_verify");
    let vr: FnVerify = p.r.f("crypto_sign_verify");
    let (_keys, m, sc, sr) = fixture();
    let mut rng = Rng::for_row(7);
    for i in 0..4 {
        let other = keypair(&rng.bytes(CRYPTO_SEEDBYTES));
        let (a, b) = unsafe {
            (
                vc(
                    sc.as_ptr(),
                    SPX_BYTES,
                    m.as_ptr(),
                    m.len(),
                    other.pk_c.as_ptr(),
                ),
                vr(
                    sr.as_ptr(),
                    SPX_BYTES,
                    m.as_ptr(),
                    m.len(),
                    other.pk_r.as_ptr(),
                ),
            )
        };
        eq("ERRORS row 7: verify(wrong pk)", i, a, b);
        eq("ERRORS row 7: verify(wrong pk) == -1", i, a, -1);
    }
}

#[test]
fn err_08_verify_wrong_mlen() {
    let p = libs();
    let vc: FnVerify = p.c.f("crypto_sign_verify");
    let vr: FnVerify = p.r.f("crypto_sign_verify");
    let (keys, m, sc, sr) = fixture();
    for (i, mlen) in [0usize, 1, 32, 63].into_iter().enumerate() {
        let (a, b) = unsafe {
            (
                vc(sc.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, keys.pk_c.as_ptr()),
                vr(sr.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, keys.pk_r.as_ptr()),
            )
        };
        eq(&format!("ERRORS row 8: verify(truncated mlen={mlen})"), i, a, b);
        eq(
            &format!("ERRORS row 8: verify(truncated mlen={mlen}) == -1"),
            i,
            a,
            -1,
        );
    }
}

/* ================================================================== */
/* Rows 9-12 — crypto_sign_open                                       */
/* ================================================================== */

/// Calls `crypto_sign_open` on both sides and compares the return value,
/// `*mlen`, and the *entire* output buffer (the C zeroises `smlen` bytes of `m`
/// on failure, which is a side effect the Rust must reproduce exactly).
fn open_pair(keys: &Keys, sm_c: &[u8], sm_r: &[u8], smlen: u64, mbuf: usize) -> (i32, i32) {
    let p = libs();
    let oc: FnSignOpen = p.c.f("crypto_sign_open");
    let or_: FnSignOpen = p.r.f("crypto_sign_open");
    let mut mc = vec![0xA5u8; mbuf];
    let mut mr = vec![0xA5u8; mbuf];
    let mut nc = 0xDEAD_BEEFu64;
    let mut nr = 0xDEAD_BEEFu64;
    let (a, b) = unsafe {
        (
            oc(mc.as_mut_ptr(), &mut nc, sm_c.as_ptr(), smlen, keys.pk_c.as_ptr()),
            or_(mr.as_mut_ptr(), &mut nr, sm_r.as_ptr(), smlen, keys.pk_r.as_ptr()),
        )
    };
    eq("crypto_sign_open/*mlen", 0, nc, nr);
    eq_bytes("crypto_sign_open/output buffer", 0, &mc, &mr);
    (a, b)
}

#[test]
fn err_09_open_smlen_zero() {
    let (keys, _m, sc, sr) = fixture();
    let (a, b) = open_pair(&keys, &sc, &sr, 0, 64);
    eq("ERRORS row 9: crypto_sign_open(smlen=0)", 0, a, b);
    eq("ERRORS row 9: crypto_sign_open(smlen=0) == -1", 0, a, -1);
}

#[test]
fn err_10_open_smlen_minus_one() {
    let (keys, _m, sc, sr) = fixture();
    // `memset(m, 0, smlen)` writes SPX_BYTES-1 bytes, so the buffer must be
    // that large on both sides.
    let (a, b) = open_pair(&keys, &sc, &sr, (SPX_BYTES - 1) as u64, SPX_BYTES);
    eq("ERRORS row 10: crypto_sign_open(smlen=SPX_BYTES-1)", 0, a, b);
    eq(
        "ERRORS row 10: crypto_sign_open(smlen=SPX_BYTES-1) == -1",
        0,
        a,
        -1,
    );
}

#[test]
fn err_11_open_verify_fail_zeroizes() {
    let p = libs();
    let sc_: FnSign = p.c.f("crypto_sign");
    let sr_: FnSign = p.r.f("crypto_sign");
    let mut rng = Rng::for_row(11);
    let keys = keypair(&rng.bytes(CRYPTO_SEEDBYTES));
    let mlen = 40usize;
    let m = rng.bytes(mlen);
    let entropy = rng.bytes(48);
    seed_both_drbgs(&entropy);
    let mut smc = vec![0u8; SPX_BYTES + mlen];
    let mut smr = vec![0u8; SPX_BYTES + mlen];
    let mut lc = 0u64;
    let mut lr = 0u64;
    unsafe {
        sc_(
            smc.as_mut_ptr(),
            &mut lc,
            m.as_ptr(),
            mlen as u64,
            keys.sk_c.as_ptr(),
        );
        sr_(
            smr.as_mut_ptr(),
            &mut lr,
            m.as_ptr(),
            mlen as u64,
            keys.sk_r.as_ptr(),
        );
    }
    assert_eq!(smc, smr, "crypto_sign diverged (see Phase B)");
    // Corrupt the signature so the inner verify fails, then check the
    // "memset(m, 0, smlen)" side effect (note: smlen, not smlen - SPX_BYTES).
    smc[10] ^= 0xFF;
    smr[10] ^= 0xFF;
    let (a, b) = open_pair(&keys, &smc, &smr, lc, SPX_BYTES + mlen);
    eq("ERRORS row 11: crypto_sign_open(bad sig)", 0, a, b);
    eq("ERRORS row 11: crypto_sign_open(bad sig) == -1", 0, a, -1);
}

#[test]
fn err_12_open_exact_len_invalid() {
    let mut rng = Rng::for_row(12);
    let keys = keypair(&rng.bytes(CRYPTO_SEEDBYTES));
    // smlen == SPX_BYTES exactly: passes the length check, empty tail message,
    // random (invalid) signature body.
    let sm = rng.bytes(SPX_BYTES);
    let (a, b) = open_pair(&keys, &sm, &sm, SPX_BYTES as u64, SPX_BYTES);
    eq("ERRORS row 12: crypto_sign_open(smlen==SPX_BYTES, invalid)", 0, a, b);
    eq(
        "ERRORS row 12: crypto_sign_open(smlen==SPX_BYTES, invalid) == -1",
        0,
        a,
        -1,
    );
}

/* ================================================================== */
/* Rows 13-20 — seedexpander_init / seedexpander                      */
/* ================================================================== */

fn seedexpander_init_pair(
    seed: &[u8],
    div: &[u8],
    maxlen: u64,
) -> (i32, i32, AesXofStruct, AesXofStruct) {
    let p = libs();
    let ic: FnSeedexpanderInit = p.c.f("seedexpander_init");
    let ir: FnSeedexpanderInit = p.r.f("seedexpander_init");
    let mut sc = AesXofStruct::default();
    let mut sr = AesXofStruct::default();
    // Pre-fill with a recognisable pattern so "did not touch ctx" is testable.
    sc.buffer = [0x5Au8; 16];
    sc.buffer_pos = 0x1234;
    sc.length_remaining = 0x5678;
    sc.key = [0x5Au8; 32];
    sc.ctr = [0x5Au8; 16];
    sr = AesXofStruct {
        buffer: sc.buffer,
        buffer_pos: sc.buffer_pos,
        length_remaining: sc.length_remaining,
        key: sc.key,
        ctr: sc.ctr,
    };
    let mut s = seed.to_vec();
    let mut d = div.to_vec();
    let (a, b) = unsafe {
        (
            ic(
                &mut sc,
                s.as_mut_ptr(),
                d.as_mut_ptr(),
                maxlen as core::ffi::c_ulong,
            ),
            ir(
                &mut sr,
                s.as_mut_ptr(),
                d.as_mut_ptr(),
                maxlen as core::ffi::c_ulong,
            ),
        )
    };
    (a, b, sc, sr)
}

fn seedexpander_init_bad(row: u64, maxlen: u64, label: &str) {
    let mut rng = Rng::for_row(row);
    let seed = rng.bytes(32);
    let div = rng.bytes(8);
    let (a, b, sc, sr) = seedexpander_init_pair(&seed, &div, maxlen);
    eq(&format!("ERRORS row {row}: seedexpander_init({label})"), 0, a, b);
    eq(
        &format!("ERRORS row {row}: seedexpander_init({label}) == RNG_BAD_MAXLEN"),
        0,
        a,
        RNG_BAD_MAXLEN,
    );
    eq_bytes(
        &format!("ERRORS row {row}: seedexpander_init({label}) leaves ctx untouched"),
        0,
        &sc.bytes(),
        &sr.bytes(),
    );
    // The C returns before writing anything: the pattern must survive.
    assert_eq!(sc.key, [0x5Au8; 32], "row {row}: C wrote to ctx->key");
    assert_eq!(sr.key, [0x5Au8; 32], "row {row}: Rust wrote to ctx->key");
}

#[test]
fn err_13_seedexpander_init_maxlen_boundary() {
    seedexpander_init_bad(13, 0x1_0000_0000, "maxlen=2^32");
}

#[test]
fn err_14_seedexpander_init_maxlen_max() {
    seedexpander_init_bad(14, u64::MAX, "maxlen=ULONG_MAX");
}

#[test]
fn err_15_seedexpander_init_maxlen_ok() {
    let mut rng = Rng::for_row(15);
    let seed = rng.bytes(32);
    let div = rng.bytes(8);
    let (a, b, sc, sr) = seedexpander_init_pair(&seed, &div, 0xFFFF_FFFF);
    eq("ERRORS row 15: seedexpander_init(maxlen=2^32-1)", 0, a, b);
    eq(
        "ERRORS row 15: seedexpander_init(maxlen=2^32-1) == RNG_SUCCESS",
        0,
        a,
        RNG_SUCCESS,
    );
    eq_bytes(
        "ERRORS row 15: seedexpander_init ctx",
        0,
        &sc.bytes(),
        &sr.bytes(),
    );
}

/// Build a valid, initialised expander state on both sides.
fn seedexpander_ready(row: u64, maxlen: u64) -> (AesXofStruct, AesXofStruct) {
    let mut rng = Rng::for_row(row);
    let seed = rng.bytes(32);
    let div = rng.bytes(8);
    let p = libs();
    let ic: FnSeedexpanderInit = p.c.f("seedexpander_init");
    let ir: FnSeedexpanderInit = p.r.f("seedexpander_init");
    let mut sc = AesXofStruct::default();
    let mut sr = AesXofStruct::default();
    let mut s = seed.clone();
    let mut d = div.clone();
    unsafe {
        assert_eq!(
            ic(
                &mut sc,
                s.as_mut_ptr(),
                d.as_mut_ptr(),
                maxlen as core::ffi::c_ulong
            ),
            0
        );
        assert_eq!(
            ir(
                &mut sr,
                s.as_mut_ptr(),
                d.as_mut_ptr(),
                maxlen as core::ffi::c_ulong
            ),
            0
        );
    }
    assert_eq!(sc.bytes(), sr.bytes(), "seedexpander_init diverged");
    (sc, sr)
}

#[test]
fn err_16_seedexpander_null_out() {
    let p = libs();
    let ec: FnSeedexpander = p.c.f("seedexpander");
    let er: FnSeedexpander = p.r.f("seedexpander");
    let (mut sc, mut sr) = seedexpander_ready(16, 1024);
    let before = sc.bytes();
    let (a, b) = unsafe {
        (
            ec(&mut sc, core::ptr::null_mut(), 8),
            er(&mut sr, core::ptr::null_mut(), 8),
        )
    };
    eq("ERRORS row 16: seedexpander(x=NULL)", 0, a, b);
    eq(
        "ERRORS row 16: seedexpander(x=NULL) == RNG_BAD_OUTBUF",
        0,
        a,
        RNG_BAD_OUTBUF,
    );
    eq_bytes("ERRORS row 16: ctx", 0, &sc.bytes(), &sr.bytes());
    eq_bytes("ERRORS row 16: ctx untouched", 0, &sc.bytes(), &before);
}

fn seedexpander_bad_len(row: u64, delta: i64, expected: i32, label: &str) {
    let p = libs();
    let ec: FnSeedexpander = p.c.f("seedexpander");
    let er: FnSeedexpander = p.r.f("seedexpander");
    let (mut sc, mut sr) = seedexpander_ready(row, 1024);
    let xlen = (sc.length_remaining as i64 + delta) as u64;
    let before = sc.bytes();
    let mut oc = vec![0xA5u8; 2048];
    let mut or = vec![0xA5u8; 2048];
    let (a, b) = unsafe {
        (
            ec(&mut sc, oc.as_mut_ptr(), xlen as core::ffi::c_ulong),
            er(&mut sr, or.as_mut_ptr(), xlen as core::ffi::c_ulong),
        )
    };
    eq(&format!("ERRORS row {row}: seedexpander({label})"), 0, a, b);
    eq(
        &format!("ERRORS row {row}: seedexpander({label}) == {expected}"),
        0,
        a,
        expected,
    );
    eq_bytes(&format!("ERRORS row {row}: ctx"), 0, &sc.bytes(), &sr.bytes());
    eq_bytes(
        &format!("ERRORS row {row}: ctx untouched"),
        0,
        &sc.bytes(),
        &before,
    );
    eq_bytes(&format!("ERRORS row {row}: out untouched"), 0, &oc, &or);
    assert!(
        oc.iter().all(|&x| x == 0xA5),
        "row {row}: C wrote to the output buffer on rejection"
    );
}

#[test]
fn err_17_seedexpander_xlen_eq_remaining() {
    // The check is `xlen >= ctx->length_remaining`, so asking for exactly the
    // remaining amount is REJECTED.
    seedexpander_bad_len(17, 0, RNG_BAD_REQ_LEN, "xlen == length_remaining");
}

#[test]
fn err_18_seedexpander_xlen_gt_remaining() {
    seedexpander_bad_len(18, 1, RNG_BAD_REQ_LEN, "xlen > length_remaining");
}

#[test]
fn err_19_seedexpander_null_beats_len() {
    let p = libs();
    let ec: FnSeedexpander = p.c.f("seedexpander");
    let er: FnSeedexpander = p.r.f("seedexpander");
    let (mut sc, mut sr) = seedexpander_ready(19, 1024);
    let xlen = sc.length_remaining + 100;
    let (a, b) = unsafe {
        (
            ec(&mut sc, core::ptr::null_mut(), xlen),
            er(&mut sr, core::ptr::null_mut(), xlen),
        )
    };
    eq("ERRORS row 19: seedexpander(NULL, oversized)", 0, a, b);
    eq(
        "ERRORS row 19: NULL check wins over the length check",
        0,
        a,
        RNG_BAD_OUTBUF,
    );
}

#[test]
fn err_20_seedexpander_zero_len() {
    let p = libs();
    let ec: FnSeedexpander = p.c.f("seedexpander");
    let er: FnSeedexpander = p.r.f("seedexpander");
    let (mut sc, mut sr) = seedexpander_ready(20, 1024);
    let before = sc.bytes();
    let mut oc = vec![0xA5u8; 16];
    let mut or = vec![0xA5u8; 16];
    let (a, b) = unsafe {
        (
            ec(&mut sc, oc.as_mut_ptr(), 0),
            er(&mut sr, or.as_mut_ptr(), 0),
        )
    };
    eq("ERRORS row 20: seedexpander(xlen=0)", 0, a, b);
    eq(
        "ERRORS row 20: seedexpander(xlen=0) == RNG_SUCCESS",
        0,
        a,
        RNG_SUCCESS,
    );
    eq_bytes("ERRORS row 20: out", 0, &oc, &or);
    eq_bytes("ERRORS row 20: ctx", 0, &sc.bytes(), &sr.bytes());
    // `while (xlen > 0)` is never entered, and length_remaining -= 0.
    eq_bytes("ERRORS row 20: ctx unchanged", 0, &sc.bytes(), &before);
}

/* ================================================================== */
/* Rows 21-26 — degenerate-but-accepted inputs                        */
/* ================================================================== */

#[test]
fn err_21_randombytes_zero_len() {
    let p = libs();
    let c: FnRandombytes = p.c.f("randombytes");
    let r: FnRandombytes = p.r.f("randombytes");
    let mut rng = Rng::for_row(21);
    let entropy = rng.bytes(48);
    seed_both_drbgs(&entropy);
    let before_c = unsafe { p.c.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec() };
    let mut oc = [0xA5u8; 8];
    let mut or = [0xA5u8; 8];
    let (a, b) = unsafe { (c(oc.as_mut_ptr(), 0), r(or.as_mut_ptr(), 0)) };
    eq("ERRORS row 21: randombytes(xlen=0)", 0, a, b);
    eq("ERRORS row 21: randombytes(xlen=0) == RNG_SUCCESS", 0, a, 0);
    eq_bytes("ERRORS row 21: out untouched", 0, &oc, &or);
    assert_eq!(oc, [0xA5u8; 8], "C wrote output for xlen=0");
    let after_c = unsafe { p.c.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec() };
    let after_r = unsafe { p.r.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec() };
    eq_bytes("ERRORS row 21: DRBG_ctx after", 0, &after_c, &after_r);
    // The C still runs the trailing DRBG update, so the state MUST have moved.
    assert_ne!(
        before_c, after_c,
        "row 21: the C DRBG state should advance even for xlen=0"
    );
}

#[test]
fn err_22_randombytes_init_null_pers() {
    let p = libs();
    let c: FnRandombytesInit = p.c.f("randombytes_init");
    let r: FnRandombytesInit = p.r.f("randombytes_init");
    let mut rng = Rng::for_row(22);
    for i in 0..4 {
        let entropy = rng.bytes(48);
        let mut ec = entropy.clone();
        let mut er = entropy.clone();
        unsafe {
            c(ec.as_mut_ptr(), core::ptr::null_mut());
            r(er.as_mut_ptr(), core::ptr::null_mut());
        }
        let sc = unsafe { p.c.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec() };
        let sr = unsafe { p.r.data("DRBG_ctx", DRBG_CTX_BYTES).to_vec() };
        eq_bytes("ERRORS row 22: DRBG_ctx", i, &sc, &sr);
        eq_bytes("ERRORS row 22: entropy untouched", i, &ec, &entropy);
        eq_bytes("ERRORS row 22: entropy untouched (Rust)", i, &er, &entropy);
    }
}

#[test]
fn err_23_seed_keypair_always_zero() {
    let p = libs();
    let c: FnSeedKeypair = p.c.f("crypto_sign_seed_keypair");
    let r: FnSeedKeypair = p.r.f("crypto_sign_seed_keypair");
    let mut rng = Rng::for_row(23);
    for (i, seed) in [
        vec![0u8; CRYPTO_SEEDBYTES],
        vec![0xFFu8; CRYPTO_SEEDBYTES],
        rng.bytes(CRYPTO_SEEDBYTES),
    ]
    .into_iter()
    .enumerate()
    {
        let mut pc = vec![0u8; SPX_PK_BYTES];
        let mut sk = vec![0u8; SPX_SK_BYTES];
        let mut pr = vec![0u8; SPX_PK_BYTES];
        let mut sr = vec![0u8; SPX_SK_BYTES];
        let (a, b) = unsafe {
            (
                c(pc.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()),
                r(pr.as_mut_ptr(), sr.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq("ERRORS row 23: crypto_sign_seed_keypair retval", i, a, b);
        eq("ERRORS row 23: crypto_sign_seed_keypair == 0", i, a, 0);
    }
}

#[test]
fn err_24_signature_always_zero() {
    let p = libs();
    let c: FnSignature = p.c.f("crypto_sign_signature");
    let r: FnSignature = p.r.f("crypto_sign_signature");
    let mut rng = Rng::for_row(24);
    let keys = keypair(&rng.bytes(CRYPTO_SEEDBYTES));
    for (i, mlen) in [0usize, 1].into_iter().enumerate() {
        let m = rng.bytes(mlen.max(1));
        let entropy = rng.bytes(48);
        seed_both_drbgs(&entropy);
        let mut sc = vec![0u8; SPX_BYTES];
        let mut sr = vec![0u8; SPX_BYTES];
        let mut lc = 0usize;
        let mut lr = 0usize;
        let (a, b) = unsafe {
            (
                c(
                    sc.as_mut_ptr(),
                    &mut lc,
                    m.as_ptr(),
                    mlen,
                    keys.sk_c.as_ptr(),
                ),
                r(
                    sr.as_mut_ptr(),
                    &mut lr,
                    m.as_ptr(),
                    mlen,
                    keys.sk_r.as_ptr(),
                ),
            )
        };
        eq("ERRORS row 24: crypto_sign_signature retval", i, a, b);
        eq("ERRORS row 24: crypto_sign_signature == 0", i, a, 0);
        eq("ERRORS row 24: *siglen", i, lc, lr);
        eq("ERRORS row 24: *siglen == SPX_BYTES", i, lc, SPX_BYTES);
    }
}

#[test]
fn err_25_sign_always_zero() {
    let p = libs();
    let c: FnSign = p.c.f("crypto_sign");
    let r: FnSign = p.r.f("crypto_sign");
    let mut rng = Rng::for_row(25);
    let keys = keypair(&rng.bytes(CRYPTO_SEEDBYTES));
    for (i, mlen) in [0usize, 7].into_iter().enumerate() {
        let m = rng.bytes(mlen.max(1));
        let entropy = rng.bytes(48);
        seed_both_drbgs(&entropy);
        let mut sc = vec![0u8; SPX_BYTES + mlen];
        let mut sr = vec![0u8; SPX_BYTES + mlen];
        let mut lc = 0u64;
        let mut lr = 0u64;
        let (a, b) = unsafe {
            (
                c(
                    sc.as_mut_ptr(),
                    &mut lc,
                    m.as_ptr(),
                    mlen as u64,
                    keys.sk_c.as_ptr(),
                ),
                r(
                    sr.as_mut_ptr(),
                    &mut lr,
                    m.as_ptr(),
                    mlen as u64,
                    keys.sk_r.as_ptr(),
                ),
            )
        };
        eq("ERRORS row 25: crypto_sign retval", i, a, b);
        eq("ERRORS row 25: crypto_sign == 0", i, a, 0);
        eq("ERRORS row 25: *smlen", i, lc, lr);
        eq(
            "ERRORS row 25: *smlen == SPX_BYTES + mlen",
            i,
            lc as usize,
            SPX_BYTES + mlen,
        );
    }
}

#[test]
fn err_26_keypair_always_zero() {
    let p = libs();
    let c: FnKeypair = p.c.f("crypto_sign_keypair");
    let r: FnKeypair = p.r.f("crypto_sign_keypair");
    let mut rng = Rng::for_row(26);
    let entropy = rng.bytes(48);
    seed_both_drbgs(&entropy);
    let mut pc = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    let mut pr = vec![0u8; SPX_PK_BYTES];
    let mut sr = vec![0u8; SPX_SK_BYTES];
    let (a, b) = unsafe {
        (
            c(pc.as_mut_ptr(), sk.as_mut_ptr()),
            r(pr.as_mut_ptr(), sr.as_mut_ptr()),
        )
    };
    eq("ERRORS row 26: crypto_sign_keypair retval", 0, a, b);
    eq("ERRORS row 26: crypto_sign_keypair == 0", 0, a, 0);
}

/* ================================================================== */
/* Rows 27-33 — out-of-range values across the FFI boundary           */
/* ================================================================== */

const SLOTS: usize = 16;

fn addrs(rng: &mut Rng) -> [u32; SLOTS] {
    let mut a = [0u32; SLOTS];
    for x in a.iter_mut() {
        *x = rng.next_u32();
    }
    a
}

/// `set_type` with values that have no corresponding `SPX_ADDR_TYPE_*` variant.
/// C enums / `uint32_t` parameters accept any int, and the C silently truncates
/// to `unsigned char`.
#[test]
fn err_27_set_type_out_of_range() {
    let p = libs();
    let c: FnSetU32 = p.c.f("SPX_set_type");
    let r: FnSetU32 = p.r.f("SPX_set_type");
    let mut rng = Rng::for_row(27);
    let bad = [
        7u32,
        8,
        100,
        255,
        256,
        257,
        0x0000_FF00,
        0x1234_5678,
        u32::MAX,
        u32::MAX - 1,
    ];
    for (i, &t) in bad.iter().enumerate() {
        let base = addrs(&mut rng);
        let mut ac = base;
        let mut ar = base;
        unsafe {
            c(ac.as_mut_ptr(), t);
            r(ar.as_mut_ptr(), t);
        }
        eq_u32s(&format!("ERRORS row 27: SPX_set_type({t})"), i, &ac, &ar);
        // Document the actual C behaviour: byte-level truncation, no rejection.
        let ab: &[u8] = unsafe {
            core::slice::from_raw_parts(ac.as_ptr() as *const u8, SLOTS * 4)
        };
        eq(
            &format!("ERRORS row 27: SPX_set_type({t}) truncates"),
            i,
            ab[SPX_OFFSET_TYPE],
            (t & 0xFF) as u8,
        );
    }
}

fn set_byte_field_row(row: u64, sym: &str, offset: usize) {
    let p = libs();
    let c: FnSetU32 = p.c.f(sym);
    let r: FnSetU32 = p.r.f(sym);
    let mut rng = Rng::for_row(row);
    let bad = [
        256u32,
        257,
        0x0000_FF00,
        0x0000_FFFF,
        0x1234_5678,
        u32::MAX,
    ];
    for (i, &v) in bad.iter().enumerate() {
        let base = addrs(&mut rng);
        let mut ac = base;
        let mut ar = base;
        unsafe {
            c(ac.as_mut_ptr(), v);
            r(ar.as_mut_ptr(), v);
        }
        eq_u32s(&format!("ERRORS row {row}: {sym}({v})"), i, &ac, &ar);
        let ab: &[u8] =
            unsafe { core::slice::from_raw_parts(ac.as_ptr() as *const u8, SLOTS * 4) };
        eq(
            &format!("ERRORS row {row}: {sym}({v}) truncates to a byte"),
            i,
            ab[offset],
            (v & 0xFF) as u8,
        );
    }
}

#[test]
fn err_28_set_layer_truncates() {
    set_byte_field_row(28, "SPX_set_layer_addr", SPX_OFFSET_LAYER);
}

#[test]
fn err_29_set_chain_truncates() {
    set_byte_field_row(29, "SPX_set_chain_addr", SPX_OFFSET_CHAIN_ADDR);
}

#[test]
fn err_30_set_hash_truncates() {
    set_byte_field_row(30, "SPX_set_hash_addr", SPX_OFFSET_HASH_ADDR);
}

#[test]
fn err_31_set_tree_height_truncates() {
    set_byte_field_row(31, "SPX_set_tree_height", SPX_OFFSET_TREE_HGT);
}

#[test]
fn err_32_set_tree_addr_full_range() {
    let p = libs();
    let c: FnSetU64 = p.c.f("SPX_set_tree_addr");
    let r: FnSetU64 = p.r.f("SPX_set_tree_addr");
    let mut rng = Rng::for_row(32);
    let bad = [
        u64::MAX,
        u64::MAX - 1,
        1u64 << 63,
        0xDEAD_BEEF_CAFE_BABE,
        // Well past 2^SPX_TREE_BITS, which the C does NOT mask.
        (1u64 << SPX_TREE_BITS.min(62)) << 1,
    ];
    for (i, &v) in bad.iter().enumerate() {
        let base = addrs(&mut rng);
        let mut ac = base;
        let mut ar = base;
        unsafe {
            c(ac.as_mut_ptr(), v);
            r(ar.as_mut_ptr(), v);
        }
        eq_u32s(&format!("ERRORS row 32: SPX_set_tree_addr({v:#x})"), i, &ac, &ar);
        let ab: &[u8] =
            unsafe { core::slice::from_raw_parts(ac.as_ptr() as *const u8, SLOTS * 4) };
        eq_bytes(
            &format!("ERRORS row 32: SPX_set_tree_addr({v:#x}) is 8-byte big-endian"),
            i,
            &ab[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8],
            &v.to_be_bytes(),
        );
    }
}

#[test]
fn err_33_set_u32_fields_full_range() {
    let p = libs();
    let mut rng = Rng::for_row(33);
    for (sym, offset) in [
        ("SPX_set_keypair_addr", SPX_OFFSET_KP_ADDR),
        ("SPX_set_tree_index", SPX_OFFSET_TREE_INDEX),
    ] {
        let c: FnSetU32 = p.c.f(sym);
        let r: FnSetU32 = p.r.f(sym);
        for (i, &v) in [u32::MAX, u32::MAX - 1, 1u32 << 31, 0xDEAD_BEEF]
            .iter()
            .enumerate()
        {
            let base = addrs(&mut rng);
            let mut ac = base;
            let mut ar = base;
            unsafe {
                c(ac.as_mut_ptr(), v);
                r(ar.as_mut_ptr(), v);
            }
            eq_u32s(&format!("ERRORS row 33: {sym}({v:#x})"), i, &ac, &ar);
            let ab: &[u8] =
                unsafe { core::slice::from_raw_parts(ac.as_ptr() as *const u8, SLOTS * 4) };
            eq_bytes(
                &format!("ERRORS row 33: {sym}({v:#x}) is 4-byte big-endian"),
                i,
                &ab[offset..offset + 4],
                &v.to_be_bytes(),
            );
        }
    }
}

/* ================================================================== */
/* Rows 34-37 — degenerate lengths in the byte utilities              */
/* ================================================================== */

#[test]
fn err_34_ull_to_bytes_outlen_zero() {
    let p = libs();
    let c: FnUllToBytes = p.c.f("SPX_ull_to_bytes");
    let r: FnUllToBytes = p.r.f("SPX_ull_to_bytes");
    let mut rng = Rng::for_row(34);
    for i in 0..8 {
        let v = rng.next_u64();
        let mut bc = [0xA5u8; 16];
        let mut br = [0xA5u8; 16];
        unsafe {
            c(bc.as_mut_ptr(), 0, v);
            r(br.as_mut_ptr(), 0, v);
        }
        eq_bytes("ERRORS row 34: SPX_ull_to_bytes(outlen=0)", i, &bc, &br);
        assert_eq!(bc, [0xA5u8; 16], "row 34: the C wrote with outlen=0");
    }
}

#[test]
fn err_35_ull_to_bytes_outlen_gt_8() {
    let p = libs();
    let c: FnUllToBytes = p.c.f("SPX_ull_to_bytes");
    let r: FnUllToBytes = p.r.f("SPX_ull_to_bytes");
    let mut rng = Rng::for_row(35);
    for (i, &outlen) in [9u32, 12, 16, 24].iter().enumerate() {
        let v = if i % 2 == 0 { u64::MAX } else { rng.next_u64() };
        let mut bc = vec![0xA5u8; outlen as usize + 8];
        let mut br = bc.clone();
        unsafe {
            c(bc.as_mut_ptr(), outlen, v);
            r(br.as_mut_ptr(), outlen, v);
        }
        eq_bytes(
            &format!("ERRORS row 35: SPX_ull_to_bytes(outlen={outlen})"),
            i,
            &bc,
            &br,
        );
        // Documented C behaviour: the high outlen-8 bytes become 0.
        assert!(
            bc[..outlen as usize - 8].iter().all(|&x| x == 0),
            "row 35: expected the C to zero-extend the high bytes"
        );
    }
}

#[test]
fn err_36_bytes_to_ull_inlen_zero() {
    let p = libs();
    let c: FnBytesToUll = p.c.f("SPX_bytes_to_ull");
    let r: FnBytesToUll = p.r.f("SPX_bytes_to_ull");
    let mut rng = Rng::for_row(36);
    for i in 0..8 {
        let buf = rng.bytes(16);
        let (a, b) = unsafe { (c(buf.as_ptr(), 0), r(buf.as_ptr(), 0)) };
        eq("ERRORS row 36: SPX_bytes_to_ull(inlen=0)", i, a, b);
        eq("ERRORS row 36: SPX_bytes_to_ull(inlen=0) == 0", i, a, 0);
        // Also with a NULL pointer: inlen=0 means the pointer is never read.
        let (a, b) = unsafe { (c(core::ptr::null(), 0), r(core::ptr::null(), 0)) };
        eq("ERRORS row 36: SPX_bytes_to_ull(NULL, 0)", i, a, b);
        eq("ERRORS row 36: SPX_bytes_to_ull(NULL, 0) == 0", i, a, 0);
    }
}

#[test]
fn err_37_bytes_to_ull_inlen_8() {
    let p = libs();
    let c: FnBytesToUll = p.c.f("SPX_bytes_to_ull");
    let r: FnBytesToUll = p.r.f("SPX_bytes_to_ull");
    let mut rng = Rng::for_row(37);
    let cases: Vec<Vec<u8>> = vec![
        vec![0u8; 8],
        vec![0xFFu8; 8],
        vec![0x80, 0, 0, 0, 0, 0, 0, 0],
        rng.bytes(8),
        rng.bytes(8),
    ];
    for (i, buf) in cases.into_iter().enumerate() {
        let (a, b) = unsafe { (c(buf.as_ptr(), 8), r(buf.as_ptr(), 8)) };
        eq("ERRORS row 37: SPX_bytes_to_ull(inlen=8)", i, a, b);
        eq(
            "ERRORS row 37: SPX_bytes_to_ull(inlen=8) is big-endian",
            i,
            a,
            u64::from_be_bytes(buf.clone().try_into().unwrap()),
        );
    }
}

/* ================================================================== */
/* Rows 38-40 — degenerate arguments in the hash / WOTS layer          */
/* ================================================================== */

#[test]
fn err_38_thash_inblocks_zero() {
    let p = libs();
    let ic: FnInitHash = p.c.f("SPX_initialize_hash_function");
    let ir: FnInitHash = p.r.f("SPX_initialize_hash_function");
    let c: FnThash = p.c.f("SPX_thash");
    let r: FnThash = p.r.f("SPX_thash");
    let mut rng = Rng::for_row(38);
    for i in 0..8 {
        let pub_seed = rng.bytes(SPX_N);
        let sk_seed = rng.bytes(SPX_N);
        let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
        let mut cr = Ctx::with_seeds(&pub_seed, &sk_seed);
        unsafe {
            ic(cc.as_mut_ptr());
            ir(cr.as_mut_ptr());
        }
        let input = rng.bytes(SPX_N);
        let addr = addrs(&mut rng);
        let mut ac = addr;
        let mut ar = addr;
        let mut oc = vec![0xA5u8; SPX_N + 8];
        let mut or = oc.clone();
        unsafe {
            c(
                oc.as_mut_ptr(),
                input.as_ptr(),
                0,
                cc.as_ptr(),
                ac.as_mut_ptr(),
            );
            r(
                or.as_mut_ptr(),
                input.as_ptr(),
                0,
                cr.as_ptr(),
                ar.as_mut_ptr(),
            );
        }
        eq_bytes("ERRORS row 38: SPX_thash(inblocks=0)", i, &oc, &or);
        eq_u32s("ERRORS row 38: SPX_thash(inblocks=0)/addr", i, &ac, &ar);
    }
}

#[test]
fn err_39_chain_lengths_extremes() {
    let p = libs();
    let c: FnChainLengths = p.c.f("SPX_chain_lengths");
    let r: FnChainLengths = p.r.f("SPX_chain_lengths");
    let cases: Vec<Vec<u8>> = vec![
        vec![0x00u8; SPX_N],
        vec![0xFFu8; SPX_N],
        vec![0x0Fu8; SPX_N],
        vec![0xF0u8; SPX_N],
        {
            let mut v = vec![0x00u8; SPX_N];
            v[0] = 0xFF;
            v
        },
        {
            let mut v = vec![0xFFu8; SPX_N];
            v[SPX_N - 1] = 0x00;
            v
        },
    ];
    for (i, m) in cases.into_iter().enumerate() {
        let mut lc = vec![0u32; SPX_WOTS_LEN + 4];
        let mut lr = lc.clone();
        unsafe {
            c(lc.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
            r(lr.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
        }
        eq_u32s("ERRORS row 39: SPX_chain_lengths(extreme)", i, &lc, &lr);
    }
}

#[test]
fn err_40_wots_gen_leafx1_null_sig() {
    // `merkle_gen_root` calls this with wots_sign_leaf = ~0u and no signature
    // buffer, so the NULL `wots_sig` must never be dereferenced.
    let p = libs();
    let ic: FnInitHash = p.c.f("SPX_initialize_hash_function");
    let ir: FnInitHash = p.r.f("SPX_initialize_hash_function");
    let cl: FnChainLengths = p.c.f("SPX_chain_lengths");
    let c: FnWotsGenLeafX1 = p.c.f("SPX_wots_gen_leafx1");
    let r: FnWotsGenLeafX1 = p.r.f("SPX_wots_gen_leafx1");
    let mut rng = Rng::for_row(40);
    for i in 0..4 {
        let pub_seed = rng.bytes(SPX_N);
        let sk_seed = rng.bytes(SPX_N);
        let mut cc = Ctx::with_seeds(&pub_seed, &sk_seed);
        let mut cr = Ctx::with_seeds(&pub_seed, &sk_seed);
        unsafe {
            ic(cc.as_mut_ptr());
            ir(cr.as_mut_ptr());
        }
        let root = rng.bytes(SPX_N);
        let mut steps = vec![0u32; SPX_WOTS_LEN];
        unsafe { cl(steps.as_mut_ptr() as *mut core::ffi::c_uint, root.as_ptr()) };

        let base = addrs(&mut rng);
        let leaf_idx = rng.next_u32() & 0xFFFF;
        let mut infoc = LeafInfoX1 {
            wots_sig: core::ptr::null_mut(),
            wots_sign_leaf: !0u32,
            wots_steps: steps.as_mut_ptr(),
            leaf_addr: base[..8].try_into().unwrap(),
            pk_addr: base[8..16].try_into().unwrap(),
        };
        let mut infor = infoc;
        let mut dc = vec![0xA5u8; SPX_N + 8];
        let mut dr = dc.clone();
        unsafe {
            c(dc.as_mut_ptr(), cc.as_ptr(), leaf_idx, &mut infoc);
            r(dr.as_mut_ptr(), cr.as_ptr(), leaf_idx, &mut infor);
        }
        eq_bytes("ERRORS row 40: SPX_wots_gen_leafx1(NULL wots_sig)", i, &dc, &dr);
        eq_u32s(
            "ERRORS row 40: leaf_addr",
            i,
            &infoc.leaf_addr,
            &infor.leaf_addr,
        );
        eq_u32s("ERRORS row 40: pk_addr", i, &infoc.pk_addr, &infor.pk_addr);
    }
}
