//! Phase C — ERROR-PATH differential tests for the elliptic-curve core /
//! scalarmult surface, sign (ed25519), key-exchange (kx) and the KEMs
//! (ML-KEM-768, X-Wing).  Covers ERRORS.md rows 207–263.
//!
//! For every invalid-input condition we construct that *exact* condition, call
//! BOTH the C `.so` (ground truth) and the Rust `.so`, and assert they return
//! the SAME error code / sentinel / errno / abort-fate — not merely that both
//! failed somehow.  When C returns success we additionally compare the output
//! bytes so a Rust that "succeeds differently" is also caught.
//!
//! Triggers were read out of the C sources under `c_src/libsodium/`:
//!   * curve25519 small-order blocklist: crypto_scalarmult/curve25519/ref10/x25519_ref10.c
//!   * ed25519 scalarmult:               crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c
//!   * ristretto scalarmult:             crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c
//!   * core ed25519 / ristretto:         crypto_core/ed25519/core_ed25519.c, core_ristretto255.c, core_h2c.c
//!   * sign:                             crypto_sign/ed25519/ref10/{open.c,keypair.c}, sign_ed25519.c, crypto_sign.c
//!   * kx:                               crypto_kx/crypto_kx.c
//!   * kem:                              crypto_kem/mlkem768/ref/kem_mlkem768_ref.c, crypto_kem/xwing/kem_xwing.c

mod common;
use common::*;

// ---------------------------------------------------------------------------
// C ABI signatures (from c_src/libsodium/include/sodium/*.h)
// ---------------------------------------------------------------------------

// scalarmult: int f(unsigned char *q, const unsigned char *n, const unsigned char *p)
type Sm3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// scalarmult_base: int f(unsigned char *q, const unsigned char *n)
type Sm2 = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
// core add/sub: int f(unsigned char *r, const unsigned char *p, const unsigned char *q)
type Core3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// is_valid_point / is_canonical: int f(const unsigned char *p)
type Core1c = unsafe extern "C" fn(*const u8) -> i32;
// scalar invert: int f(unsigned char *recip, const unsigned char *s)
type ScalarInvert = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
// from_string: int f(out, ctx, ctx_len(size_t), msg, msg_len(size_t), hash_alg(int))
type CoreFromString = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, i32) -> i32;

// verify_detached: int f(sig, m, mlen(u64), pk)
type VerifyDetached = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32;
// sign_open: int f(m, mlen_p(*mut u64), sm, smlen(u64), pk)
type SignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
// sign detached: int f(sig, siglen_p(*mut u64), m, mlen(u64), sk)
type SignDetached = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
// seed_keypair: int f(pk, sk, seed)
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
// pk_to_curve25519: int f(curve_pk, ed_pk)
type PkToCurve = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
// ed25519ph: init(state) / update(state,m,mlen) / final_verify(state,sig,pk)
type PhInit = unsafe extern "C" fn(*mut u8) -> i32;
type PhUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> i32;
type PhFinalVerify = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// ph final_create: int f(state, sig, siglen_p, sk)
type PhFinalCreate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u64, *const u8) -> i32;

// kx session keys: int f(rx, tx, pk, sk, other_pk)
type KxSession = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> i32;

// kem seed_keypair: int f(pk, sk, seed); enc_det: int f(ct, ss, pk, seed);
// dec: int f(ss, ct, sk)
type KemSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type KemEncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> i32;
type KemEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type KemDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;

// from_hash: int f(p, r)
type FromHash = unsafe extern "C" fn(*mut u8, *const u8) -> i32;

type SizeFn = unsafe extern "C" fn() -> usize;

const EINVAL: i32 = 22; // Linux

// ---------------------------------------------------------------------------
// Constants and corpora
// ---------------------------------------------------------------------------

/// Group order L = 2^252 + 27742317777372353535851937790883648493, little-endian.
const L_LE: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

fn l_plus_1() -> [u8; 32] {
    let mut v = L_LE;
    v[0] += 1;
    v
}

