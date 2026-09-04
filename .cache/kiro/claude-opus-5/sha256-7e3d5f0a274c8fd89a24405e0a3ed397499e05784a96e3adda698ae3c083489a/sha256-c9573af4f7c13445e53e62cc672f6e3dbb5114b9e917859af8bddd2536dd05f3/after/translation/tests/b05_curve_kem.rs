//! Phase B — CONFIGS.md rows 94–119: the elliptic-curve core / scalarmult
//! surface plus key-exchange (kx) and KEMs (mlkem768, xwing, generic).
//!
//! Every function is exercised through BOTH shared libraries (C ground-truth
//! vs the Rust translation), loaded via `libloading`, and the outputs / return
//! codes are compared byte-for-byte. Many of these functions return -1 for
//! invalid inputs; we always compare the return code first and only compare the
//! output buffer when the C side returned 0.

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
// is_valid_point: int f(const unsigned char *p)
type Core1c = unsafe extern "C" fn(*const u8) -> i32;
// random: void f(unsigned char *p)
type CoreRnd = unsafe extern "C" fn(*mut u8);
// from_hash: int f(unsigned char *p, const unsigned char *r)
type CoreFromHash = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
// from_string: int f(out, ctx, ctx_len(size_t), msg, msg_len(size_t), hash_alg(int))
type CoreFromString = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, i32) -> i32;

// scalar ops that return int (invert)
type ScalarInvert = unsafe extern "C" fn(*mut u8, *const u8) -> i32;
// scalar ops void unary: negate/complement/reduce
type ScalarUnary = unsafe extern "C" fn(*mut u8, *const u8);
// scalar ops void binary: add/sub/mul
type ScalarBinary = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
// scalar_random: void f(unsigned char *r)  (randomized -> length only)
type ScalarRandom = unsafe extern "C" fn(*mut u8);

// kx keypair: int f(pk, sk)   seed_keypair: int f(pk, sk, seed)
type KxKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type KxSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
// session keys: int f(rx, tx, pk, sk, other_pk)
type KxSession = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> i32;

// kem seed_keypair: int f(pk, sk, seed)
type KemSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
// kem keypair: int f(pk, sk)
type KemKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
// kem enc: int f(ct, ss, pk)
type KemEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
// kem enc_deterministic: int f(ct, ss, pk, seed)
type KemEncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> i32;
// kem dec: int f(ss, ct, sk)
type KemDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;

// size accessor
type SizeFn = unsafe extern "C" fn() -> usize;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Group order L = 2^252 + 27742317777372353535851937790883648493, little-endian.
const L_LE: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

fn l_minus_1() -> [u8; 32] {
    let mut v = L_LE;
    v[0] -= 1; // 0xed -> 0xec, no borrow
    v
}

fn l_plus_1() -> [u8; 32] {
    let mut v = L_LE;
    v[0] += 1; // 0xed -> 0xee, no carry
    v
}

/// A corpus of 32-byte scalars: random, all-zero, L-1, L, L+1, all-0xff.
fn scalar32_corpus(rng: &mut Rng, n_random: usize) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    v.push([0u8; 32]);
    v.push(l_minus_1());
    v.push(L_LE);
    v.push(l_plus_1());
    v.push([0xffu8; 32]);
    for _ in 0..n_random {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        v.push(s);
    }
    v
}

/// A corpus of 64-byte (non-reduced) scalars for the _reduce path etc.
fn scalar64_corpus(rng: &mut Rng, n_random: usize) -> Vec<[u8; 64]> {
    let mut v: Vec<[u8; 64]> = Vec::new();
    v.push([0u8; 64]);
    v.push([0xffu8; 64]);
    {
        // L embedded in low 32 bytes, zero high.
        let mut s = [0u8; 64];
        s[..32].copy_from_slice(&L_LE);
        v.push(s);
    }
    for _ in 0..n_random {
        let mut s = [0u8; 64];
        rng.fill(&mut s);
        v.push(s);
    }
    v
}

/// Obtain a corpus of valid ed25519 point encodings using the library itself
/// (scalarmult_base of random scalars). Uses the C library so the points are
/// unambiguously valid ground-truth encodings.
fn valid_ed25519_points(d: &'static Duo, rng: &mut Rng, count: usize) -> Vec<[u8; 32]> {
    let (base_c, _) = d.pair::<Sm2>("crypto_scalarmult_ed25519_base");
    let mut pts = Vec::new();
    for _ in 0..count {
        // Non-zero scalar.
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        n[0] |= 1;
        let mut q = [0u8; 32];
        let rc = unsafe { base_c(q.as_mut_ptr(), n.as_ptr()) };
        if rc == 0 {
            pts.push(q);
        }
    }
    pts
}

/// Obtain valid ristretto255 point encodings via C _from_hash of random input.
fn valid_ristretto_points(d: &'static Duo, rng: &mut Rng, count: usize) -> Vec<[u8; 32]> {
    let (fh_c, _) = d.pair::<CoreFromHash>("crypto_core_ristretto255_from_hash");
    let mut pts = Vec::new();
    for _ in 0..count {
        let h = rng.bytes(64);
        let mut p = [0u8; 32];
        let rc = unsafe { fh_c(p.as_mut_ptr(), h.as_ptr()) };
        if rc == 0 {
            pts.push(p);
        }
    }
    pts
}

/// The eight canonical ed25519 small-order point encodings (from libsodium).
const ED25519_SMALL_ORDER: [[u8; 32]; 8] = [
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ],
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

const HASH_ALGS: [i32; 2] = [1, 2]; // SHA256, SHA512
const CTX_LENS: [usize; 4] = [0, 1, 32, 255];
const MSG_LENS: [usize; 4] = [0, 1, 64, 1000];

// ===========================================================================
// Row 94: generic crypto_scalarmult / crypto_scalarmult_base (curve25519)
// ===========================================================================

#[test]
fn r94_scalarmult_generic() {
    let d = duo();
    let mut rng = Rng::new(0x94_00_01);
    let (sm_c, sm_r) = d.pair::<Sm3>("crypto_scalarmult");
    let (base_c, base_r) = d.pair::<Sm2>("crypto_scalarmult_base");

    // scalarmult_base over many random scalars, then scalarmult(n, base_pt).
    for _ in 0..256 {
        let n = rng.bytes(32);
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        let rc = unsafe { base_c(pc.as_mut_ptr(), n.as_ptr()) };
        let rr = unsafe { base_r(pr.as_mut_ptr(), n.as_ptr()) };
        eq_i32("scalarmult_base rc", rc, rr);
        if rc == 0 {
            eq_bytes("scalarmult_base", &pc, &pr);
        }

        // Use the (valid) public point pc as the peer point.
        let n2 = rng.bytes(32);
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc2 = unsafe { sm_c(qc.as_mut_ptr(), n2.as_ptr(), pc.as_ptr()) };
        let rr2 = unsafe { sm_r(qr.as_mut_ptr(), n2.as_ptr(), pc.as_ptr()) };
        eq_i32("scalarmult rc", rc2, rr2);
        if rc2 == 0 {
            eq_bytes("scalarmult", &qc, &qr);
        }
    }

    // Edge scalars / points.
    for fill in [0x00u8, 0xff] {
        let n = vec![fill; 32];
        let p = vec![fill; 32];
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { sm_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { sm_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32(&format!("scalarmult edge fill={fill:#04x} rc"), rc, rr);
        if rc == 0 {
            eq_bytes(&format!("scalarmult edge fill={fill:#04x}"), &qc, &qr);
        }
    }
}

// ===========================================================================
// Rows 95-96: crypto_scalarmult_ed25519 (+base, clamped) and _noclamp variants
// ===========================================================================

#[test]
fn r95_96_scalarmult_ed25519() {
    let d = duo();
    let mut rng = Rng::new(0x95_00_01);

    let (b_c, b_r) = d.pair::<Sm2>("crypto_scalarmult_ed25519_base");
    let (bn_c, bn_r) = d.pair::<Sm2>("crypto_scalarmult_ed25519_base_noclamp");
    let (m_c, m_r) = d.pair::<Sm3>("crypto_scalarmult_ed25519");
    let (mn_c, mn_r) = d.pair::<Sm3>("crypto_scalarmult_ed25519_noclamp");

    let (red_c, _) = d.pair::<ScalarUnary>("crypto_core_ed25519_scalar_reduce");

    // Build a set of valid base points from _base.
    let mut points: Vec<[u8; 32]> = Vec::new();

    // Scalars: some raw random, some reduced mod L.
    for i in 0..200 {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        // Half of them reduced mod L (row 96: reduced and non-reduced).
        if i % 2 == 0 {
            let mut wide = [0u8; 64];
            wide[..32].copy_from_slice(&n);
            let mut r = [0u8; 32];
            unsafe { red_c(r.as_mut_ptr(), wide.as_ptr()) };
            n = r;
        }

        // clamped base
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        let rc = unsafe { b_c(pc.as_mut_ptr(), n.as_ptr()) };
        let rr = unsafe { b_r(pr.as_mut_ptr(), n.as_ptr()) };
        eq_i32("ed25519_base rc", rc, rr);
        if rc == 0 {
            eq_bytes("ed25519_base", &pc, &pr);
            points.push(pc);
        }

        // noclamp base
        let mut pnc = [0u8; 32];
        let mut pnr = [0u8; 32];
        let rc2 = unsafe { bn_c(pnc.as_mut_ptr(), n.as_ptr()) };
        let rr2 = unsafe { bn_r(pnr.as_mut_ptr(), n.as_ptr()) };
        eq_i32("ed25519_base_noclamp rc", rc2, rr2);
        if rc2 == 0 {
            eq_bytes("ed25519_base_noclamp", &pnc, &pnr);
            points.push(pnc);
        }
    }

    // Now scalarmult over valid points.
    for (i, p) in points.iter().enumerate() {
        let mut n = [0u8; 32];
        rng.fill(&mut n);
        if i % 3 == 0 {
            n[0] |= 1; // ensure non-trivial
        }

        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { m_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { m_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32("ed25519 scalarmult rc", rc, rr);
        if rc == 0 {
            eq_bytes("ed25519 scalarmult", &qc, &qr);
        }

        let mut qnc = [0u8; 32];
        let mut qnr = [0u8; 32];
        let rc2 = unsafe { mn_c(qnc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr2 = unsafe { mn_r(qnr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32("ed25519 scalarmult_noclamp rc", rc2, rr2);
        if rc2 == 0 {
            eq_bytes("ed25519 scalarmult_noclamp", &qnc, &qnr);
        }
    }

    // Edge: zero scalar (should fail), invalid point (all-0xff).
    for n in [[0u8; 32], [0xffu8; 32]] {
        let p = [0xffu8; 32];
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { m_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { m_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32("ed25519 scalarmult edge rc", rc, rr);
        if rc == 0 {
            eq_bytes("ed25519 scalarmult edge", &qc, &qr);
        }
    }
}

// ===========================================================================
// Row 97: crypto_scalarmult_ristretto255 / _base
// ===========================================================================

#[test]
fn r97_scalarmult_ristretto255() {
    let d = duo();
    let mut rng = Rng::new(0x97_00_01);

    let (b_c, b_r) = d.pair::<Sm2>("crypto_scalarmult_ristretto255_base");
    let (m_c, m_r) = d.pair::<Sm3>("crypto_scalarmult_ristretto255");

    let points = valid_ristretto_points(d, &mut rng, 48);

    for _ in 0..200 {
        let n = rng.bytes(32);
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        let rc = unsafe { b_c(pc.as_mut_ptr(), n.as_ptr()) };
        let rr = unsafe { b_r(pr.as_mut_ptr(), n.as_ptr()) };
        eq_i32("ristretto_base rc", rc, rr);
        if rc == 0 {
            eq_bytes("ristretto_base", &pc, &pr);
        }
    }

    for p in &points {
        let n = rng.bytes(32);
        let mut qc = [0u8; 32];
        let mut qr = [0u8; 32];
        let rc = unsafe { m_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { m_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eq_i32("ristretto scalarmult rc", rc, rr);
        if rc == 0 {
            eq_bytes("ristretto scalarmult", &qc, &qr);
        }
    }

    // Invalid point.
    let n = rng.bytes(32);
    let p = [0xffu8; 32];
    let mut qc = [0u8; 32];
    let mut qr = [0u8; 32];
    let rc = unsafe { m_c(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
    let rr = unsafe { m_r(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
    eq_i32("ristretto scalarmult invalid rc", rc, rr);
}

// ===========================================================================
// Rows 98-99: crypto_core_ed25519_add / _sub on valid point pairs, plus valid
// point generation (row 99 targets _from_uniform, which is not exported in this
// build — we generate valid points via scalarmult_base instead).
// ===========================================================================

#[test]
fn r98_99_ed25519_add_sub() {
    let d = duo();
    let mut rng = Rng::new(0x98_00_01);

    let (add_c, add_r) = d.pair::<Core3>("crypto_core_ed25519_add");
    let (sub_c, sub_r) = d.pair::<Core3>("crypto_core_ed25519_sub");

    let pts = valid_ed25519_points(d, &mut rng, 64);
    assert!(pts.len() >= 2, "need at least 2 valid ed25519 points");

    for i in 0..pts.len() {
        let p = &pts[i];
        let q = &pts[(i + 1) % pts.len()];

        let mut rc_buf = [0u8; 32];
        let mut rr_buf = [0u8; 32];
        let rc = unsafe { add_c(rc_buf.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let rr = unsafe { add_r(rr_buf.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        eq_i32("ed25519_add rc", rc, rr);
        if rc == 0 {
            eq_bytes("ed25519_add", &rc_buf, &rr_buf);
        }

        let mut sc_buf = [0u8; 32];
        let mut sr_buf = [0u8; 32];
        let sc = unsafe { sub_c(sc_buf.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let sr = unsafe { sub_r(sr_buf.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        eq_i32("ed25519_sub rc", sc, sr);
        if sc == 0 {
            eq_bytes("ed25519_sub", &sc_buf, &sr_buf);
        }
    }

    // add/sub with invalid points: all-zero, all-0xff.
    for bad in [[0u8; 32], [0xffu8; 32]] {
        let good = &pts[0];
        for (name_c, f_c, f_r) in [
            ("ed25519_add bad-p", add_c.clone(), add_r.clone()),
            ("ed25519_sub bad-p", sub_c.clone(), sub_r.clone()),
        ] {
            let mut oc = [0u8; 32];
            let mut or = [0u8; 32];
            let rc = unsafe { f_c(oc.as_mut_ptr(), bad.as_ptr(), good.as_ptr()) };
            let rr = unsafe { f_r(or.as_mut_ptr(), bad.as_ptr(), good.as_ptr()) };
            eq_i32(&format!("{name_c} rc"), rc, rr);
            if rc == 0 {
                eq_bytes(name_c, &oc, &or);
            }
        }
    }
}

// ===========================================================================
// Rows 100-101: crypto_core_ed25519_from_string / _from_string_nu
// (this build exports `_from_string` and `_from_string_nu`, not `_ro`).
// hash_alg {1,2} × ctx_len {0,1,32,255} × msg_len {0,1,64,1000}
// ===========================================================================

#[test]
fn r100_101_ed25519_from_string() {
    let d = duo();
    let mut rng = Rng::new(0x100_00_01);

    let mut fns: Vec<(&str, _, _)> = Vec::new();
    for name in [
        "crypto_core_ed25519_from_string",
        "crypto_core_ed25519_from_string_nu",
    ] {
        if d.has(name) {
            let (c, r) = d.pair::<CoreFromString>(name);
            fns.push((name, c, r));
        }
    }
    assert!(!fns.is_empty(), "no ed25519 from_string symbols present");

    for (name, c, r) in &fns {
        for &alg in &HASH_ALGS {
            for &cl in &CTX_LENS {
                for &ml in &MSG_LENS {
                    // A few randomized (ctx,msg) per config.
                    for _ in 0..3 {
                        let ctx = rng.bytes(cl);
                        let msg = rng.bytes(ml);
                        let cptr = if cl == 0 {
                            std::ptr::null()
                        } else {
                            ctx.as_ptr()
                        };
                        let mptr = if ml == 0 {
                            std::ptr::null()
                        } else {
                            msg.as_ptr()
                        };
                        let mut oc = [0u8; 32];
                        let mut or = [0u8; 32];
                        let rc = unsafe { c(oc.as_mut_ptr(), cptr, cl, mptr, ml, alg) };
                        let rr = unsafe { r(or.as_mut_ptr(), cptr, cl, mptr, ml, alg) };
                        eq_i32(&format!("{name} rc alg={alg} cl={cl} ml={ml}"), rc, rr);
                        if rc == 0 {
                            eq_bytes(&format!("{name} alg={alg} cl={cl} ml={ml}"), &oc, &or);
                        }
                    }
                }
            }
        }
        // Invalid hash_alg must fail identically.
        for bad_alg in [0i32, 3, -1] {
            let msg = rng.bytes(16);
            let mut oc = [0u8; 32];
            let mut or = [0u8; 32];
            let rc = unsafe {
                c(
                    oc.as_mut_ptr(),
                    std::ptr::null(),
                    0,
                    msg.as_ptr(),
                    16,
                    bad_alg,
                )
            };
            let rr = unsafe {
                r(
                    or.as_mut_ptr(),
                    std::ptr::null(),
                    0,
                    msg.as_ptr(),
                    16,
                    bad_alg,
                )
            };
            eq_i32(&format!("{name} bad_alg={bad_alg} rc"), rc, rr);
        }
    }
}

// ===========================================================================
// Row 102: crypto_core_ed25519_random (length only) and _is_valid_point on
// valid and hand-built invalid encodings.
// ===========================================================================

#[test]
fn r102_ed25519_random_and_valid_point() {
    let d = duo();
    let mut rng = Rng::new(0x102_00_01);

    // random: randomized output, so just exercise it and check it yields a
    // valid point in both libraries (length + validity, not equality).
    let (rnd_c, rnd_r) = d.pair::<CoreRnd>("crypto_core_ed25519_random");
    let (ivp_c, ivp_r) = d.pair::<Core1c>("crypto_core_ed25519_is_valid_point");

    for _ in 0..64 {
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        unsafe {
            rnd_c(pc.as_mut_ptr());
            rnd_r(pr.as_mut_ptr());
        }
        // Both outputs must be valid points per each library's own check.
        assert_eq!(unsafe { ivp_c(pc.as_ptr()) }, 1, "C random not valid point");
        assert_eq!(
            unsafe { ivp_r(pr.as_ptr()) },
            1,
            "Rust random not valid point"
        );
    }

    // Valid points from scalarmult_base -> is_valid_point must agree (== 1).
    let valid = valid_ed25519_points(d, &mut rng, 32);
    for p in &valid {
        let c = unsafe { ivp_c(p.as_ptr()) };
        let r = unsafe { ivp_r(p.as_ptr()) };
        eq_i32("is_valid_point(valid)", c, r);
    }

    // Invalid / edge encodings: all-zero, all-0xff, small-order, non-canonical.
    let mut invalid: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    invalid.extend_from_slice(&ED25519_SMALL_ORDER);
    // Non-canonical y (y >= p): y = 2^255-1 which is >= p, non-canonical.
    invalid.push({
        let mut v = [0xffu8; 32];
        v[31] = 0x7f;
        v
    });
    // A few random 32-byte blobs (mostly invalid encodings).
    for _ in 0..64 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        invalid.push(v);
    }
    for (i, p) in invalid.iter().enumerate() {
        let c = unsafe { ivp_c(p.as_ptr()) };
        let r = unsafe { ivp_r(p.as_ptr()) };
        eq_i32(&format!("is_valid_point(invalid#{i})"), c, r);
    }
}

// ===========================================================================
// Row 103: crypto_core_ed25519_scalar_* over the scalar corpus.
// ===========================================================================

#[test]
fn r103_ed25519_scalar_ops() {
    let d = duo();
    let mut rng = Rng::new(0x103_00_01);

    let (inv_c, inv_r) = d.pair::<ScalarInvert>("crypto_core_ed25519_scalar_invert");
    let (neg_c, neg_r) = d.pair::<ScalarUnary>("crypto_core_ed25519_scalar_negate");
    let (cmp_c, cmp_r) = d.pair::<ScalarUnary>("crypto_core_ed25519_scalar_complement");
    let (add_c, add_r) = d.pair::<ScalarBinary>("crypto_core_ed25519_scalar_add");
    let (sub_c, sub_r) = d.pair::<ScalarBinary>("crypto_core_ed25519_scalar_sub");
    let (mul_c, mul_r) = d.pair::<ScalarBinary>("crypto_core_ed25519_scalar_mul");
    let (red_c, red_r) = d.pair::<ScalarUnary>("crypto_core_ed25519_scalar_reduce");

    let s32 = scalar32_corpus(&mut rng, 40);
    let s64 = scalar64_corpus(&mut rng, 40);

    // invert / negate / complement over 32-byte scalars.
    for s in &s32 {
        let mut ic = [0u8; 32];
        let mut ir = [0u8; 32];
        let rc = unsafe { inv_c(ic.as_mut_ptr(), s.as_ptr()) };
        let rr = unsafe { inv_r(ir.as_mut_ptr(), s.as_ptr()) };
        eq_i32("scalar_invert rc", rc, rr);
        if rc == 0 {
            eq_bytes("scalar_invert", &ic, &ir);
        }

        let mut nc = [0u8; 32];
        let mut nr = [0u8; 32];
        unsafe {
            neg_c(nc.as_mut_ptr(), s.as_ptr());
            neg_r(nr.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("scalar_negate", &nc, &nr);

        let mut cc = [0u8; 32];
        let mut cr = [0u8; 32];
        unsafe {
            cmp_c(cc.as_mut_ptr(), s.as_ptr());
            cmp_r(cr.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("scalar_complement", &cc, &cr);
    }

    // add / sub / mul over pairs.
    for x in &s32 {
        for y in &s32 {
            for (name, f_c, f_r) in [
                ("scalar_add", add_c.clone(), add_r.clone()),
                ("scalar_sub", sub_c.clone(), sub_r.clone()),
                ("scalar_mul", mul_c.clone(), mul_r.clone()),
            ] {
                let mut oc = [0u8; 32];
                let mut or = [0u8; 32];
                unsafe {
                    f_c(oc.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    f_r(or.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                }
                eq_bytes(name, &oc, &or);
            }
        }
    }

    // reduce over 64-byte inputs.
    for s in &s64 {
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        unsafe {
            red_c(oc.as_mut_ptr(), s.as_ptr());
            red_r(or.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("scalar_reduce", &oc, &or);
    }

    // scalar_random: length + validity only (randomized).
    let (srnd_c, srnd_r) = d.pair::<ScalarRandom>("crypto_core_ed25519_scalar_random");
    for _ in 0..32 {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        unsafe {
            srnd_c(a.as_mut_ptr());
            srnd_r(b.as_mut_ptr());
        }
        // Non-degenerate: should not be all zero.
        assert_ne!(a, [0u8; 32], "C scalar_random all zero");
        assert_ne!(b, [0u8; 32], "Rust scalar_random all zero");
    }
}

// ===========================================================================
// Row 104: crypto_core_ed25519_scalar_from_string
// ===========================================================================

#[test]
fn r104_ed25519_scalar_from_string() {
    let d = duo();
    let mut rng = Rng::new(0x104_00_01);

    let (c, r) = d.pair::<CoreFromString>("crypto_core_ed25519_scalar_from_string");
    for &alg in &HASH_ALGS {
        for &cl in &CTX_LENS {
            for &ml in &MSG_LENS {
                for _ in 0..3 {
                    let ctx = rng.bytes(cl);
                    let msg = rng.bytes(ml);
                    let cptr = if cl == 0 {
                        std::ptr::null()
                    } else {
                        ctx.as_ptr()
                    };
                    let mptr = if ml == 0 {
                        std::ptr::null()
                    } else {
                        msg.as_ptr()
                    };
                    let mut oc = [0u8; 32];
                    let mut or = [0u8; 32];
                    let rc = unsafe { c(oc.as_mut_ptr(), cptr, cl, mptr, ml, alg) };
                    let rr = unsafe { r(or.as_mut_ptr(), cptr, cl, mptr, ml, alg) };
                    eq_i32(
                        &format!("scalar_from_string rc alg={alg} cl={cl} ml={ml}"),
                        rc,
                        rr,
                    );
                    if rc == 0 {
                        eq_bytes(
                            &format!("scalar_from_string alg={alg} cl={cl} ml={ml}"),
                            &oc,
                            &or,
                        );
                    }
                }
            }
        }
    }
    for bad_alg in [0i32, 3, -1] {
        let msg = rng.bytes(16);
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        let rc = unsafe {
            c(
                oc.as_mut_ptr(),
                std::ptr::null(),
                0,
                msg.as_ptr(),
                16,
                bad_alg,
            )
        };
        let rr = unsafe {
            r(
                or.as_mut_ptr(),
                std::ptr::null(),
                0,
                msg.as_ptr(),
                16,
                bad_alg,
            )
        };
        eq_i32(&format!("scalar_from_string bad_alg={bad_alg} rc"), rc, rr);
    }
}

// ===========================================================================
// Rows 105-108: crypto_core_ristretto255_add/_sub, _from_hash, _from_string,
// _random, _is_valid_point.
// ===========================================================================

#[test]
fn r105_108_ristretto255_core() {
    let d = duo();
    let mut rng = Rng::new(0x105_00_01);

    let (add_c, add_r) = d.pair::<Core3>("crypto_core_ristretto255_add");
    let (sub_c, sub_r) = d.pair::<Core3>("crypto_core_ristretto255_sub");
    let (fh_c, fh_r) = d.pair::<CoreFromHash>("crypto_core_ristretto255_from_hash");
    let (ivp_c, ivp_r) = d.pair::<Core1c>("crypto_core_ristretto255_is_valid_point");
    let (rnd_c, rnd_r) = d.pair::<CoreRnd>("crypto_core_ristretto255_random");

    // Row 106: from_hash over random 64-byte inputs (and edge fills).
    let mut valid: Vec<[u8; 32]> = Vec::new();
    let mut hashes: Vec<Vec<u8>> = (0..64).map(|_| rng.bytes(64)).collect();
    hashes.push(vec![0u8; 64]);
    hashes.push(vec![0xffu8; 64]);
    for h in &hashes {
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        let rc = unsafe { fh_c(pc.as_mut_ptr(), h.as_ptr()) };
        let rr = unsafe { fh_r(pr.as_mut_ptr(), h.as_ptr()) };
        eq_i32("ristretto from_hash rc", rc, rr);
        if rc == 0 {
            eq_bytes("ristretto from_hash", &pc, &pr);
            valid.push(pc);
        }
    }
    assert!(valid.len() >= 2, "need valid ristretto points");

    // Row 105: add / sub over valid point pairs.
    for i in 0..valid.len() {
        let p = &valid[i];
        let q = &valid[(i + 1) % valid.len()];
        let mut ac = [0u8; 32];
        let mut ar = [0u8; 32];
        let rc = unsafe { add_c(ac.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let rr = unsafe { add_r(ar.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        eq_i32("ristretto_add rc", rc, rr);
        if rc == 0 {
            eq_bytes("ristretto_add", &ac, &ar);
        }
        let mut sc = [0u8; 32];
        let mut sr = [0u8; 32];
        let rcs = unsafe { sub_c(sc.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let rrs = unsafe { sub_r(sr.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        eq_i32("ristretto_sub rc", rcs, rrs);
        if rcs == 0 {
            eq_bytes("ristretto_sub", &sc, &sr);
        }
    }

    // add/sub with invalid inputs.
    for bad in [[0u8; 32], [0xffu8; 32]] {
        let good = &valid[0];
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        let rc = unsafe { add_c(oc.as_mut_ptr(), bad.as_ptr(), good.as_ptr()) };
        let rr = unsafe { add_r(or.as_mut_ptr(), bad.as_ptr(), good.as_ptr()) };
        eq_i32("ristretto_add invalid rc", rc, rr);
        if rc == 0 {
            eq_bytes("ristretto_add invalid", &oc, &or);
        }
    }

    // Row 107: from_string (only `_from_string` exported in this build).
    if d.has("crypto_core_ristretto255_from_string") {
        let (fs_c, fs_r) = d.pair::<CoreFromString>("crypto_core_ristretto255_from_string");
        for &alg in &HASH_ALGS {
            for &cl in &CTX_LENS {
                for &ml in &MSG_LENS {
                    for _ in 0..2 {
                        let ctx = rng.bytes(cl);
                        let msg = rng.bytes(ml);
                        let cptr = if cl == 0 {
                            std::ptr::null()
                        } else {
                            ctx.as_ptr()
                        };
                        let mptr = if ml == 0 {
                            std::ptr::null()
                        } else {
                            msg.as_ptr()
                        };
                        let mut oc = [0u8; 32];
                        let mut or = [0u8; 32];
                        let rc = unsafe { fs_c(oc.as_mut_ptr(), cptr, cl, mptr, ml, alg) };
                        let rr = unsafe { fs_r(or.as_mut_ptr(), cptr, cl, mptr, ml, alg) };
                        eq_i32(
                            &format!("ristretto from_string rc alg={alg} cl={cl} ml={ml}"),
                            rc,
                            rr,
                        );
                        if rc == 0 {
                            eq_bytes(
                                &format!("ristretto from_string alg={alg} cl={cl} ml={ml}"),
                                &oc,
                                &or,
                            );
                        }
                    }
                }
            }
        }
    }

    // Row 108: random (validity only) + is_valid_point.
    for _ in 0..64 {
        let mut pc = [0u8; 32];
        let mut pr = [0u8; 32];
        unsafe {
            rnd_c(pc.as_mut_ptr());
            rnd_r(pr.as_mut_ptr());
        }
        assert_eq!(
            unsafe { ivp_c(pc.as_ptr()) },
            1,
            "C ristretto random not valid"
        );
        assert_eq!(
            unsafe { ivp_r(pr.as_ptr()) },
            1,
            "Rust ristretto random not valid"
        );
    }
    for p in &valid {
        eq_i32(
            "ristretto is_valid(valid)",
            unsafe { ivp_c(p.as_ptr()) },
            unsafe { ivp_r(p.as_ptr()) },
        );
    }
    // Invalid encodings.
    let mut invalid: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    invalid.push({
        // non-canonical field element (top bit set)
        let mut v = [0u8; 32];
        v[31] = 0x80;
        v
    });
    for _ in 0..64 {
        let mut v = [0u8; 32];
        rng.fill(&mut v);
        invalid.push(v);
    }
    for (i, p) in invalid.iter().enumerate() {
        eq_i32(
            &format!("ristretto is_valid(invalid#{i})"),
            unsafe { ivp_c(p.as_ptr()) },
            unsafe { ivp_r(p.as_ptr()) },
        );
    }
}

// ===========================================================================
// Row 109: crypto_core_ristretto255_scalar_* over the same corpus as row 103.
// ===========================================================================

#[test]
fn r109_ristretto255_scalar_ops() {
    let d = duo();
    let mut rng = Rng::new(0x109_00_01);

    let (inv_c, inv_r) = d.pair::<ScalarInvert>("crypto_core_ristretto255_scalar_invert");
    let (neg_c, neg_r) = d.pair::<ScalarUnary>("crypto_core_ristretto255_scalar_negate");
    let (cmp_c, cmp_r) = d.pair::<ScalarUnary>("crypto_core_ristretto255_scalar_complement");
    let (add_c, add_r) = d.pair::<ScalarBinary>("crypto_core_ristretto255_scalar_add");
    let (sub_c, sub_r) = d.pair::<ScalarBinary>("crypto_core_ristretto255_scalar_sub");
    let (mul_c, mul_r) = d.pair::<ScalarBinary>("crypto_core_ristretto255_scalar_mul");
    let (red_c, red_r) = d.pair::<ScalarUnary>("crypto_core_ristretto255_scalar_reduce");

    let s32 = scalar32_corpus(&mut rng, 40);
    let s64 = scalar64_corpus(&mut rng, 40);

    for s in &s32 {
        let mut ic = [0u8; 32];
        let mut ir = [0u8; 32];
        let rc = unsafe { inv_c(ic.as_mut_ptr(), s.as_ptr()) };
        let rr = unsafe { inv_r(ir.as_mut_ptr(), s.as_ptr()) };
        eq_i32("r_scalar_invert rc", rc, rr);
        if rc == 0 {
            eq_bytes("r_scalar_invert", &ic, &ir);
        }
        let mut nc = [0u8; 32];
        let mut nr = [0u8; 32];
        unsafe {
            neg_c(nc.as_mut_ptr(), s.as_ptr());
            neg_r(nr.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("r_scalar_negate", &nc, &nr);
        let mut cc = [0u8; 32];
        let mut cr = [0u8; 32];
        unsafe {
            cmp_c(cc.as_mut_ptr(), s.as_ptr());
            cmp_r(cr.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("r_scalar_complement", &cc, &cr);
    }

    for x in &s32 {
        for y in &s32 {
            for (name, f_c, f_r) in [
                ("r_scalar_add", add_c.clone(), add_r.clone()),
                ("r_scalar_sub", sub_c.clone(), sub_r.clone()),
                ("r_scalar_mul", mul_c.clone(), mul_r.clone()),
            ] {
                let mut oc = [0u8; 32];
                let mut or = [0u8; 32];
                unsafe {
                    f_c(oc.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                    f_r(or.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                }
                eq_bytes(name, &oc, &or);
            }
        }
    }

    for s in &s64 {
        let mut oc = [0u8; 32];
        let mut or = [0u8; 32];
        unsafe {
            red_c(oc.as_mut_ptr(), s.as_ptr());
            red_r(or.as_mut_ptr(), s.as_ptr());
        }
        eq_bytes("r_scalar_reduce", &oc, &or);
    }

    let (srnd_c, srnd_r) = d.pair::<ScalarRandom>("crypto_core_ristretto255_scalar_random");
    for _ in 0..32 {
        let mut a = [0u8; 32];
        let mut b = [0u8; 32];
        unsafe {
            srnd_c(a.as_mut_ptr());
            srnd_r(b.as_mut_ptr());
        }
        assert_ne!(a, [0u8; 32], "C r_scalar_random all zero");
        assert_ne!(b, [0u8; 32], "Rust r_scalar_random all zero");
    }
}

// ===========================================================================
// Row 110: crypto_kx_keypair (length/validity) and _seed_keypair (fixed seeds).
// ===========================================================================

#[test]
fn r110_kx_keypair() {
    let d = duo();
    let mut rng = Rng::new(0x110_00_01);

    let (skp_c, skp_r) = d.pair::<KxSeedKeypair>("crypto_kx_seed_keypair");
    let (kp_c, kp_r) = d.pair::<KxKeypair>("crypto_kx_keypair");

    // seed_keypair: fixed seeds -> deterministic, must match byte-for-byte.
    let mut seeds: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    for _ in 0..32 {
        let mut s = [0u8; 32];
        rng.fill(&mut s);
        seeds.push(s);
    }
    for seed in &seeds {
        let mut pkc = [0u8; 32];
        let mut skc = [0u8; 32];
        let mut pkr = [0u8; 32];
        let mut skr = [0u8; 32];
        let rc = unsafe { skp_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { skp_r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        eq_i32("kx_seed_keypair rc", rc, rr);
        eq_bytes("kx_seed_keypair pk", &pkc, &pkr);
        eq_bytes("kx_seed_keypair sk", &skc, &skr);
    }

    // keypair: randomized -> validity/length only; sk must not be all-zero.
    for _ in 0..16 {
        let mut pkc = [0u8; 32];
        let mut skc = [0u8; 32];
        let mut pkr = [0u8; 32];
        let mut skr = [0u8; 32];
        let rc = unsafe { kp_c(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
        let rr = unsafe { kp_r(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
        eq_i32("kx_keypair rc", rc, rr);
        assert_ne!(skc, [0u8; 32], "C kx_keypair sk zero");
        assert_ne!(skr, [0u8; 32], "Rust kx_keypair sk zero");
    }
}

// ===========================================================================
// Rows 111-112: crypto_kx_client_session_keys / _server_session_keys, with rx
// and tx both set, AND with tx == NULL (aliases tx to rx inside the C).
// ===========================================================================

#[test]
fn r111_112_kx_session_keys() {
    let d = duo();
    let mut rng = Rng::new(0x111_00_01);

    let (skp_c, _) = d.pair::<KxSeedKeypair>("crypto_kx_seed_keypair");
    let (cli_c, cli_r) = d.pair::<KxSession>("crypto_kx_client_session_keys");
    let (srv_c, srv_r) = d.pair::<KxSession>("crypto_kx_server_session_keys");

    // Build client + server keypairs from fixed seeds (via C, ground truth).
    let make_kp = |seed: &[u8; 32]| {
        let mut pk = [0u8; 32];
        let mut sk = [0u8; 32];
        let rc = unsafe { skp_c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
        assert_eq!(rc, 0);
        (pk, sk)
    };

    for _ in 0..24 {
        let mut cseed = [0u8; 32];
        let mut sseed = [0u8; 32];
        rng.fill(&mut cseed);
        rng.fill(&mut sseed);
        let (cpk, csk) = make_kp(&cseed);
        let (spk, ssk) = make_kp(&sseed);

        // ---- both rx and tx set ----
        for (name, f_c, f_r, pk, sk, other) in [
            ("kx_client", cli_c.clone(), cli_r.clone(), &cpk, &csk, &spk),
            ("kx_server", srv_c.clone(), srv_r.clone(), &spk, &ssk, &cpk),
        ] {
            let mut rxc = [0u8; 32];
            let mut txc = [0u8; 32];
            let mut rxr = [0u8; 32];
            let mut txr = [0u8; 32];
            let rc = unsafe {
                f_c(
                    rxc.as_mut_ptr(),
                    txc.as_mut_ptr(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    other.as_ptr(),
                )
            };
            let rr = unsafe {
                f_r(
                    rxr.as_mut_ptr(),
                    txr.as_mut_ptr(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    other.as_ptr(),
                )
            };
            eq_i32(&format!("{name} rc"), rc, rr);
            if rc == 0 {
                eq_bytes(&format!("{name} rx"), &rxc, &rxr);
                eq_bytes(&format!("{name} tx"), &txc, &txr);
            }

            // ---- tx == NULL (aliases tx to rx) ----
            let mut rxc2 = [0u8; 32];
            let mut rxr2 = [0u8; 32];
            let rc2 = unsafe {
                f_c(
                    rxc2.as_mut_ptr(),
                    std::ptr::null_mut(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    other.as_ptr(),
                )
            };
            let rr2 = unsafe {
                f_r(
                    rxr2.as_mut_ptr(),
                    std::ptr::null_mut(),
                    pk.as_ptr(),
                    sk.as_ptr(),
                    other.as_ptr(),
                )
            };
            eq_i32(&format!("{name} tx-null rc"), rc2, rr2);
            if rc2 == 0 {
                eq_bytes(&format!("{name} tx-null rx"), &rxc2, &rxr2);
            }
        }
    }

    // Invalid peer key (all-zero pk) should fail identically for client.
    let (_cpk, csk) = make_kp(&[7u8; 32]);
    let bad_peer = [0u8; 32];
    let mut rxc = [0u8; 32];
    let mut txc = [0u8; 32];
    let mut rxr = [0u8; 32];
    let mut txr = [0u8; 32];
    let my_pk = [1u8; 32];
    let rc = unsafe {
        cli_c(
            rxc.as_mut_ptr(),
            txc.as_mut_ptr(),
            my_pk.as_ptr(),
            csk.as_ptr(),
            bad_peer.as_ptr(),
        )
    };
    let rr = unsafe {
        cli_r(
            rxr.as_mut_ptr(),
            txr.as_mut_ptr(),
            my_pk.as_ptr(),
            csk.as_ptr(),
            bad_peer.as_ptr(),
        )
    };
    eq_i32("kx_client bad-peer rc", rc, rr);
}

// ===========================================================================
// Rows 113-115: crypto_kem_mlkem768_*
// ===========================================================================

#[test]
fn r113_115_kem_mlkem768() {
    let d = duo();
    let mut rng = Rng::new(0x113_00_01);

    let (pkbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_publickeybytes");
    let (skbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_secretkeybytes");
    let (ctbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_ciphertextbytes");
    let (ssbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_sharedsecretbytes");
    let (sdbf, _) = d.pair::<SizeFn>("crypto_kem_mlkem768_seedbytes");
    let (pkb, skb, ctb, ssb, sdb) = unsafe { (pkbf(), skbf(), ctbf(), ssbf(), sdbf()) };
    assert_eq!((pkb, skb, ctb, ssb, sdb), (1184, 2400, 1088, 32, 64));

    let (skp_c, skp_r) = d.pair::<KemSeedKeypair>("crypto_kem_mlkem768_seed_keypair");
    let (encdet_c, encdet_r) = d.pair::<KemEncDet>("crypto_kem_mlkem768_enc_deterministic");
    let (enc_c, enc_r) = d.pair::<KemEnc>("crypto_kem_mlkem768_enc");
    let (dec_c, dec_r) = d.pair::<KemDec>("crypto_kem_mlkem768_dec");

    // Row 113: seed_keypair with fixed 64-byte seeds incl all-zero/all-0xff.
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 64], vec![0xffu8; 64]];
    for _ in 0..8 {
        seeds.push(rng.bytes(64));
    }

    for seed in &seeds {
        let mut pkc = vec![0u8; pkb];
        let mut skc = vec![0u8; skb];
        let mut pkr = vec![0u8; pkb];
        let mut skr = vec![0u8; skb];
        let rc = unsafe { skp_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { skp_r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        eq_i32("mlkem768_seed_keypair rc", rc, rr);
        eq_bytes("mlkem768_seed_keypair pk", &pkc, &pkr);
        eq_bytes("mlkem768_seed_keypair sk", &skc, &skr);

        // Row 114: enc_deterministic with fixed 32-byte coins + dec.
        for _ in 0..4 {
            let coins = rng.bytes(32);
            let mut ctc = vec![0u8; ctb];
            let mut ssc = vec![0u8; ssb];
            let mut ctr = vec![0u8; ctb];
            let mut ssr = vec![0u8; ssb];
            let ec = unsafe {
                encdet_c(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pkc.as_ptr(),
                    coins.as_ptr(),
                )
            };
            let er = unsafe {
                encdet_r(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pkr.as_ptr(),
                    coins.as_ptr(),
                )
            };
            eq_i32("mlkem768_enc_det rc", ec, er);
            if ec == 0 {
                eq_bytes("mlkem768_enc_det ct", &ctc, &ctr);
                eq_bytes("mlkem768_enc_det ss", &ssc, &ssr);

                // dec on both libraries with the produced ct.
                let mut dssc = vec![0u8; ssb];
                let mut dssr = vec![0u8; ssb];
                let dc = unsafe { dec_c(dssc.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
                let dr = unsafe { dec_r(dssr.as_mut_ptr(), ctr.as_ptr(), skr.as_ptr()) };
                eq_i32("mlkem768_dec rc", dc, dr);
                if dc == 0 {
                    eq_bytes("mlkem768_dec ss", &dssc, &dssr);
                    // Round-trip: decapsulated ss == encapsulated ss.
                    eq_bytes("mlkem768 roundtrip det (C)", &ssc, &dssc);
                }
            }
        }
    }

    // Row 115: enc + dec round-trip agreement (randomized enc). Because enc
    // draws internal randomness we cannot compare ct/ss across libraries, but we
    // verify round-trip and cross-library decapsulation. Keys are generated
    // deterministically so pk/sk are identical across libs.
    let seed = rng.bytes(64);
    let mut pkc = vec![0u8; pkb];
    let mut skc = vec![0u8; skb];
    unsafe {
        skp_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
    }
    for _ in 0..8 {
        // C encapsulates.
        let mut ctc = vec![0u8; ctb];
        let mut ssc = vec![0u8; ssb];
        let ec = unsafe { enc_c(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pkc.as_ptr()) };
        assert_eq!(ec, 0);
        // Both libs decapsulate that ct and must agree with the enc ss.
        let mut d1 = vec![0u8; ssb];
        let mut d2 = vec![0u8; ssb];
        let r1 = unsafe { dec_c(d1.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
        let r2 = unsafe { dec_r(d2.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
        eq_i32("mlkem768 dec(enc) rc", r1, r2);
        eq_bytes("mlkem768 dec(enc) ss C-vs-R", &d1, &d2);
        eq_bytes("mlkem768 roundtrip enc/dec", &ssc, &d1);

        // Rust encapsulates; C must decapsulate to the same ss.
        let mut ctr = vec![0u8; ctb];
        let mut ssr = vec![0u8; ssb];
        let er = unsafe { enc_r(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pkc.as_ptr()) };
        assert_eq!(er, 0);
        let mut d3 = vec![0u8; ssb];
        let r3 = unsafe { dec_c(d3.as_mut_ptr(), ctr.as_ptr(), skc.as_ptr()) };
        assert_eq!(r3, 0);
        eq_bytes("mlkem768 roundtrip Rust-enc/C-dec", &ssr, &d3);
    }
}

// ===========================================================================
// Rows 116-118: crypto_kem_xwing_*
// ===========================================================================

#[test]
fn r116_118_kem_xwing() {
    let d = duo();
    let mut rng = Rng::new(0x116_00_01);

    let (pkbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_publickeybytes");
    let (skbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_secretkeybytes");
    let (ctbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_ciphertextbytes");
    let (ssbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_sharedsecretbytes");
    let (sdbf, _) = d.pair::<SizeFn>("crypto_kem_xwing_seedbytes");
    let (pkb, skb, ctb, ssb, sdb) = unsafe { (pkbf(), skbf(), ctbf(), ssbf(), sdbf()) };
    assert_eq!((pkb, skb, ctb, ssb, sdb), (1216, 32, 1120, 32, 32));

    let (skp_c, skp_r) = d.pair::<KemSeedKeypair>("crypto_kem_xwing_seed_keypair");
    let (encdet_c, encdet_r) = d.pair::<KemEncDet>("crypto_kem_xwing_enc_deterministic");
    let (enc_c, enc_r) = d.pair::<KemEnc>("crypto_kem_xwing_enc");
    let (dec_c, dec_r) = d.pair::<KemDec>("crypto_kem_xwing_dec");

    // Row 116: seed_keypair with fixed 32-byte seeds.
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; 32], vec![0xffu8; 32]];
    for _ in 0..8 {
        seeds.push(rng.bytes(32));
    }

    for seed in &seeds {
        let mut pkc = vec![0u8; pkb];
        let mut skc = vec![0u8; skb];
        let mut pkr = vec![0u8; pkb];
        let mut skr = vec![0u8; skb];
        let rc = unsafe { skp_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { skp_r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        eq_i32("xwing_seed_keypair rc", rc, rr);
        eq_bytes("xwing_seed_keypair pk", &pkc, &pkr);
        eq_bytes("xwing_seed_keypair sk", &skc, &skr);

        // Row 117: enc_deterministic with fixed 64-byte seeds + dec.
        for _ in 0..4 {
            let eseed = rng.bytes(64);
            let mut ctc = vec![0u8; ctb];
            let mut ssc = vec![0u8; ssb];
            let mut ctr = vec![0u8; ctb];
            let mut ssr = vec![0u8; ssb];
            let ec = unsafe {
                encdet_c(
                    ctc.as_mut_ptr(),
                    ssc.as_mut_ptr(),
                    pkc.as_ptr(),
                    eseed.as_ptr(),
                )
            };
            let er = unsafe {
                encdet_r(
                    ctr.as_mut_ptr(),
                    ssr.as_mut_ptr(),
                    pkr.as_ptr(),
                    eseed.as_ptr(),
                )
            };
            eq_i32("xwing_enc_det rc", ec, er);
            if ec == 0 {
                eq_bytes("xwing_enc_det ct", &ctc, &ctr);
                eq_bytes("xwing_enc_det ss", &ssc, &ssr);

                let mut dssc = vec![0u8; ssb];
                let mut dssr = vec![0u8; ssb];
                let dc = unsafe { dec_c(dssc.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
                let dr = unsafe { dec_r(dssr.as_mut_ptr(), ctr.as_ptr(), skr.as_ptr()) };
                eq_i32("xwing_dec rc", dc, dr);
                if dc == 0 {
                    eq_bytes("xwing_dec ss", &dssc, &dssr);
                    eq_bytes("xwing roundtrip det", &ssc, &dssc);
                }
            }
        }
    }

    // Row 118: enc + dec round-trip agreement (randomized enc).
    let seed = rng.bytes(32);
    let mut pkc = vec![0u8; pkb];
    let mut skc = vec![0u8; skb];
    unsafe {
        skp_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
    }
    for _ in 0..8 {
        let mut ctc = vec![0u8; ctb];
        let mut ssc = vec![0u8; ssb];
        let ec = unsafe { enc_c(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pkc.as_ptr()) };
        assert_eq!(ec, 0);
        let mut d1 = vec![0u8; ssb];
        let mut d2 = vec![0u8; ssb];
        let r1 = unsafe { dec_c(d1.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
        let r2 = unsafe { dec_r(d2.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
        eq_i32("xwing dec(enc) rc", r1, r2);
        eq_bytes("xwing dec(enc) ss C-vs-R", &d1, &d2);
        eq_bytes("xwing roundtrip enc/dec", &ssc, &d1);

        let mut ctr = vec![0u8; ctb];
        let mut ssr = vec![0u8; ssb];
        let er = unsafe { enc_r(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pkc.as_ptr()) };
        assert_eq!(er, 0);
        let mut d3 = vec![0u8; ssb];
        let r3 = unsafe { dec_c(d3.as_mut_ptr(), ctr.as_ptr(), skc.as_ptr()) };
        assert_eq!(r3, 0);
        eq_bytes("xwing roundtrip Rust-enc/C-dec", &ssr, &d3);
    }
}

// ===========================================================================
// Row 119: generic crypto_kem_* wrappers (xwing primitive), round-trip +
// constants.
// ===========================================================================

#[test]
fn r119_kem_generic() {
    let d = duo();
    let mut rng = Rng::new(0x119_00_01);

    // Constants must match C.
    for name in [
        "crypto_kem_publickeybytes",
        "crypto_kem_secretkeybytes",
        "crypto_kem_ciphertextbytes",
        "crypto_kem_sharedsecretbytes",
        "crypto_kem_seedbytes",
    ] {
        let (cf, rf) = d.pair::<SizeFn>(name);
        let c = unsafe { cf() };
        let r = unsafe { rf() };
        assert_eq!(c, r, "{name}: C {c} != Rust {r}");
    }

    let (pkbf, _) = d.pair::<SizeFn>("crypto_kem_publickeybytes");
    let (skbf, _) = d.pair::<SizeFn>("crypto_kem_secretkeybytes");
    let (ctbf, _) = d.pair::<SizeFn>("crypto_kem_ciphertextbytes");
    let (ssbf, _) = d.pair::<SizeFn>("crypto_kem_sharedsecretbytes");
    let (sdbf, _) = d.pair::<SizeFn>("crypto_kem_seedbytes");
    let (pkb, skb, ctb, ssb, sdb) = unsafe { (pkbf(), skbf(), ctbf(), ssbf(), sdbf()) };
    // Generic == xwing.
    assert_eq!((pkb, skb, ctb, ssb, sdb), (1216, 32, 1120, 32, 32));

    let (skp_c, skp_r) = d.pair::<KemSeedKeypair>("crypto_kem_seed_keypair");
    let (kp_c, _kp_r) = d.pair::<KemKeypair>("crypto_kem_keypair");
    let (enc_c, _enc_r) = d.pair::<KemEnc>("crypto_kem_enc");
    let (dec_c, dec_r) = d.pair::<KemDec>("crypto_kem_dec");

    // Deterministic seed_keypair must match byte-for-byte.
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; sdb], vec![0xffu8; sdb]];
    for _ in 0..6 {
        seeds.push(rng.bytes(sdb));
    }
    for seed in &seeds {
        let mut pkc = vec![0u8; pkb];
        let mut skc = vec![0u8; skb];
        let mut pkr = vec![0u8; pkb];
        let mut skr = vec![0u8; skb];
        let rc = unsafe { skp_c(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
        let rr = unsafe { skp_r(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
        eq_i32("kem_seed_keypair rc", rc, rr);
        eq_bytes("kem_seed_keypair pk", &pkc, &pkr);
        eq_bytes("kem_seed_keypair sk", &skc, &skr);

        // Round-trip: enc in C, dec in both must agree with ss.
        let mut ctc = vec![0u8; ctb];
        let mut ssc = vec![0u8; ssb];
        let ec = unsafe { enc_c(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pkc.as_ptr()) };
        assert_eq!(ec, 0);
        let mut d1 = vec![0u8; ssb];
        let mut d2 = vec![0u8; ssb];
        let r1 = unsafe { dec_c(d1.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
        let r2 = unsafe { dec_r(d2.as_mut_ptr(), ctc.as_ptr(), skr.as_ptr()) };
        eq_i32("kem_dec rc", r1, r2);
        eq_bytes("kem_dec ss C-vs-R", &d1, &d2);
        eq_bytes("kem roundtrip", &ssc, &d1);
    }

    // keypair (randomized) then round-trip within the C library.
    for _ in 0..4 {
        let mut pkc = vec![0u8; pkb];
        let mut skc = vec![0u8; skb];
        let rc = unsafe { kp_c(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
        eq_i32("kem_keypair rc", rc, 0);
        let mut ctc = vec![0u8; ctb];
        let mut ssc = vec![0u8; ssb];
        let ec = unsafe { enc_c(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pkc.as_ptr()) };
        assert_eq!(ec, 0);
        let mut dss = vec![0u8; ssb];
        let dc = unsafe { dec_c(dss.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr()) };
        assert_eq!(dc, 0);
        eq_bytes("kem generic keypair roundtrip", &ssc, &dss);
    }
}