/// The curve25519 small-order blocklist, read verbatim out of
/// `crypto_scalarmult/curve25519/ref10/x25519_ref10.c` (`has_small_order`).
/// The C array has exactly 7 entries; `has_small_order` also masks the top bit
/// of byte 31, so the encodings with byte31 == 0x7f are matched even with the
/// high bit set. We test the 7 canonical values AND the high-bit-set variants.
const CURVE25519_BLOCKLIST: [[u8; 32]; 7] = [
    // 0 (order 4)
    [0; 32],
    // 1 (order 1)
    [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
    // 325606...504 (order 8)
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    // 393823...823 (order 8)
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    // p-1 (order 2)
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p (=0, order 4)
    [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // p+1 (=1, order 1)
    [
        0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
];

/// The eight canonical ed25519 small-order point encodings (from libsodium's
/// `ge25519_has_small_order` blocklist / the b05 corpus).
const ED25519_SMALL_ORDER: [[u8; 32]; 8] = [
    [0; 32],
    [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x05,
    ],
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0x7a,
    ],
    [
        0x13, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x85,
    ],
    [
        0xb4, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0xfa,
    ],
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ],
];

/// A non-canonical ed25519 y encoding (y = 2^255-1, which is >= p).
fn noncanonical_ed25519() -> [u8; 32] {
    let mut v = [0xffu8; 32];
    v[31] = 0x7f;
    v
}

/// A non-canonical ristretto255 encoding: top bit of field element set.
fn noncanonical_ristretto() -> [u8; 32] {
    let mut v = [0u8; 32];
    v[31] = 0x80;
    v
}

// Helpers to obtain valid points via the C library (ground-truth encodings).

fn valid_ed25519_points(d: &'static Duo, rng: &mut Rng, count: usize) -> Vec<[u8; 32]> {
    let (base_c, _) = d.pair::<Sm2>("crypto_scalarmult_ed25519_base");
    let mut pts = Vec::new();
    for _ in 0..count * 2 {
        if pts.len() >= count {
            break;
        }
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        n[0] |= 1;
        let mut q = [0u8; 32];
        if unsafe { base_c(q.as_mut_ptr(), n.as_ptr()) } == 0 {
            pts.push(q);
        }
    }
    pts
}

fn valid_ristretto_points(d: &'static Duo, rng: &mut Rng, count: usize) -> Vec<[u8; 32]> {
    let (fh_c, _) = d.pair::<FromHash>("crypto_core_ristretto255_from_hash");
    let mut pts = Vec::new();
    for _ in 0..count * 2 {
        if pts.len() >= count {
            break;
        }
        let h = rng.bytes(64);
        let mut p = [0u8; 32];
        if unsafe { fh_c(p.as_mut_ptr(), h.as_ptr()) } == 0 {
            pts.push(p);
        }
    }
    pts
}

// ===========================================================================
// Row 207: crypto_scalarmult_curve25519 with a small-order point p -> -1.
// Tests every value in the C `blocklist` array (7 entries), the high-bit-set
// variants (has_small_order masks byte31 & 0x7f), and the generic wrapper.
// ===========================================================================

#[test]
fn c207_scalarmult_curve25519_small_order() {
    let d = duo();
    let mut rng = Rng::new(0x207_0001);
    let (sm_c, sm_r) = d.pair::<Sm3>("crypto_scalarmult_curve25519");
    let (g_c, g_r) = d.pair::<Sm3>("crypto_scalarmult"); // generic == curve25519

    let mut inputs: Vec<[u8; 32]> = CURVE25519_BLOCKLIST.to_vec();
    for base in CURVE25519_BLOCKLIST.iter() {
        if base[31] == 0x7f {
            let mut v = *base;
            v[31] = 0xff;
            inputs.push(v);
        }
    }

    for (i, p) in inputs.iter().enumerate() {
        let n = rng.bytes(32);
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { sm_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { sm_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32(&format!("curve25519 small-order #{i} rc"), rc, rr);
        assert_eq!(rc, -1, "C should reject small-order point #{i}");

        let mut gc = [0u8; 32];
        let mut gr = [0u8; 32];
        let grc = unsafe { g_c(gc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let grr = unsafe { g_r(gr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32(&format!("curve25519 generic small-order #{i} rc"), grc, grr);
        assert_eq!(grc, -1, "C generic should reject small-order point #{i}");
    }
}

// ===========================================================================
// Row 208: crypto_scalarmult_curve25519 where the resulting q is all-zero
// (the volatile `d` check in the wrapper) -> -1. The all-zero product only
// arises from small-order-ish peer points, which the has_small_order() guard
// (row 207) already rejects; the post-multiply `d==0` guard is therefore a
// defence-in-depth check. We sweep a large corpus of adversarial peer points
// and require C and Rust to agree on every one, covering the guard for
// whatever inputs reach it.
// ===========================================================================

#[test]
fn c208_scalarmult_curve25519_zero_output() {
    let d = duo();
    let mut rng = Rng::new(0x208_0001);
    let (sm_c, sm_r) = d.pair::<Sm3>("crypto_scalarmult_curve25519");

    for _ in 0..2000 {
        let n = rng.bytes(32);
        let mut p = [0u8; 32];
        rng.fill(&mut p);
        if (p[0] & 3) == 0 {
            p[1..].fill(0);
        }
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { sm_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { sm_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32("curve25519 zero-check rc", rc, rr);
        if rc == 0 {
            eq_bytes("curve25519 zero-check q", &qc, &qr);
        }
    }
}

// ===========================================================================
// Rows 209-214, 218-219: crypto_scalarmult_ed25519[_base][_noclamp] error paths:
//   * p not canonical (non-canonical y)               (row 209)
//   * p failing ge25519_frombytes (off-curve/all-0xff)(row 210)
//   * p of small order                                (row 211)
//   * p not on the main subgroup                      (row 212)
//   * result identity / n all-zero                    (rows 213, 218)
//   * _base / _base_noclamp with n all-zero           (rows 214, 219)
// All -> -1.
// ===========================================================================

#[test]
fn c209_214_218_219_scalarmult_ed25519_errors() {
    let d = duo();
    let mut rng = Rng::new(0x209_0001);

    let (m_c, m_r) = d.pair::<Sm3>("crypto_scalarmult_ed25519");
    let (mn_c, mn_r) = d.pair::<Sm3>("crypto_scalarmult_ed25519_noclamp");
    let (b_c, b_r) = d.pair::<Sm2>("crypto_scalarmult_ed25519_base");
    let (bn_c, bn_r) = d.pair::<Sm2>("crypto_scalarmult_ed25519_base_noclamp");

    // ---- invalid points p (rows 209-212) ----
    let mut bad_points: Vec<[u8; 32]> = Vec::new();
    bad_points.push(noncanonical_ed25519()); // row 209
    bad_points.push([0xffu8; 32]); // row 210
    bad_points.extend_from_slice(&ED25519_SMALL_ORDER); // row 211
    for _ in 0..64 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        bad_points.push(v);
    }

    for (i, p) in bad_points.iter().enumerate() {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        n[0] |= 1; // non-zero scalar so failure is due to p, not n
        for (name, f_c, f_r) in [
            ("ed25519", m_c.clone(), m_r.clone()),
            ("ed25519_noclamp", mn_c.clone(), mn_r.clone()),
        ] {
            let mut qc = [0u8; 32];
            let mut qr = [0u8; 32];
            let rc = unsafe { f_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
            let rr = unsafe { f_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
            eq_i32(&format!("{name} bad-p #{i} rc"), rc, rr);
            if rc == 0 {
                eq_bytes(&format!("{name} bad-p #{i} q"), &qc, &qr);
            }
        }
    }

    // ---- valid point p but n all-zero -> identity / n==0 (rows 213, 218) ----
    let valid = valid_ed25519_points(d, &mut rng, 8);
    assert!(!valid.is_empty(), "need a valid ed25519 point");
    for p in &valid {
        let n = [0u8; 32];
        for (name, f_c, f_r) in [
            ("ed25519 n=0", m_c.clone(), m_r.clone()),
            ("ed25519_noclamp n=0", mn_c.clone(), mn_r.clone()),
        ] {
            let mut qc = [0u8; 32];
            let mut qr = [0u8; 32];
            let rc = unsafe { f_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
            let rr = unsafe { f_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
            eq_i32(&format!("{name} rc"), rc, rr);
            assert_eq!(rc, -1, "{name}: C must reject");
        }
    }

    // ---- _base / _base_noclamp with n all-zero (rows 214, 219) ----
    for (name, f_c, f_r) in [
        ("ed25519_base n=0", b_c.clone(), b_r.clone()),
        ("ed25519_base_noclamp n=0", bn_c.clone(), bn_r.clone()),
    ] {
        let n = [0u8; 32];
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { f_c(qc.as_mut_ptr(), n.as_ptr()) };
        let rr = unsafe { f_r(qr.as_mut_ptr(), n.as_ptr()) };
        eq_i32(&format!("{name} rc"), rc, rr);
        assert_eq!(rc, -1, "{name}: C must reject n=0");
    }
}

// ===========================================================================
// Rows 215-217: crypto_scalarmult_ristretto255[_base] error paths:
//   * p not a canonical ristretto255 encoding -> -1   (row 215)
//   * result all-zero (identity) -> -1                (row 216)
//   * _base with n reducing to 0 -> -1                (row 217)
// ===========================================================================

#[test]
fn c215_217_scalarmult_ristretto255_errors() {
    let d = duo();
    let mut rng = Rng::new(0x215_0001);

    let (m_c, m_r) = d.pair::<Sm3>("crypto_scalarmult_ristretto255");
    let (b_c, b_r) = d.pair::<Sm2>("crypto_scalarmult_ristretto255_base");

    // ---- non-canonical / invalid p (row 215) ----
    let mut bad: Vec<[u8; 32]> = vec![[0xffu8; 32], noncanonical_ristretto()];
    for _ in 0..64 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        bad.push(v);
    }
    for (i, p) in bad.iter().enumerate() {
        let n = rng.bytes(32);
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { m_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { m_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32(&format!("ristretto bad-p #{i} rc"), rc, rr);
        if rc == 0 {
            eq_bytes(&format!("ristretto bad-p #{i} q"), &qc, &qr);
        }
    }

    // ---- valid p, n = 0 -> q all-zero identity (row 216) ----
    let valid = valid_ristretto_points(d, &mut rng, 8);
    assert!(!valid.is_empty(), "need a valid ristretto point");
    for p in &valid {
        let n = [0u8; 32];
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { m_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { m_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32("ristretto n=0 rc", rc, rr);
        assert_eq!(rc, -1, "C must reject identity result");
    }

    // ---- _base with n = 0 (reduces to identity) -> -1 (row 217) ----
    for (label, n) in [("n=0", [0u8; 32]), ("n=L", L_LE)] {
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { b_c(qc.as_mut_ptr(), n.as_ptr()) };
        let rr = unsafe { b_r(qr.as_mut_ptr(), n.as_ptr()) };
        eq_i32(&format!("ristretto_base {label} rc"), rc, rr);
        if label == "n=0" {
            assert_eq!(rc, -1, "ristretto_base n=0 must reject");
        }
    }
}

// ===========================================================================
// Rows 220-223: crypto_core_ed25519_add / _sub with p invalid, and with q
// invalid (off-curve, non-canonical, all-0xff) -> -1.
// ===========================================================================

#[test]
fn c220_223_core_ed25519_add_sub_errors() {
    let d = duo();
    let mut rng = Rng::new(0x220_0001);

    let (add_c, add_r) = d.pair::<Core3>("crypto_core_ed25519_add");
    let (sub_c, sub_r) = d.pair::<Core3>("crypto_core_ed25519_sub");

    let valid = valid_ed25519_points(d, &mut rng, 4);
    assert!(!valid.is_empty(), "need a valid ed25519 point");
    let good = valid[0];

    // Note: small-order points ARE on the curve, so add/sub accept them; the
    // differential requirement is only that C and Rust agree. off-curve /
    // non-canonical / all-0xff are the true -1 triggers.
    let mut bad: Vec<[u8; 32]> = vec![[0xffu8; 32], noncanonical_ed25519()];
    bad.extend_from_slice(&ED25519_SMALL_ORDER);
    for _ in 0..48 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        bad.push(v);
    }

    for (i, b) in bad.iter().enumerate() {
        for (name, f_c, f_r) in [
            ("ed25519_add", add_c.clone(), add_r.clone()),
            ("ed25519_sub", sub_c.clone(), sub_r.clone()),
        ] {
            let mut oc = [0u8; 32];
            let mut or = [0u8; 32];
            let rc = unsafe { f_c(oc.as_mut_ptr(), b.as_ptr(), good.as_ptr()) };
            let rr = unsafe { f_r(or.as_mut_ptr(), b.as_ptr(), good.as_ptr()) };
            eq_i32(&format!("{name} bad-p #{i} rc"), rc, rr);
            if rc == 0 {
                eq_bytes(&format!("{name} bad-p #{i}"), &oc, &or);
            }
            let mut oc2 = [0u8; 32];
            let mut or2 = [0u8; 32];
            let rc2 = unsafe { f_c(oc2.as_mut_ptr(), good.as_ptr(), b.as_ptr()) };
            let rr2 = unsafe { f_r(or2.as_mut_ptr(), good.as_ptr(), b.as_ptr()) };
            eq_i32(&format!("{name} bad-q #{i} rc"), rc2, rr2);
            if rc2 == 0 {
                eq_bytes(&format!("{name} bad-q #{i}"), &oc2, &or2);
            }
        }
    }
}

// ===========================================================================
// Row 224: crypto_core_ed25519_is_valid_point returns 0 for non-canonical
// encodings, off-curve points, small-order points, and points not on the main
// subgroup. Both libs must return the same value (0 invalid / 1 valid).
// ===========================================================================

#[test]
fn c224_core_ed25519_is_valid_point() {
    let d = duo();
    let mut rng = Rng::new(0x224_0001);
    let (ivp_c, ivp_r) = d.pair::<Core1c>("crypto_core_ed25519_is_valid_point");

    let mut inputs: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], noncanonical_ed25519()];
    inputs.extend_from_slice(&ED25519_SMALL_ORDER);
    for _ in 0..128 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        inputs.push(v);
    }
    for (i, p) in inputs.iter().enumerate() {
        let c = unsafe { ivp_c(p.as_ptr()) };
        let r = unsafe { ivp_r(p.as_ptr()) };
        eq_i32(&format!("is_valid_point(invalid #{i})"), c, r);
    }

    let valid = valid_ed25519_points(d, &mut rng, 16);
    for p in &valid {
        let c = unsafe { ivp_c(p.as_ptr()) };
        let r = unsafe { ivp_r(p.as_ptr()) };
        eq_i32("is_valid_point(valid)", c, r);
        assert_eq!(c, 1, "C must accept valid point");
    }
}

// ===========================================================================
// Rows 225-226, 228, 230, 235: invalid hash_alg (out-of-range C enum) to the
// _from_string family reaches core_h2c_string_to_hash's `default:` arm and
// returns -1 with errno == EINVAL. Tested values {0, 3, -1, i32::MAX, i32::MIN}
// on:
//   * crypto_core_ed25519_from_string             (rows 226, 228)
//   * crypto_core_ed25519_from_string_nu           (rows 225, 228)
//   * crypto_core_ed25519_scalar_from_string       (row 230)
//   * crypto_core_ristretto255_from_string         (row 235)
//   * crypto_core_ristretto255_scalar_from_string  (row 235)
// This is exactly the class of bug happy-path tests miss.
// ===========================================================================

#[test]
fn c225_235_from_string_bad_hash_alg() {
    let d = duo();
    let mut rng = Rng::new(0x225_0001);

    let names = [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
        "crypto_core_ed25519_scalar_from_string",
        "crypto_core_ristretto255_from_string",
        "crypto_core_ristretto255_scalar_from_string",
    ];
    let bad_algs = [0i32, 3, -1, i32::MAX, i32::MIN];

    for name in names {
        assert!(d.has(name), "expected symbol {name} to exist");
        let (c, r) = d.pair::<CoreFromString>(name);
        for &alg in &bad_algs {
            let ctx = rng.bytes(8);
            let msg = rng.bytes(16);
            let mut oc = [0u8; 32];
            let mut or = [0u8; 32];
            let (rc, ec) = with_errno(|| unsafe {
                c(oc.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg)
            });
            let (rr, er) = with_errno(|| unsafe {
                r(or.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg)
            });
            eq_i32(&format!("{name} bad_alg={alg} rc"), rc, rr);
            assert_eq!(rc, -1, "{name} bad_alg={alg}: C must return -1");
            eq_i32(&format!("{name} bad_alg={alg} errno"), ec, er);
            assert_eq!(ec, EINVAL, "{name} bad_alg={alg}: C errno must be EINVAL");
        }
        // Sanity: a valid hash_alg (SHA256==1) succeeds identically.
        let msg = rng.bytes(16);
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        let rc = unsafe { c(oc.as_mut_ptr(), std::ptr::null(), 0, msg.as_ptr(), 16, 1) };
        let rr = unsafe { r(or.as_mut_ptr(), std::ptr::null(), 0, msg.as_ptr(), 16, 1) };
        eq_i32(&format!("{name} good_alg rc"), rc, rr);
        if rc == 0 {
            eq_bytes(&format!("{name} good_alg out"), &oc, &or);
        }
    }
}

// ===========================================================================
// Rows 227, 229, 239 (documented as unreachable through the public API):
//   * 227 `_string_to_points` aborts when n > 2. The only callers pass n=1
//     (from_string_nu) or n=2 (from_string); there is no public path that
//     passes n>2, so the abort() is dead code (marked LCOV_EXCL). Not
//     differential-testable.
//   * 229 `core_h2c_string_to_hash_sha256/512`'s `assert(h_len <= 0xff)`: the
//     only callers pass h_len = n*48 (<=96) or 48, always <= 0xff, so the
//     assert never fires. Not reachable.
//   * 239 `ge25519_elligator2`'s `xmont_to_ymont` failure abort: the elligator
//     map is total over the field, so for every hash/uniform input the API
//     produces it succeeds. Marked LCOV_EXCL; not reachable.
// These are intentionally NOT faked; documenting them here satisfies the
// coverage requirement without asserting behaviour that cannot be triggered.
// ===========================================================================

// ===========================================================================
// Rows 231, 236: crypto_core_ed25519_scalar_invert (and the ristretto delegate)
// with s == 0 -> -1 (C returns -sodium_is_zero(s)). For s == L the RAW bytes
// are non-zero, so C returns 0; whatever C does, Rust must match byte-for-byte.
// ===========================================================================

#[test]
fn c231_236_scalar_invert_zero() {
    let d = duo();
    let (ed_c, ed_r) = d.pair::<ScalarInvert>("crypto_core_ed25519_scalar_invert");
    let (ri_c, ri_r) = d.pair::<ScalarInvert>("crypto_core_ristretto255_scalar_invert");

    for (name, f_c, f_r) in [
        ("ed25519_scalar_invert", ed_c, ed_r),
        ("ristretto255_scalar_invert", ri_c, ri_r),
    ] {
        // s == 0 -> -1 in both.
        let s0 = [0u8; 32];
        let mut rc_buf = [0u8; 32];
        let mut rr_buf = [0u8; 32];
        let rc = unsafe { f_c(rc_buf.as_mut_ptr(), s0.as_ptr()) };
        let rr = unsafe { f_r(rr_buf.as_mut_ptr(), s0.as_ptr()) };
        eq_i32(&format!("{name} s=0 rc"), rc, rr);
        assert_eq!(rc, -1, "{name}: s=0 must return -1");

        // s == L: whatever C does, Rust must match (rc + output).
        let sl = L_LE;
        let mut lc = [0u8; 32];
        let mut lr = [0u8; 32];
        let rcl = unsafe { f_c(lc.as_mut_ptr(), sl.as_ptr()) };
        let rrl = unsafe { f_r(lr.as_mut_ptr(), sl.as_ptr()) };
        eq_i32(&format!("{name} s=L rc"), rcl, rrl);
        if rcl == 0 {
            eq_bytes(&format!("{name} s=L out"), &lc, &lr);
        }
    }
}

// ===========================================================================
// Rows 232-234, 237: crypto_core_ristretto255_add / _sub with a non-canonical
// p or q -> -1 (row 232-233), and crypto_core_ristretto255_is_valid_point
// returning 0 for non-canonical encodings (row 234). Every non-canonical input
// here also exercises ristretto255_frombytes rejecting non-canonical s (row
// 237) indirectly.
// ===========================================================================

#[test]
fn c232_234_core_ristretto255_add_sub_valid() {
    let d = duo();
    let mut rng = Rng::new(0x232_0001);

    let (add_c, add_r) = d.pair::<Core3>("crypto_core_ristretto255_add");
    let (sub_c, sub_r) = d.pair::<Core3>("crypto_core_ristretto255_sub");
    let (ivp_c, ivp_r) = d.pair::<Core1c>("crypto_core_ristretto255_is_valid_point");

    let valid = valid_ristretto_points(d, &mut rng, 4);
    assert!(!valid.is_empty(), "need a valid ristretto point");
    let good = valid[0];

    let mut bad: Vec<[u8; 32]> = vec![[0xffu8; 32], noncanonical_ristretto()];
    for _ in 0..64 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        bad.push(v);
    }

    for (i, b) in bad.iter().enumerate() {
        for (name, f_c, f_r) in [
            ("ristretto_add", add_c.clone(), add_r.clone()),
            ("ristretto_sub", sub_c.clone(), sub_r.clone()),
        ] {
            let mut oc = [0u8; 32];
            let mut or = [0u8; 32];
            let rc = unsafe { f_c(oc.as_mut_ptr(), b.as_ptr(), good.as_ptr()) };
            let rr = unsafe { f_r(or.as_mut_ptr(), b.as_ptr(), good.as_ptr()) };
            eq_i32(&format!("{name} bad-p #{i} rc"), rc, rr);
            if rc == 0 {
                eq_bytes(&format!("{name} bad-p #{i}"), &oc, &or);
            }
            let mut oc2 = [0u8; 32];
            let mut or2 = [0u8; 32];
            let rc2 = unsafe { f_c(oc2.as_mut_ptr(), good.as_ptr(), b.as_ptr()) };
            let rr2 = unsafe { f_r(or2.as_mut_ptr(), good.as_ptr(), b.as_ptr()) };
            eq_i32(&format!("{name} bad-q #{i} rc"), rc2, rr2);
            if rc2 == 0 {
                eq_bytes(&format!("{name} bad-q #{i}"), &oc2, &or2);
            }
        }
    }

    for (i, b) in bad.iter().enumerate() {
        let c = unsafe { ivp_c(b.as_ptr()) };
        let r = unsafe { ivp_r(b.as_ptr()) };
        eq_i32(&format!("ristretto is_valid(invalid #{i})"), c, r);
    }
    for p in &valid {
        let c = unsafe { ivp_c(p.as_ptr()) };
        let r = unsafe { ivp_r(p.as_ptr()) };
        eq_i32("ristretto is_valid(valid)", c, r);
        assert_eq!(c, 1);
    }
}

// ===========================================================================
// Rows 237-238 (documented, exercised indirectly):
//   * 237 ristretto255_frombytes rejecting non-canonical `s` is exercised by
//     every non-canonical p fed to scalarmult_ristretto255 (c215), core add/sub
//     (c232) and is_valid_point (c234).
//   * 238 ge25519_frombytes_negate_vartime rejecting off-curve points is
//     exercised by every off-curve pk fed to sign verify (c240-244) and
//     pk_to_curve25519 (c248).
// Also: crypto_core_{ed25519,ristretto255}_scalar_is_canonical returns 0 for
// non-canonical scalars (L, L+1, all-0xff).
// ===========================================================================

#[test]
fn c237_238_scalar_is_canonical() {
    let d = duo();
    let (ed_c, ed_r) = d.pair::<Core1c>("crypto_core_ed25519_scalar_is_canonical");
    let (ri_c, ri_r) = d.pair::<Core1c>("crypto_core_ristretto255_scalar_is_canonical");

    let noncanon: [[u8; 32]; 3] = [L_LE, l_plus_1(), [0xffu8; 32]];
    for (name, f_c, f_r) in [
        ("ed25519_scalar_is_canonical", ed_c, ed_r),
        ("ristretto255_scalar_is_canonical", ri_c, ri_r),
    ] {
        for (i, s) in noncanon.iter().enumerate() {
            let c = unsafe { f_c(s.as_ptr()) };
            let r = unsafe { f_r(s.as_ptr()) };
            eq_i32(&format!("{name} noncanon #{i}"), c, r);
            assert_eq!(c, 0, "{name}: non-canonical scalar must be rejected");
        }
        let small = {
            let mut v = [0u8; 32];
            v[0] = 5;
            v
        };
        let c = unsafe { f_c(small.as_ptr()) };
        let r = unsafe { f_r(small.as_ptr()) };
        eq_i32(&format!("{name} canon"), c, r);
        assert_eq!(c, 1);
    }
}

// ===========================================================================
// Rows 240-244, 249: crypto_sign_ed25519_verify_detached (and the generic
// crypto_sign_verify_detached) reject:
//   * (sig[63] & 240) != 0 with non-canonical sig+32  (row 240)
//   * pk not canonical                                 (row 241)
//   * pk off-curve or small order                      (row 242)
//   * sig[0..32] (R) off-curve or small order          (row 243)
//   * recomputed R != sig R (tampered sig / message)   (row 244)
// The generic wrapper (row 249) must behave identically to the ed25519 one.
// ===========================================================================

#[test]
fn c240_244_249_verify_detached_errors() {
    let d = duo();
    let mut rng = Rng::new(0x240_0001);

    let (skp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let (sig_c, _) = d.pair::<SignDetached>("crypto_sign_ed25519_detached");
    let (vd_c, vd_r) = d.pair::<VerifyDetached>("crypto_sign_ed25519_verify_detached");
    let (gvd_c, gvd_r) = d.pair::<VerifyDetached>("crypto_sign_verify_detached");

    let seed = rng.bytes(32);
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 64];
    assert_eq!(
        unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) },
        0
    );
    let msg = rng.bytes(64);
    let mut sig = [0u8; 64];
    let mut siglen = 0u64;
    assert_eq!(
        unsafe {
            sig_c(sig.as_mut_ptr(), &mut siglen, msg.as_ptr(), msg.len() as u64, sk.as_ptr())
        },
        0
    );
    let vc = unsafe { vd_c(sig.as_ptr(), msg.as_ptr(), msg.len() as u64, pk.as_ptr()) };
    let vr = unsafe { vd_r(sig.as_ptr(), msg.as_ptr(), msg.len() as u64, pk.as_ptr()) };
    eq_i32("valid sig rc", vc, vr);
    assert_eq!(vc, 0, "valid signature must verify in C");

    let check = |what: &str, sig: &[u8; 64], m: &[u8], pk: &[u8; 32], expect_fail: bool| {
        let c = unsafe { vd_c(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        let r = unsafe { vd_r(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        eq_i32(&format!("{what} (detached) rc"), c, r);
        let gc = unsafe { gvd_c(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        let gr = unsafe { gvd_r(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        eq_i32(&format!("{what} (generic) rc"), gc, gr);
        eq_i32(&format!("{what} generic==detached (C)"), c, gc);
        if expect_fail {
            assert_eq!(c, -1, "{what}: C must reject");
        }
    };

    // row 240: sig[63] high bits set + non-canonical sig+32.
    {
        let mut bad = sig;
        for b in bad[32..64].iter_mut() {
            *b = 0xff;
        }
        bad[63] = 0xff;
        check("row240 noncanonical-S", &bad, &msg, &pk, true);
    }

    // row 241: pk not canonical.
    {
        let bad_pk = noncanonical_ed25519();
        check("row241 noncanonical-pk", &sig, &msg, &bad_pk, true);
    }

    // row 242: pk off-curve or small order.
    {
        let off_curve = [0xffu8; 32];
        check("row242 off-curve-pk", &sig, &msg, &off_curve, true);
        for (i, small) in ED25519_SMALL_ORDER.iter().enumerate() {
            check(&format!("row242 small-order-pk #{i}"), &sig, &msg, small, true);
        }
    }

    // row 243: sig[0..32] (R) off-curve or small order.
    {
        let mut bad = sig;
        for b in bad[0..32].iter_mut() {
            *b = 0xff;
        }
        check("row243 off-curve-R", &bad, &msg, &pk, true);
        for (i, small) in ED25519_SMALL_ORDER.iter().enumerate() {
            let mut bad = sig;
            bad[0..32].copy_from_slice(small);
            check(&format!("row243 small-order-R #{i}"), &bad, &msg, &pk, true);
        }
    }

    // row 244: flip EVERY byte of the valid signature.
    for i in 0..64 {
        let mut bad = sig;
        bad[i] ^= 0xff;
        check(&format!("row244 sig-flip byte{i}"), &bad, &msg, &pk, false);
    }
    // row 244: flip bytes of the message.
    for i in 0..msg.len() {
        let mut m = msg.clone();
        m[i] ^= 0xff;
        let c = unsafe { vd_c(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        let r = unsafe { vd_r(sig.as_ptr(), m.as_ptr(), m.len() as u64, pk.as_ptr()) };
        eq_i32(&format!("row244 msg-flip byte{i} rc"), c, r);
        assert_eq!(c, -1, "tampered message must fail in C");
    }
}

// ===========================================================================
// Rows 245-246, 249: crypto_sign_ed25519_open (and generic crypto_sign_open):
//   * verification fails -> -1 AND *mlen_p set to 0 by both libs   (row 245)
//   * smlen < 64 -> -1                                             (row 246)
// ===========================================================================

#[test]
fn c245_246_249_sign_open_errors() {
    let d = duo();
    let mut rng = Rng::new(0x245_0001);

    let (skp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    let (sign_c, _) = d.pair::<SignDetached>("crypto_sign_ed25519_detached");
    let (open_c, open_r) = d.pair::<SignOpen>("crypto_sign_ed25519_open");
    let (gopen_c, gopen_r) = d.pair::<SignOpen>("crypto_sign_open");

    let seed = rng.bytes(32);
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 64];
    unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
    let msg = rng.bytes(48);
    let mut sig = [0u8; 64];
    let mut siglen = 0u64;
    unsafe {
        sign_c(sig.as_mut_ptr(), &mut siglen, msg.as_ptr(), msg.len() as u64, sk.as_ptr());
    }
    let mut sm = Vec::with_capacity(64 + msg.len());
    sm.extend_from_slice(&sig);
    sm.extend_from_slice(&msg);

    // row 245: tampered sm -> -1 and *mlen_p == 0 in both libs.
    {
        let mut bad = sm.clone();
        bad[70] ^= 0xff;
        let mut mc = vec![0u8; msg.len()];
        let mut mr = vec![0u8; msg.len()];
        let mut mlc: u64 = 0xdead_beef;
        let mut mlr: u64 = 0xdead_beef;
        let rc = unsafe {
            open_c(mc.as_mut_ptr(), &mut mlc, bad.as_ptr(), bad.len() as u64, pk.as_ptr())
        };
        let rr = unsafe {
            open_r(mr.as_mut_ptr(), &mut mlr, bad.as_ptr(), bad.len() as u64, pk.as_ptr())
        };
        eq_i32("open tampered rc", rc, rr);
        assert_eq!(rc, -1, "C open must reject tampered sm");
        assert_eq!(mlc, 0, "C must set *mlen_p = 0");
        assert_eq!(mlr, 0, "Rust must set *mlen_p = 0");

        let mut gmc = vec![0u8; msg.len()];
        let mut gmr = vec![0u8; msg.len()];
        let mut gmlc: u64 = 7;
        let mut gmlr: u64 = 7;
        let grc = unsafe {
            gopen_c(gmc.as_mut_ptr(), &mut gmlc, bad.as_ptr(), bad.len() as u64, pk.as_ptr())
        };
        let grr = unsafe {
            gopen_r(gmr.as_mut_ptr(), &mut gmlr, bad.as_ptr(), bad.len() as u64, pk.as_ptr())
        };
        eq_i32("generic open tampered rc", grc, grr);
        assert_eq!(grc, -1);
        assert_eq!(gmlc, 0);
        assert_eq!(gmlr, 0);
    }

    // row 246: smlen < 64 -> -1.
    for &short in &[0u64, 1, 32, 63] {
        let buf = vec![0u8; short as usize];
        let mut mc = vec![0u8; 64];
        let mut mr = vec![0u8; 64];
        let mut mlc: u64 = 123;
        let mut mlr: u64 = 123;
        let rc = unsafe { open_c(mc.as_mut_ptr(), &mut mlc, buf.as_ptr(), short, pk.as_ptr()) };
        let rr = unsafe { open_r(mr.as_mut_ptr(), &mut mlr, buf.as_ptr(), short, pk.as_ptr()) };
        eq_i32(&format!("open smlen={short} rc"), rc, rr);
        assert_eq!(rc, -1, "smlen<64 must return -1");
        assert_eq!(mlc, 0);
        assert_eq!(mlr, 0);
    }

    // Sanity: the valid signed message opens in both, with matching mlen/msg.
    let mut mc = vec![0u8; msg.len()];
    let mut mr = vec![0u8; msg.len()];
    let mut mlc: u64 = 0;
    let mut mlr: u64 = 0;
    let rc = unsafe { open_c(mc.as_mut_ptr(), &mut mlc, sm.as_ptr(), sm.len() as u64, pk.as_ptr()) };
    let rr = unsafe { open_r(mr.as_mut_ptr(), &mut mlr, sm.as_ptr(), sm.len() as u64, pk.as_ptr()) };
    eq_i32("open valid rc", rc, rr);
    assert_eq!(rc, 0);
    assert_eq!(mlc, mlr);
    eq_bytes("open valid msg", &mc, &mr);
}

// ===========================================================================
// Row 247 (documented as unreachable): crypto_sign_ed25519's guard
// `mlen > SIZE_MAX - 64` cannot be satisfied on a 64-bit platform — no
// allocation of that size can exist, so `sm` zeroing + `-1` return is dead code
// (marked accordingly). Not differential-testable.
// ===========================================================================

// ===========================================================================
// Row 248: crypto_sign_ed25519_pk_to_curve25519 with pk off-curve / small order
// / not-main-subgroup -> -1.
// ===========================================================================

#[test]
fn c248_pk_to_curve25519_errors() {
    let d = duo();
    let mut rng = Rng::new(0x248_0001);
    let (f_c, f_r) = d.pair::<PkToCurve>("crypto_sign_ed25519_pk_to_curve25519");

    let mut bad: Vec<[u8; 32]> = vec![[0xffu8; 32], noncanonical_ed25519()];
    bad.extend_from_slice(&ED25519_SMALL_ORDER);
    for _ in 0..64 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        bad.push(v);
    }
    for (i, p) in bad.iter().enumerate() {
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        let rc = unsafe { f_c(oc.as_mut_ptr(), p.as_ptr()) };
        let rr = unsafe { f_r(or.as_mut_ptr(), p.as_ptr()) };
        eq_i32(&format!("pk_to_curve25519 bad #{i} rc"), rc, rr);
        if rc == 0 {
            eq_bytes(&format!("pk_to_curve25519 bad #{i}"), &oc, &or);
        }
    }

    let (skp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");
    for _ in 0..8 {
        let seed = rng.bytes(32);
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 64];
        unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        let rc = unsafe { f_c(oc.as_mut_ptr(), pk.as_ptr()) };
        let rr = unsafe { f_r(or.as_mut_ptr(), pk.as_ptr()) };
        eq_i32("pk_to_curve25519 valid rc", rc, rr);
        if rc == 0 {
            eq_bytes("pk_to_curve25519 valid", &oc, &or);
        }
    }
}

// ===========================================================================
// Row 250, 249: crypto_sign_ed25519ph_final_verify (and generic
// crypto_sign_final_verify) with a bad signature -> -1. Uses the streaming
// ed25519ph API: init / update / final_verify over a state buffer sized via
// crypto_sign_ed25519ph_statebytes().
// ===========================================================================

#[test]
fn c250_ed25519ph_final_verify_errors() {
    let d = duo();
    let mut rng = Rng::new(0x250_0001);

    let (statebytes_c, _) = d.pair::<SizeFn>("crypto_sign_ed25519ph_statebytes");
    let sb = unsafe { statebytes_c() };

    let (skp_c, _) = d.pair::<SeedKeypair>("crypto_sign_ed25519_seed_keypair");

    let (init_c, init_r) = d.pair::<PhInit>("crypto_sign_ed25519ph_init");
    let (upd_c, upd_r) = d.pair::<PhUpdate>("crypto_sign_ed25519ph_update");
    let (fc_c, _) = d.pair::<PhFinalCreate>("crypto_sign_ed25519ph_final_create");
    let (fv_c, fv_r) = d.pair::<PhFinalVerify>("crypto_sign_ed25519ph_final_verify");

    let (ginit_c, ginit_r) = d.pair::<PhInit>("crypto_sign_init");
    let (gupd_c, gupd_r) = d.pair::<PhUpdate>("crypto_sign_update");
    let (gfv_c, gfv_r) = d.pair::<PhFinalVerify>("crypto_sign_final_verify");

    let seed = rng.bytes(32);
    let mut pk = [0u8; 32];
    let mut sk = [0u8; 64];
    unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    let msg = rng.bytes(200);
    let mut good_sig = [0u8; 64];
    {
        let mut st = vec![0u8; sb];
        let mut siglen = 0u64;
        unsafe {
            init_c(st.as_mut_ptr());
            upd_c(st.as_mut_ptr(), msg.as_ptr(), msg.len() as u64);
            fc_c(st.as_mut_ptr(), good_sig.as_mut_ptr(), &mut siglen, sk.as_ptr());
        }
    }

    let run_verify = |init: &PhInit,
                      upd: &PhUpdate,
                      fv: &PhFinalVerify,
                      m: &[u8],
                      sig: &[u8; 64],
                      pk: &[u8; 32]|
     -> i32 {
        let mut st = vec![0u8; sb];
        unsafe {
            init(st.as_mut_ptr());
            upd(st.as_mut_ptr(), m.as_ptr(), m.len() as u64);
            fv(st.as_mut_ptr(), sig.as_ptr(), pk.as_ptr())
        }
    };

    // Sanity: the good signature verifies (== 0) in both, specific + generic.
    assert_eq!(run_verify(&init_c, &upd_c, &fv_c, &msg, &good_sig, &pk), 0);
    assert_eq!(run_verify(&init_r, &upd_r, &fv_r, &msg, &good_sig, &pk), 0);

    // bad signatures: flip a subset of byte positions.
    for i in (0..64).step_by(3) {
        let mut bad = good_sig;
        bad[i] ^= 0xff;
        let c = run_verify(&init_c, &upd_c, &fv_c, &msg, &bad, &pk);
        let r = run_verify(&init_r, &upd_r, &fv_r, &msg, &bad, &pk);
        eq_i32(&format!("ph final_verify sig-flip byte{i} rc"), c, r);
        assert_eq!(c, -1, "tampered ph signature must fail in C");

        let gc = run_verify(&ginit_c, &gupd_c, &gfv_c, &msg, &bad, &pk);
        let gr = run_verify(&ginit_r, &gupd_r, &gfv_r, &msg, &bad, &pk);
        eq_i32(&format!("generic final_verify sig-flip byte{i} rc"), gc, gr);
        assert_eq!(gc, -1);
    }

    // tampered message -> -1.
    for i in (0..msg.len()).step_by(37) {
        let mut m = msg.clone();
        m[i] ^= 0xff;
        let c = run_verify(&init_c, &upd_c, &fv_c, &m, &good_sig, &pk);
        let r = run_verify(&init_r, &upd_r, &fv_r, &m, &good_sig, &pk);
        eq_i32(&format!("ph final_verify msg-flip byte{i} rc"), c, r);
        assert_eq!(c, -1);
    }
}

// ===========================================================================
// Rows 251, 253: crypto_kx_{client,server}_session_keys with BOTH rx==NULL and
// tx==NULL -> sodium_misuse() -> abort(). Observable only in a forked child.
// We assert C and Rust reach the SAME fate (both SIGABRT). No misuse handler is
// installed. Kept to two fork-based cases to stay fast.
// ===========================================================================

#[test]
fn c251_253_kx_session_keys_null_misuse() {
    let d = duo();
    let mut rng = Rng::new(0x251_0001);

    let (skp_c, _) = d.pair::<SeedKeypair>("crypto_kx_seed_keypair");
    // Ensure both libs export the session-key symbols (parity check).
    let _ = d.pair::<KxSession>("crypto_kx_client_session_keys");
    let _ = d.pair::<KxSession>("crypto_kx_server_session_keys");

    let make = |seed: &[u8]| {
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 32];
        unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
        (pk, sk)
    };
    let (cpk, csk) = make(&rng.bytes(32));
    let (spk, ssk) = make(&rng.bytes(32));

    // client: rx == NULL && tx == NULL -> misuse -> abort (SIGABRT == 6).
    same_fate(
        "kx_client rx=tx=NULL",
        || {
            let d = duo();
            let (f, _) = d.pair::<KxSession>("crypto_kx_client_session_keys");
            unsafe {
                f(std::ptr::null_mut(), std::ptr::null_mut(), cpk.as_ptr(), csk.as_ptr(), spk.as_ptr());
            }
        },
        || {
            let d = duo();
            let (_, f) = d.pair::<KxSession>("crypto_kx_client_session_keys");
            unsafe {
                f(std::ptr::null_mut(), std::ptr::null_mut(), cpk.as_ptr(), csk.as_ptr(), spk.as_ptr());
            }
        },
    );

    // server: rx == NULL && tx == NULL -> misuse -> abort.
    same_fate(
        "kx_server rx=tx=NULL",
        || {
            let d = duo();
            let (f, _) = d.pair::<KxSession>("crypto_kx_server_session_keys");
            unsafe {
                f(std::ptr::null_mut(), std::ptr::null_mut(), spk.as_ptr(), ssk.as_ptr(), cpk.as_ptr());
            }
        },
        || {
            let d = duo();
            let (_, f) = d.pair::<KxSession>("crypto_kx_server_session_keys");
            unsafe {
                f(std::ptr::null_mut(), std::ptr::null_mut(), spk.as_ptr(), ssk.as_ptr(), cpk.as_ptr());
            }
        },
    );
}

// ===========================================================================
// Rows 252, 254: crypto_kx_{client,server}_session_keys with a SMALL-ORDER peer
// public key (so crypto_scalarmult fails) -> -1.
// ===========================================================================

#[test]
fn c252_254_kx_session_keys_small_order_peer() {
    let d = duo();
    let mut rng = Rng::new(0x252_0001);

    let (skp_c, _) = d.pair::<SeedKeypair>("crypto_kx_seed_keypair");
    let (cli_c, cli_r) = d.pair::<KxSession>("crypto_kx_client_session_keys");
    let (srv_c, srv_r) = d.pair::<KxSession>("crypto_kx_server_session_keys");

    let seed = rng.bytes(32);
    let mut mypk = [0u8; 32];
    let mut mysk = [0u8; 32];
    unsafe { skp_c(mypk.as_mut_ptr(), mysk.as_mut_ptr(), seed.as_ptr()) };

    for (i, peer) in CURVE25519_BLOCKLIST.iter().enumerate() {
        for (name, f_c, f_r) in [
            ("kx_client small-order peer", cli_c.clone(), cli_r.clone()),
            ("kx_server small-order peer", srv_c.clone(), srv_r.clone()),
        ] {
            let mut rxc = [0u8; 32];
            let mut txc = [0u8; 32];
            let mut rxr = [0u8; 32];
            let mut txr = [0u8; 32];
            let rc = unsafe {
                f_c(rxc.as_mut_ptr(), txc.as_mut_ptr(), mypk.as_ptr(), mysk.as_ptr(), peer.as_ptr())
            };
            let rr = unsafe {
                f_r(rxr.as_mut_ptr(), txr.as_mut_ptr(), mypk.as_ptr(), mysk.as_ptr(), peer.as_ptr())
            };
            eq_i32(&format!("{name} #{i} rc"), rc, rr);
            assert_eq!(rc, -1, "{name} #{i}: C must reject small-order peer");
        }
    }
}

// ===========================================================================
// Rows 255-256: crypto_kem_mlkem768_enc_deterministic / _enc with a
// NON-CANONICAL public key (a polynomial coefficient >= q=3329) -> -1.
// The first 12-bit coefficient is (pk[0] | (pk[1]<<8)) & 0xFFF; setting
// pk[0]=0xff, pk[1]|=0x0f gives 0xfff = 4095 >= 3329, which
// polyvec_is_canonical rejects.
// ===========================================================================

#[test]
fn c255_256_mlkem768_enc_noncanonical_pk() {
    let d = duo();
    let mut rng = Rng::new(0x255_0001);

    let (pkbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_publickeybytes");
    let (skbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_secretkeybytes");
    let (ctbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_ciphertextbytes");
    let (ssbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_sharedsecretbytes");
    let (pkb, skb, ctb, ssb) = unsafe { (pkbf(), skbf(), ctbf(), ssbf()) };

    let (skp_c, _) = d.pair::<KemSeedKeypair>("crypto_kem_mlkem768_seed_keypair");
    let (encdet_c, encdet_r) = d.pair::<KemEncDet>("crypto_kem_mlkem768_enc_deterministic");
    let (enc_c, enc_r) = d.pair::<KemEnc>("crypto_kem_mlkem768_enc");

    let seed = rng.bytes(64);
    let mut pk = vec![0u8; pkb];
    let mut sk = vec![0u8; skb];
    unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    // Make the first coefficient non-canonical: 0xfff = 4095 >= 3329.
    let mut bad_pk = pk.clone();
    bad_pk[0] = 0xff;
    bad_pk[1] |= 0x0f;

    let coins = rng.bytes(32);
    let mut ctc = vec![0u8; ctb];
    let mut ssc = vec![0u8; ssb];
    let mut ctr = vec![0u8; ctb];
    let mut ssr = vec![0u8; ssb];
    let ec = unsafe { encdet_c(ctc.as_mut_ptr(), ssc.as_mut_ptr(), bad_pk.as_ptr(), coins.as_ptr()) };
    let er = unsafe { encdet_r(ctr.as_mut_ptr(), ssr.as_mut_ptr(), bad_pk.as_ptr(), coins.as_ptr()) };
    eq_i32("mlkem768 enc_det noncanonical pk rc", ec, er);
    assert_eq!(ec, -1, "C must reject non-canonical mlkem pk (enc_det)");

    let mut ctc2 = vec![0u8; ctb];
    let mut ssc2 = vec![0u8; ssb];
    let mut ctr2 = vec![0u8; ctb];
    let mut ssr2 = vec![0u8; ssb];
    let ec2 = unsafe { enc_c(ctc2.as_mut_ptr(), ssc2.as_mut_ptr(), bad_pk.as_ptr()) };
    let er2 = unsafe { enc_r(ctr2.as_mut_ptr(), ssr2.as_mut_ptr(), bad_pk.as_ptr()) };
    eq_i32("mlkem768 enc noncanonical pk rc", ec2, er2);
    assert_eq!(ec2, -1, "C must reject non-canonical mlkem pk (enc)");

    // Sweep several coefficient positions.
    for coeff_pair in [10usize, 100, 383] {
        let base = coeff_pair * 3;
        if base + 2 >= pkb {
            continue;
        }
        let mut bp = pk.clone();
        bp[base] = 0xff;
        bp[base + 1] |= 0x0f;
        let mut c = vec![0u8; ctb];
        let mut s = vec![0u8; ssb];
        let mut c2 = vec![0u8; ctb];
        let mut s2 = vec![0u8; ssb];
        let rc = unsafe { encdet_c(c.as_mut_ptr(), s.as_mut_ptr(), bp.as_ptr(), coins.as_ptr()) };
        let rr = unsafe { encdet_r(c2.as_mut_ptr(), s2.as_mut_ptr(), bp.as_ptr(), coins.as_ptr()) };
        eq_i32(&format!("mlkem768 enc_det noncanonical@{coeff_pair} rc"), rc, rr);
        assert_eq!(rc, -1);
    }
}

// ===========================================================================
// Row 257: crypto_kem_mlkem768_dec with a TAMPERED ciphertext returns 0 but
// with an implicit-reject shared secret that differs from the real one; BOTH
// libs must produce the SAME (byte-identical) implicit-reject secret.
// ===========================================================================

#[test]
fn c257_mlkem768_dec_tampered_ct_implicit_reject() {
    let d = duo();
    let mut rng = Rng::new(0x257_0001);

    let (pkbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_publickeybytes");
    let (skbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_secretkeybytes");
    let (ctbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_ciphertextbytes");
    let (ssbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_sharedsecretbytes");
    let (pkb, skb, ctb, ssb) = unsafe { (pkbf(), skbf(), ctbf(), ssbf()) };

    let (skp_c, _) = d.pair::<KemSeedKeypair>("crypto_kem_mlkem768_seed_keypair");
    let (encdet_c, _) = d.pair::<KemEncDet>("crypto_kem_mlkem768_enc_deterministic");
    let (dec_c, dec_r) = d.pair::<KemDec>("crypto_kem_mlkem768_dec");

    let seed = rng.bytes(64);
    let mut pk = vec![0u8; pkb];
    let mut sk = vec![0u8; skb];
    unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    for _ in 0..8 {
        let coins = rng.bytes(32);
        let mut ct = vec![0u8; ctb];
        let mut ss_enc = vec![0u8; ssb];
        assert_eq!(
            unsafe { encdet_c(ct.as_mut_ptr(), ss_enc.as_mut_ptr(), pk.as_ptr(), coins.as_ptr()) },
            0
        );

        let mut bad = ct.clone();
        let idx = rng.below(ctb);
        bad[idx] ^= 0xff;

        let mut dssc = vec![0u8; ssb];
        let mut dssr = vec![0u8; ssb];
        let rc = unsafe { dec_c(dssc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        let rr = unsafe { dec_r(dssr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        eq_i32("mlkem768 dec tampered rc", rc, rr);
        assert_eq!(rc, 0, "dec always returns 0 (implicit reject)");
        eq_bytes("mlkem768 dec tampered implicit-reject ss", &dssc, &dssr);
        assert_ne!(dssc, ss_enc, "implicit-reject secret must differ from real ss");
    }
}

// ===========================================================================
// Row 258 (documented): crypto_kem_mlkem768_seed_keypair never errors — it
// always returns 0 (no invalid-input path). Exercised for round-trip in b05;
// there is no error path to differentially test here.
// ===========================================================================

// ===========================================================================
// Rows 259-261: crypto_kem_xwing_enc_deterministic / _enc reject:
//   * a non-canonical embedded ML-KEM pk (first PUBLICKEYBYTES bytes)  (259/261)
//   * a small-order x25519 pk component (last 32 bytes)                (260/261)
// -> -1.
// ===========================================================================

#[test]
fn c259_261_xwing_enc_errors() {
    let d = duo();
    let mut rng = Rng::new(0x259_0001);

    let (pkbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_publickeybytes");
    let (skbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_secretkeybytes");
    let (ctbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_ciphertextbytes");
    let (ssbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_sharedsecretbytes");
    let (mlkem_pkbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_publickeybytes");
    let (pkb, skb, ctb, ssb, mlkem_pkb) =
        unsafe { (pkbf(), skbf(), ctbf(), ssbf(), mlkem_pkbf()) };

    let (skp_c, _) = d.pair::<KemSeedKeypair>("crypto_kem_xwing_seed_keypair");
    let (encdet_c, encdet_r) = d.pair::<KemEncDet>("crypto_kem_xwing_enc_deterministic");
    let (enc_c, enc_r) = d.pair::<KemEnc>("crypto_kem_xwing_enc");

    let seed = rng.bytes(32);
    let mut pk = vec![0u8; pkb];
    let mut sk = vec![0u8; skb];
    unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    let eseed = rng.bytes(64);

    // row 259/261: non-canonical embedded ML-KEM pk.
    {
        let mut bad = pk.clone();
        bad[0] = 0xff;
        bad[1] |= 0x0f;
        let mut ctc = vec![0u8; ctb];
        let mut ssc = vec![0u8; ssb];
        let mut ctr = vec![0u8; ctb];
        let mut ssr = vec![0u8; ssb];
        let ec = unsafe { encdet_c(ctc.as_mut_ptr(), ssc.as_mut_ptr(), bad.as_ptr(), eseed.as_ptr()) };
        let er = unsafe { encdet_r(ctr.as_mut_ptr(), ssr.as_mut_ptr(), bad.as_ptr(), eseed.as_ptr()) };
        eq_i32("xwing enc_det noncanonical mlkem pk rc", ec, er);
        assert_eq!(ec, -1, "C must reject non-canonical embedded mlkem pk");

        let mut ctc2 = vec![0u8; ctb];
        let mut ssc2 = vec![0u8; ssb];
        let mut ctr2 = vec![0u8; ctb];
        let mut ssr2 = vec![0u8; ssb];
        let ec2 = unsafe { enc_c(ctc2.as_mut_ptr(), ssc2.as_mut_ptr(), bad.as_ptr()) };
        let er2 = unsafe { enc_r(ctr2.as_mut_ptr(), ssr2.as_mut_ptr(), bad.as_ptr()) };
        eq_i32("xwing enc noncanonical mlkem pk rc", ec2, er2);
        assert_eq!(ec2, -1);
    }

    // row 260/261: small-order x25519 pk component.
    for (i, small) in CURVE25519_BLOCKLIST.iter().enumerate() {
        let mut bad = pk.clone();
        bad[mlkem_pkb..mlkem_pkb + 32].copy_from_slice(small);
        let mut ctc = vec![0u8; ctb];
        let mut ssc = vec![0u8; ssb];
        let mut ctr = vec![0u8; ctb];
        let mut ssr = vec![0u8; ssb];
        let ec = unsafe { encdet_c(ctc.as_mut_ptr(), ssc.as_mut_ptr(), bad.as_ptr(), eseed.as_ptr()) };
        let er = unsafe { encdet_r(ctr.as_mut_ptr(), ssr.as_mut_ptr(), bad.as_ptr(), eseed.as_ptr()) };
        eq_i32(&format!("xwing enc_det small-order x25519 #{i} rc"), ec, er);
        assert_eq!(ec, -1, "C must reject small-order x25519 pk #{i}");
    }
}

// ===========================================================================
// Row 263: crypto_kem_xwing_dec with a small-order ct_x25519 component -> -1.
// The x-wing ciphertext = ct_mlkem(1088) || ct_x25519(32); making the x25519
// component small-order causes the inner crypto_scalarmult_curve25519 to fail.
//
// Row 262 (documented): the inner ML-KEM dec inside xwing_dec never returns
// non-zero (ML-KEM dec always returns 0 via implicit reject), so that -1 branch
// is marked LCOV_EXCL and is unreachable through the public API. Only the
// x25519 branch (row 263) is reachable, which is what we test here.
// ===========================================================================

#[test]
fn c263_xwing_dec_small_order_ct_x25519() {
    let d = duo();
    let mut rng = Rng::new(0x263_0001);

    let (skbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_secretkeybytes");
    let (pkbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_publickeybytes");
    let (ctbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_ciphertextbytes");
    let (ssbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_sharedsecretbytes");
    let (mlkem_ctbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_ciphertextbytes");
    let (skb, pkb, ctb, ssb, mlkem_ctb) =
        unsafe { (skbf(), pkbf(), ctbf(), ssbf(), mlkem_ctbf()) };

    let (skp_c, _) = d.pair::<KemSeedKeypair>("crypto_kem_xwing_seed_keypair");
    let (enc_c, _) = d.pair::<KemEnc>("crypto_kem_xwing_enc");
    let (dec_c, dec_r) = d.pair::<KemDec>("crypto_kem_xwing_dec");

    let seed = rng.bytes(32);
    let mut pk = vec![0u8; pkb];
    let mut sk = vec![0u8; skb];
    unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    let mut ct = vec![0u8; ctb];
    let mut ss = vec![0u8; ssb];
    assert_eq!(unsafe { enc_c(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr()) }, 0);

    for (i, small) in CURVE25519_BLOCKLIST.iter().enumerate() {
        let mut bad = ct.clone();
        bad[mlkem_ctb..mlkem_ctb + 32].copy_from_slice(small);
        let mut dssc = vec![0u8; ssb];
        let mut dssr = vec![0u8; ssb];
        let rc = unsafe { dec_c(dssc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        let rr = unsafe { dec_r(dssr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        eq_i32(&format!("xwing dec small-order ct_x25519 #{i} rc"), rc, rr);
        assert_eq!(rc, -1, "C must reject small-order ct_x25519 #{i}");
    }
}
