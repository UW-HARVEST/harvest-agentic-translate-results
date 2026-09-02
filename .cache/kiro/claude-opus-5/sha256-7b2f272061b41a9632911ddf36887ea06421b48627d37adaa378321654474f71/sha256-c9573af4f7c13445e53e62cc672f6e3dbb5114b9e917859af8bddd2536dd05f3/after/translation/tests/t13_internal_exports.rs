//! Differential tests for libsodium's INTERNAL-but-EXPORTED ABI surface.
//!
//! Both the C reference `.so` and the Rust `.so` export ~130 underscore-prefixed
//! symbols (`_sodium_*`, `_crypto_*`) plus several `*_implementation` data
//! globals.  They appear in `nm -D` and are therefore part of the public ABI,
//! yet none of the other `tNN_*` suites touch them.  This file loads BOTH
//! libraries via `libloading` and compares them.
//!
//! The underscore-prefixed names come from the `private/quirks.h` renaming
//! macros (e.g. `#define ge25519_p3_tobytes _sodium_ge25519_p3_tobytes`), so
//! the real C signature for `_sodium_X` is the declaration of `X` in the
//! matching private header.  For opaque field-element / group-element types we
//! allocate an oversized zeroed buffer and compare the WHOLE buffer, so any
//! internal-representation divergence is caught.
//!
//! CRITICAL: if C and Rust genuinely disagree, C is correct.  Assertions are
//! never weakened; a real disagreement is reported by a hard panic.

mod harness;
use harness::*;

use std::ffi::{c_int, c_void};
use std::ptr;

const SEED: u64 = 0x5EED_0013;

// Oversized opaque buffers (see priority 1 in the task).  A ge25519 point is
// 4 * fe25519; with the 25.5 representation fe25519 = int32_t[10] = 40 bytes so
// a p3 point is 160 bytes, and with the 64-bit representation fe25519 =
// uint64_t[5] = 40 bytes so a p3 point is also 160 bytes.  256 / 64 are safe
// over-allocations for point / field element respectively.  Both libraries
// share the SAME ABI, so whichever representation is compiled in must match on
// both sides, and comparing the whole buffer detects any mismatch.
const PT: usize = 256; // ge25519 point (p2/p3/p1p1) scratch
const FE: usize = 64; // fe25519 scratch

// ===========================================================================
// Priority 1a — fe25519 field-element helpers
//   void fe25519_frombytes(fe25519 h, const unsigned char *s);   // s: 32 bytes
//   void fe25519_tobytes(unsigned char *s, const fe25519 h);      // s: 32 bytes
//   void fe25519_invert(fe25519 out, const fe25519 z);
// The fe25519 type is opaque here; we round-trip through bytes and also compare
// the WHOLE opaque fe buffer produced by frombytes/invert.
// ===========================================================================

type FeFromBytes = unsafe extern "C" fn(*mut u8, *const u8);
type FeToBytes = unsafe extern "C" fn(*mut u8, *const u8);
type FeInvert = unsafe extern "C" fn(*mut u8, *const u8);

#[test]
fn fe25519_roundtrip_and_invert() {
    let (c_from, r_from) = sym::<FeFromBytes>("_sodium_fe25519_frombytes");
    let (c_to, r_to) = sym::<FeToBytes>("_sodium_fe25519_tobytes");
    let (c_inv, r_inv) = sym::<FeInvert>("_sodium_fe25519_invert");

    let mut rng = Rng::new(SEED);
    for iter in 0..2000 {
        let s = rng.bytes(32);

        // frombytes: compare the whole opaque fe buffer.
        let mut fc = out_buf(FE);
        let mut fr = out_buf(FE);
        unsafe {
            c_from(fc.as_mut_ptr(), s.as_ptr());
            r_from(fr.as_mut_ptr(), s.as_ptr());
        }
        eqb(&format!("fe25519_frombytes iter={iter}"), &fc, &fr);

        // tobytes round-trip: bytes back out must match.
        let mut bc = out_buf(32);
        let mut br = out_buf(32);
        unsafe {
            c_to(bc.as_mut_ptr(), fc.as_ptr());
            r_to(br.as_mut_ptr(), fr.as_ptr());
        }
        eqb(&format!("fe25519_tobytes iter={iter}"), &bc, &br);

        // invert: compare the whole opaque fe buffer, then also its bytes.
        let mut ic = out_buf(FE);
        let mut ir = out_buf(FE);
        unsafe {
            c_inv(ic.as_mut_ptr(), fc.as_ptr());
            r_inv(ir.as_mut_ptr(), fr.as_ptr());
        }
        eqb(&format!("fe25519_invert fe iter={iter}"), &ic, &ir);
        let mut ibc = out_buf(32);
        let mut ibr = out_buf(32);
        unsafe {
            c_to(ibc.as_mut_ptr(), ic.as_ptr());
            r_to(ibr.as_mut_ptr(), ir.as_ptr());
        }
        eqb(&format!("fe25519_invert bytes iter={iter}"), &ibc, &ibr);
    }

    // Corner inputs: all-zero, all-ones, top-bit set, canonical field order-ish.
    for (tag, s) in [
        ("zero", vec![0u8; 32]),
        ("one", {
            let mut v = vec![0u8; 32];
            v[0] = 1;
            v
        }),
        ("ff", vec![0xffu8; 32]),
        ("topbit", {
            let mut v = vec![0u8; 32];
            v[31] = 0x80;
            v
        }),
    ] {
        let mut fc = out_buf(FE);
        let mut fr = out_buf(FE);
        unsafe {
            c_from(fc.as_mut_ptr(), s.as_ptr());
            r_from(fr.as_mut_ptr(), s.as_ptr());
        }
        eqb(&format!("fe25519_frombytes corner {tag}"), &fc, &fr);
        let mut ic = out_buf(FE);
        let mut ir = out_buf(FE);
        unsafe {
            c_inv(ic.as_mut_ptr(), fc.as_ptr());
            r_inv(ir.as_mut_ptr(), fr.as_ptr());
        }
        eqb(&format!("fe25519_invert corner {tag}"), &ic, &ir);
    }
}

// ===========================================================================
// Priority 1b — sc25519 scalar helpers (all pure byte-in/byte-out)
//   void sc25519_reduce(unsigned char s[64]);                     // in place
//   void sc25519_mul(unsigned char s[32], const a[32], const b[32]);
//   void sc25519_muladd(s[32], a[32], b[32], c[32]);
//   void sc25519_invert(unsigned char recip[32], const s[32]);
//   int  sc25519_is_canonical(const unsigned char s[32]);
// ===========================================================================

type ScReduce = unsafe extern "C" fn(*mut u8);
type ScMul = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type ScMulAdd = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type ScInvert = unsafe extern "C" fn(*mut u8, *const u8);
type ScIsCanon = unsafe extern "C" fn(*const u8) -> c_int;

#[test]
fn sc25519_helpers() {
    let (c_red, r_red) = sym::<ScReduce>("_sodium_sc25519_reduce");
    let (c_mul, r_mul) = sym::<ScMul>("_sodium_sc25519_mul");
    let (c_ma, r_ma) = sym::<ScMulAdd>("_sodium_sc25519_muladd");
    let (c_inv, r_inv) = sym::<ScInvert>("_sodium_sc25519_invert");
    let (c_can, r_can) = sym::<ScIsCanon>("_sodium_sc25519_is_canonical");

    let mut rng = Rng::new(SEED ^ 1);
    for iter in 0..2000 {
        // reduce: 64-byte buffer reduced in place.
        let base = rng.bytes(64);
        let mut rc = out_buf(64);
        let mut rr = out_buf(64);
        rc[..64].copy_from_slice(&base);
        rr[..64].copy_from_slice(&base);
        unsafe {
            c_red(rc.as_mut_ptr());
            r_red(rr.as_mut_ptr());
        }
        eqb(&format!("sc25519_reduce iter={iter}"), &rc, &rr);

        let a = rng.bytes(32);
        let b = rng.bytes(32);
        let k = rng.bytes(32);

        let mut mc = out_buf(32);
        let mut mr = out_buf(32);
        unsafe {
            c_mul(mc.as_mut_ptr(), a.as_ptr(), b.as_ptr());
            r_mul(mr.as_mut_ptr(), a.as_ptr(), b.as_ptr());
        }
        eqb(&format!("sc25519_mul iter={iter}"), &mc, &mr);

        let mut ac = out_buf(32);
        let mut ar = out_buf(32);
        unsafe {
            c_ma(ac.as_mut_ptr(), a.as_ptr(), b.as_ptr(), k.as_ptr());
            r_ma(ar.as_mut_ptr(), a.as_ptr(), b.as_ptr(), k.as_ptr());
        }
        eqb(&format!("sc25519_muladd iter={iter}"), &ac, &ar);

        // invert: use the reduced scalar so it is in range and non-zero-ish.
        let s = &rc[..32];
        let mut ic = out_buf(32);
        let mut ir = out_buf(32);
        unsafe {
            c_inv(ic.as_mut_ptr(), s.as_ptr());
            r_inv(ir.as_mut_ptr(), s.as_ptr());
        }
        eqb(&format!("sc25519_invert iter={iter}"), &ic, &ir);

        // is_canonical on a raw 32-byte value.
        unsafe {
            let cc = c_can(a.as_ptr());
            let cr = r_can(a.as_ptr());
            assert_eq!(cc, cr, "sc25519_is_canonical iter={iter} input={}", hex(&a));
        }
    }

    // Canonicality corner cases: 0, L-1, L, L+1, all-ones.
    for (tag, s) in [
        ("zero", vec![0u8; 32]),
        ("ff", vec![0xffu8; 32]),
        ("hi", {
            let mut v = vec![0u8; 32];
            v[31] = 0x10;
            v
        }),
    ] {
        unsafe {
            let cc = c_can(s.as_ptr());
            let cr = r_can(s.as_ptr());
            assert_eq!(cc, cr, "sc25519_is_canonical corner {tag}");
        }
    }
}

// ===========================================================================
// Priority 1c — ge25519 group-element helpers.
// Signatures (from private/ed25519_ref10.h, with the quirks.h `_sodium_`
// rename applied):
//   void ge25519_p3_tobytes(unsigned char *s, const ge25519_p3 *h);
//   void ge25519_tobytes(unsigned char *s, const ge25519_p2 *h);
//   int  ge25519_frombytes(ge25519_p3 *h, const unsigned char *s);
//   int  ge25519_frombytes_negate_vartime(ge25519_p3 *h, const unsigned char *s);
//   void ge25519_p1p1_to_p2(ge25519_p2 *r, const ge25519_p1p1 *p);
//   void ge25519_p1p1_to_p3(ge25519_p3 *r, const ge25519_p1p1 *p);
//   void ge25519_p2_to_p3(ge25519_p3 *r, const ge25519_p2 *p);
//   void ge25519_p3_add(ge25519_p3 *r, const p3 *p, const p3 *q);
//   void ge25519_p3_sub(ge25519_p3 *r, const p3 *p, const p3 *q);
//   void ge25519_scalarmult_base(ge25519_p3 *h, const unsigned char *a);
//   void ge25519_scalarmult(ge25519_p3 *h, const a, const ge25519_p3 *p);
//   void ge25519_double_scalarmult_vartime(ge25519_p2 *r, a, const p3 *A, b);
//   void ge25519_clear_cofactor(ge25519_p3 *p3);
//   int  ge25519_is_canonical(const unsigned char *s);
//   int  ge25519_is_on_curve(const ge25519_p3 *p);
//   int  ge25519_is_on_main_subgroup(const ge25519_p3 *p);
//   int  ge25519_has_small_order(const ge25519_p3 *p);
//   void ge25519_from_uniform(unsigned char s[32], const unsigned char r[32]);
//   void ge25519_from_hash(unsigned char s[32], const unsigned char h[64]);
// Points are opaque; we compare the WHOLE oversized point buffer.
// ===========================================================================

type GePtOut = unsafe extern "C" fn(*mut u8, *const u8); // (bytes32, *point)
type GeFrom = unsafe extern "C" fn(*mut u8, *const u8) -> c_int; // (*point, bytes32)
type GeConv = unsafe extern "C" fn(*mut u8, *const u8); // (*dst_point, *src_point)
type GeBinop = unsafe extern "C" fn(*mut u8, *const u8, *const u8); // (*r, *p, *q)
type GeSmBase = unsafe extern "C" fn(*mut u8, *const u8); // (*point, scalar32)
type GeSm = unsafe extern "C" fn(*mut u8, *const u8, *const u8); // (*point, scalar32, *point)
type GeDblSm = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8); // (*p2,a,*A,b)
type GeUnary = unsafe extern "C" fn(*mut u8); // clear_cofactor(*p3)
type GeCheck = unsafe extern "C" fn(*const u8) -> c_int; // is_on_curve/... (*p3)
type GeCanon = unsafe extern "C" fn(*const u8) -> c_int; // is_canonical(bytes32)
type GeMap = unsafe extern "C" fn(*mut u8, *const u8); // from_uniform(s32,r32)/from_hash(s32,h64)

/// Produce a valid ge25519_p3 point buffer for a given scalar, from ONE library.
/// Returns the oversized point buffer.
fn ge_base_point(f: GeSmBase, scalar: &[u8]) -> Vec<u8> {
    let mut p = out_buf(PT);
    unsafe { f(p.as_mut_ptr(), scalar.as_ptr()) };
    p
}

#[test]
fn ge25519_scalarmult_base_and_tobytes() {
    let (c_smb, r_smb) = sym::<GeSmBase>("_sodium_ge25519_scalarmult_base");
    let (c_p3tb, r_p3tb) = sym::<GePtOut>("_sodium_ge25519_p3_tobytes");

    let mut rng = Rng::new(SEED ^ 2);
    for iter in 0..800 {
        let a = rng.bytes(32);
        let pc = ge_base_point(c_smb, &a);
        let pr = ge_base_point(r_smb, &a);
        // Whole opaque point buffer must match.
        eqb(&format!("ge25519_scalarmult_base point iter={iter}"), &pc, &pr);
        // and its canonical byte encoding.
        let mut bc = out_buf(32);
        let mut br = out_buf(32);
        unsafe {
            c_p3tb(bc.as_mut_ptr(), pc.as_ptr());
            r_p3tb(br.as_mut_ptr(), pr.as_ptr());
        }
        eqb(&format!("ge25519_p3_tobytes iter={iter}"), &bc, &br);
    }
}

#[test]
fn ge25519_frombytes_family() {
    let (c_smb, _r_smb) = sym::<GeSmBase>("_sodium_ge25519_scalarmult_base");
    let (c_p3tb, _) = sym::<GePtOut>("_sodium_ge25519_p3_tobytes");
    let (c_from, r_from) = sym::<GeFrom>("_sodium_ge25519_frombytes");
    let (c_fromneg, r_fromneg) =
        sym::<GeFrom>("_sodium_ge25519_frombytes_negate_vartime");
    let (c_canon, r_canon) = sym::<GeCanon>("_sodium_ge25519_is_canonical");

    let mut rng = Rng::new(SEED ^ 3);
    for iter in 0..800 {
        // Build a valid canonical encoding by scalarmult_base + p3_tobytes (C).
        let a = rng.bytes(32);
        let p = ge_base_point(c_smb, &a);
        let mut enc = vec![0u8; 32];
        unsafe { c_p3tb(enc.as_mut_ptr(), p.as_ptr()) };

        // is_canonical on a genuine encoding (expect canonical) and a random one.
        for (tag, s) in [("valid", enc.clone()), ("rand", rng.bytes(32))] {
            unsafe {
                let cc = c_canon(s.as_ptr());
                let cr = r_canon(s.as_ptr());
                assert_eq!(cc, cr, "ge25519_is_canonical {tag} iter={iter} s={}", hex(&s));
            }
        }

        // frombytes / frombytes_negate_vartime on a valid encoding: rc AND the
        // decoded opaque point buffer must match.
        for (name, cf, rf) in [
            ("ge25519_frombytes", c_from, r_from),
            ("ge25519_frombytes_negate_vartime", c_fromneg, r_fromneg),
        ] {
            let mut hc = out_buf(PT);
            let mut hr = out_buf(PT);
            unsafe {
                let rc = cf(hc.as_mut_ptr(), enc.as_ptr());
                let rr = rf(hr.as_mut_ptr(), enc.as_ptr());
                assert_eq!(rc, rr, "{name} rc iter={iter}");
            }
            eqb(&format!("{name} point iter={iter}"), &hc, &hr);
        }

        // frombytes on random (often non-canonical / off-curve) input: rc must
        // agree; only compare the decoded buffer when both accepted.
        let rnd = rng.bytes(32);
        let mut hc = out_buf(PT);
        let mut hr = out_buf(PT);
        unsafe {
            let rc = c_from(hc.as_mut_ptr(), rnd.as_ptr());
            let rr = r_from(hr.as_mut_ptr(), rnd.as_ptr());
            assert_eq!(rc, rr, "ge25519_frombytes rand rc iter={iter} s={}", hex(&rnd));
            if rc == 0 {
                eqb(&format!("ge25519_frombytes rand point iter={iter}"), &hc, &hr);
            }
        }
    }
}

#[test]
fn ge25519_point_arithmetic() {
    let (c_smb, r_smb) = sym::<GeSmBase>("_sodium_ge25519_scalarmult_base");
    let (c_add, r_add) = sym::<GeBinop>("_sodium_ge25519_p3_add");
    let (c_sub, r_sub) = sym::<GeBinop>("_sodium_ge25519_p3_sub");
    let (c_p3tb, r_p3tb) = sym::<GePtOut>("_sodium_ge25519_p3_tobytes");
    let (c_sm, r_sm) = sym::<GeSm>("_sodium_ge25519_scalarmult");
    let (c_cofac, r_cofac) = sym::<GeUnary>("_sodium_ge25519_clear_cofactor");

    let mut rng = Rng::new(SEED ^ 4);
    for iter in 0..500 {
        let a = rng.bytes(32);
        let b = rng.bytes(32);
        let pc = ge_base_point(c_smb, &a);
        let pr = ge_base_point(r_smb, &a);
        let qc = ge_base_point(c_smb, &b);
        let qr = ge_base_point(r_smb, &b);

        for (name, cf, rf) in [
            ("ge25519_p3_add", c_add, r_add),
            ("ge25519_p3_sub", c_sub, r_sub),
        ] {
            let mut oc = out_buf(PT);
            let mut or = out_buf(PT);
            unsafe {
                cf(oc.as_mut_ptr(), pc.as_ptr(), qc.as_ptr());
                rf(or.as_mut_ptr(), pr.as_ptr(), qr.as_ptr());
            }
            eqb(&format!("{name} point iter={iter}"), &oc, &or);
            // Encode the result too (catches non-canonical internal reps that
            // still encode identically — and vice-versa).
            let mut bc = out_buf(32);
            let mut br = out_buf(32);
            unsafe {
                c_p3tb(bc.as_mut_ptr(), oc.as_ptr());
                r_p3tb(br.as_mut_ptr(), or.as_ptr());
            }
            eqb(&format!("{name} bytes iter={iter}"), &bc, &br);
        }

        // scalarmult: h = b * P.
        let mut sc = out_buf(PT);
        let mut sr = out_buf(PT);
        unsafe {
            c_sm(sc.as_mut_ptr(), b.as_ptr(), pc.as_ptr());
            r_sm(sr.as_mut_ptr(), b.as_ptr(), pr.as_ptr());
        }
        eqb(&format!("ge25519_scalarmult point iter={iter}"), &sc, &sr);
        let mut bc = out_buf(32);
        let mut br = out_buf(32);
        unsafe {
            c_p3tb(bc.as_mut_ptr(), sc.as_ptr());
            r_p3tb(br.as_mut_ptr(), sr.as_ptr());
        }
        eqb(&format!("ge25519_scalarmult bytes iter={iter}"), &bc, &br);

        // clear_cofactor in place on a copy of P.
        let mut cc = pc.clone();
        let mut cr = pr.clone();
        unsafe {
            c_cofac(cc.as_mut_ptr());
            r_cofac(cr.as_mut_ptr());
        }
        eqb(&format!("ge25519_clear_cofactor iter={iter}"), &cc, &cr);
    }
}

#[test]
fn ge25519_conversions_and_double_scalarmult() {
    let (c_smb, r_smb) = sym::<GeSmBase>("_sodium_ge25519_scalarmult_base");
    let (c_dbl, r_dbl) = sym::<GeDblSm>("_sodium_ge25519_double_scalarmult_vartime");
    let (c_getb, r_getb) = sym::<GePtOut>("_sodium_ge25519_tobytes"); // p2 -> bytes
    let (c_p1p2, r_p1p2) = sym::<GeConv>("_sodium_ge25519_p1p1_to_p2");
    let (c_p1p3, r_p1p3) = sym::<GeConv>("_sodium_ge25519_p1p1_to_p3");
    let (c_p2p3, r_p2p3) = sym::<GeConv>("_sodium_ge25519_p2_to_p3");

    let mut rng = Rng::new(SEED ^ 5);
    for iter in 0..400 {
        let a = rng.bytes(32);
        let b = rng.bytes(32);
        let ac = ge_base_point(c_smb, &a);
        let ar = ge_base_point(r_smb, &a);

        // double_scalarmult_vartime: r(p2) = a*A + b*B.  Output is a p2 point.
        let mut oc = out_buf(PT);
        let mut or = out_buf(PT);
        unsafe {
            c_dbl(oc.as_mut_ptr(), a.as_ptr(), ac.as_ptr(), b.as_ptr());
            r_dbl(or.as_mut_ptr(), a.as_ptr(), ar.as_ptr(), b.as_ptr());
        }
        eqb(&format!("ge25519_double_scalarmult_vartime p2 iter={iter}"), &oc, &or);
        // Encode the p2 result via ge25519_tobytes.
        let mut bc = out_buf(32);
        let mut br = out_buf(32);
        unsafe {
            c_getb(bc.as_mut_ptr(), oc.as_ptr());
            r_getb(br.as_mut_ptr(), or.as_ptr());
        }
        eqb(&format!("ge25519_tobytes(p2) iter={iter}"), &bc, &br);

        // p2_to_p3 on the p2 result, then compare the resulting p3.
        let mut p3c = out_buf(PT);
        let mut p3r = out_buf(PT);
        unsafe {
            c_p2p3(p3c.as_mut_ptr(), oc.as_ptr());
            r_p2p3(p3r.as_mut_ptr(), or.as_ptr());
        }
        eqb(&format!("ge25519_p2_to_p3 iter={iter}"), &p3c, &p3r);

        // p1p1_to_p2 / p1p1_to_p3: we do not have a public constructor for a
        // p1p1 point, so drive them from a raw randomized p1p1-sized buffer.
        // Both libraries read the SAME bytes, so the transform must agree even
        // if the buffer is not a "real" completed point.
        let raw = rng.bytes(PT);
        for (name, cf, rf) in [
            ("ge25519_p1p1_to_p2", c_p1p2, r_p1p2),
            ("ge25519_p1p1_to_p3", c_p1p3, r_p1p3),
        ] {
            let mut dc = out_buf(PT);
            let mut dr = out_buf(PT);
            unsafe {
                cf(dc.as_mut_ptr(), raw.as_ptr());
                rf(dr.as_mut_ptr(), raw.as_ptr());
            }
            eqb(&format!("{name} iter={iter}"), &dc, &dr);
        }
    }
}

#[test]
fn ge25519_predicates() {
    let (c_smb, r_smb) = sym::<GeSmBase>("_sodium_ge25519_scalarmult_base");
    let (c_onc, r_onc) = sym::<GeCheck>("_sodium_ge25519_is_on_curve");
    let (c_msg, r_msg) = sym::<GeCheck>("_sodium_ge25519_is_on_main_subgroup");
    let (c_sml, r_sml) = sym::<GeCheck>("_sodium_ge25519_has_small_order");

    let mut rng = Rng::new(SEED ^ 6);
    for iter in 0..600 {
        let a = rng.bytes(32);
        let pc = ge_base_point(c_smb, &a);
        let pr = ge_base_point(r_smb, &a);
        for (name, cf, rf) in [
            ("ge25519_is_on_curve", c_onc, r_onc),
            ("ge25519_is_on_main_subgroup", c_msg, r_msg),
            ("ge25519_has_small_order", c_sml, r_sml),
        ] {
            unsafe {
                let cc = cf(pc.as_ptr());
                let cr = rf(pr.as_ptr());
                assert_eq!(cc, cr, "{name} valid iter={iter}");
            }
        }
        // Also feed a raw random point-sized buffer (both read identical bytes).
        let raw = rng.bytes(PT);
        for (name, cf, rf) in [
            ("ge25519_is_on_curve", c_onc, r_onc),
            ("ge25519_is_on_main_subgroup", c_msg, r_msg),
            ("ge25519_has_small_order", c_sml, r_sml),
        ] {
            unsafe {
                let cc = cf(raw.as_ptr());
                let cr = rf(raw.as_ptr());
                assert_eq!(cc, cr, "{name} raw iter={iter}");
            }
        }
    }
}

#[test]
fn ge25519_from_uniform_and_hash() {
    let (c_uni, r_uni) = sym::<GeMap>("_sodium_ge25519_from_uniform");
    let (c_hash, r_hash) = sym::<GeMap>("_sodium_ge25519_from_hash");

    let mut rng = Rng::new(SEED ^ 7);
    for iter in 0..1000 {
        let r32 = rng.bytes(32);
        let mut sc = out_buf(32);
        let mut sr = out_buf(32);
        unsafe {
            c_uni(sc.as_mut_ptr(), r32.as_ptr());
            r_uni(sr.as_mut_ptr(), r32.as_ptr());
        }
        eqb(&format!("ge25519_from_uniform iter={iter}"), &sc, &sr);

        let h64 = rng.bytes(64);
        let mut hc = out_buf(32);
        let mut hr = out_buf(32);
        unsafe {
            c_hash(hc.as_mut_ptr(), h64.as_ptr());
            r_hash(hr.as_mut_ptr(), h64.as_ptr());
        }
        eqb(&format!("ge25519_from_hash iter={iter}"), &hc, &hr);
    }
}

// ===========================================================================
// Priority 1d — ristretto255
//   int  ristretto255_frombytes(ge25519_p3 *h, const unsigned char *s);
//   void ristretto255_p3_tobytes(unsigned char *s, const ge25519_p3 *h);
//   void ristretto255_from_hash(unsigned char s[32], const unsigned char h[64]);
// ===========================================================================

#[test]
fn ristretto255_helpers() {
    let (c_from, r_from) = sym::<GeFrom>("_sodium_ristretto255_frombytes");
    let (c_tob, r_tob) = sym::<GePtOut>("_sodium_ristretto255_p3_tobytes");
    let (c_hash, r_hash) = sym::<GeMap>("_sodium_ristretto255_from_hash");

    let mut rng = Rng::new(SEED ^ 8);
    for iter in 0..1000 {
        // from_hash always yields a valid canonical ristretto encoding.
        let h64 = rng.bytes(64);
        let mut ec = out_buf(32);
        let mut er = out_buf(32);
        unsafe {
            c_hash(ec.as_mut_ptr(), h64.as_ptr());
            r_hash(er.as_mut_ptr(), h64.as_ptr());
        }
        eqb(&format!("ristretto255_from_hash iter={iter}"), &ec, &er);

        // frombytes on that valid encoding: rc + decoded point buffer.
        let enc = ec[..32].to_vec();
        let mut hc = out_buf(PT);
        let mut hr = out_buf(PT);
        unsafe {
            let rc = c_from(hc.as_mut_ptr(), enc.as_ptr());
            let rr = r_from(hr.as_mut_ptr(), enc.as_ptr());
            assert_eq!(rc, rr, "ristretto255_frombytes valid rc iter={iter}");
        }
        eqb(&format!("ristretto255_frombytes point iter={iter}"), &hc, &hr);

        // p3_tobytes must round-trip back to the same encoding.
        let mut bc = out_buf(32);
        let mut br = out_buf(32);
        unsafe {
            c_tob(bc.as_mut_ptr(), hc.as_ptr());
            r_tob(br.as_mut_ptr(), hr.as_ptr());
        }
        eqb(&format!("ristretto255_p3_tobytes iter={iter}"), &bc, &br);

        // frombytes on random input: rc must agree; compare point only if both
        // accepted.
        let rnd = rng.bytes(32);
        let mut rc_buf = out_buf(PT);
        let mut rr_buf = out_buf(PT);
        unsafe {
            let rc = c_from(rc_buf.as_mut_ptr(), rnd.as_ptr());
            let rr = r_from(rr_buf.as_mut_ptr(), rnd.as_ptr());
            assert_eq!(rc, rr, "ristretto255_frombytes rand rc iter={iter} s={}", hex(&rnd));
            if rc == 0 {
                eqb(&format!("ristretto255_frombytes rand point iter={iter}"), &rc_buf, &rr_buf);
            }
        }
    }
}

// ===========================================================================
// Priority 2 — _sodium_keccak1600_ref_* raw sponge state API.
//   void keccak1600_ref_init(void *state);
//   void keccak1600_ref_xor_bytes(void *state, const unsigned char *bytes,
//                                 size_t offset, size_t length);
//   void keccak1600_ref_extract_bytes(const void *state, unsigned char *bytes,
//                                     size_t offset, size_t length);
//   void keccak1600_ref_permute_24(void *state);
//   void keccak1600_ref_permute_12(void *state);
// Same treatment as the public crypto_core_keccak1600_* API in t03_core.rs.
// ===========================================================================

const KECCAK_STATE_BYTES: usize = 256; // >= sizeof(crypto_core_keccak1600_state)

type KInit = unsafe extern "C" fn(*mut u8);
type KXor = unsafe extern "C" fn(*mut u8, *const u8, usize, usize);
type KExtract = unsafe extern "C" fn(*const u8, *mut u8, usize, usize);
type KPermute = unsafe extern "C" fn(*mut u8);

#[test]
fn sodium_keccak1600_ref_state_api() {
    let (cinit, rinit) = sym::<KInit>("_sodium_keccak1600_ref_init");
    let (cxor, rxor) = sym::<KXor>("_sodium_keccak1600_ref_xor_bytes");
    let (cext, rext) = sym::<KExtract>("_sodium_keccak1600_ref_extract_bytes");
    let (cp24, rp24) = sym::<KPermute>("_sodium_keccak1600_ref_permute_24");
    let (cp12, rp12) = sym::<KPermute>("_sodium_keccak1600_ref_permute_12");

    // init must zero identically; start from a poisoned buffer.
    let mut sc = vec![0xa5u8; KECCAK_STATE_BYTES];
    let mut sr = vec![0xa5u8; KECCAK_STATE_BYTES];
    unsafe {
        cinit(sc.as_mut_ptr());
        rinit(sr.as_mut_ptr());
    }
    // The keccak state is 200 bytes of lane storage; compare that prefix.
    eqb("keccak1600_ref init", &sc[..200], &sr[..200]);

    let mut rng = Rng::new(SEED ^ 0x20);
    for round in 0..300 {
        let mut sc = vec![0u8; KECCAK_STATE_BYTES];
        let mut sr = vec![0u8; KECCAK_STATE_BYTES];
        unsafe {
            cinit(sc.as_mut_ptr());
            rinit(sr.as_mut_ptr());
        }
        for step in 0..8 {
            let offset = rng.below(200);
            let maxlen = 200usize.saturating_sub(offset);
            let length = if maxlen == 0 { 0 } else { rng.below(maxlen + 1) };
            let data = rng.bytes(length.max(1));
            unsafe {
                cxor(sc.as_mut_ptr(), data.as_ptr(), offset, length);
                rxor(sr.as_mut_ptr(), data.as_ptr(), offset, length);
            }
            eqb(
                &format!("keccak1600_ref xor round={round} step={step} off={offset} len={length}"),
                &sc[..200],
                &sr[..200],
            );
            match rng.below(3) {
                0 => unsafe {
                    cp24(sc.as_mut_ptr());
                    rp24(sr.as_mut_ptr());
                },
                1 => unsafe {
                    cp12(sc.as_mut_ptr());
                    rp12(sr.as_mut_ptr());
                },
                _ => {}
            }
            eqb(&format!("keccak1600_ref permute round={round} step={step}"), &sc[..200], &sr[..200]);
            let eoff = rng.below(200);
            let elen = rng.below(200usize.saturating_sub(eoff) + 1);
            let mut oc = out_buf(elen);
            let mut or = out_buf(elen);
            unsafe {
                cext(sc.as_ptr(), oc.as_mut_ptr(), eoff, elen);
                rext(sr.as_ptr(), or.as_mut_ptr(), eoff, elen);
            }
            eqb(
                &format!("keccak1600_ref extract round={round} step={step} off={eoff} len={elen}"),
                &oc,
                &or,
            );
        }
    }

    // Word-alignment boundary sweep.
    for offset in 0usize..=9 {
        for length in 0usize..=24 {
            if offset + length > 200 {
                continue;
            }
            let mut sc = vec![0u8; KECCAK_STATE_BYTES];
            let mut sr = vec![0u8; KECCAK_STATE_BYTES];
            unsafe {
                cinit(sc.as_mut_ptr());
                rinit(sr.as_mut_ptr());
            }
            let data: Vec<u8> = (0..length.max(1))
                .map(|i| (i as u8).wrapping_mul(37).wrapping_add(11))
                .collect();
            unsafe {
                cxor(sc.as_mut_ptr(), data.as_ptr(), offset, length);
                rxor(sr.as_mut_ptr(), data.as_ptr(), offset, length);
                cp24(sc.as_mut_ptr());
                rp24(sr.as_mut_ptr());
            }
            eqb(&format!("keccak1600_ref sweep off={offset} len={length}"), &sc[..200], &sr[..200]);
        }
    }

    // Long alternating permutation chain.
    let mut sc = vec![0u8; KECCAK_STATE_BYTES];
    let mut sr = vec![0u8; KECCAK_STATE_BYTES];
    unsafe {
        cinit(sc.as_mut_ptr());
        rinit(sr.as_mut_ptr());
        let seed = [1u8, 2, 3, 4, 5, 6, 7, 8];
        cxor(sc.as_mut_ptr(), seed.as_ptr(), 0, 8);
        rxor(sr.as_mut_ptr(), seed.as_ptr(), 0, 8);
    }
    for i in 0..200 {
        unsafe {
            if i % 2 == 0 {
                cp24(sc.as_mut_ptr());
                rp24(sr.as_mut_ptr());
            } else {
                cp12(sc.as_mut_ptr());
                rp12(sr.as_mut_ptr());
            }
        }
        eqb(&format!("keccak1600_ref chain i={i}"), &sc[..200], &sr[..200]);
    }
}

// ===========================================================================
// Priority 3 — shake / turboshake reference XOFs.
//   int shakeNNN_ref(unsigned char *out, size_t outlen, const in, size_t inlen);
//   int shakeNNN_ref_init(state*);
//   int shakeNNN_ref_init_with_domain(state*, unsigned char domain);
//   int shakeNNN_ref_update(state*, const in, size_t inlen);
//   int shakeNNN_ref_squeeze(state*, unsigned char *out, size_t outlen);
// state = { crypto_core_keccak1600_state; size_t offset; u8 phase; u8 domain }.
// ===========================================================================

const XOF_STATE_BYTES: usize = 256; // >= sizeof(shakeNNN_state_internal)

type XofOneShot = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> c_int;
type XofInit = unsafe extern "C" fn(*mut u8) -> c_int;
type XofInitDom = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type XofUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> c_int;
type XofSqueeze = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> c_int;

fn xof_streaming_case(prefix: &str, with_domain: bool, seed_off: u64) {
    let (c_init, r_init) = sym::<XofInit>(&format!("{prefix}_init"));
    let (c_initd, r_initd) = sym::<XofInitDom>(&format!("{prefix}_init_with_domain"));
    let (c_upd, r_upd) = sym::<XofUpdate>(&format!("{prefix}_update"));
    let (c_sq, r_sq) = sym::<XofSqueeze>(&format!("{prefix}_squeeze"));

    let mut rng = Rng::new(SEED ^ seed_off);
    for iter in 0..200 {
        let mut sc = vec![0u8; XOF_STATE_BYTES];
        let mut sr = vec![0u8; XOF_STATE_BYTES];
        unsafe {
            if with_domain {
                let dom = 0x80u8 | (rng.byte() & 0x7f); // turboshake domain range
                let rc = c_initd(sc.as_mut_ptr(), dom);
                let rr = r_initd(sr.as_mut_ptr(), dom);
                assert_eq!(rc, rr, "{prefix}_init_with_domain rc iter={iter}");
            } else {
                let rc = c_init(sc.as_mut_ptr());
                let rr = r_init(sr.as_mut_ptr());
                assert_eq!(rc, rr, "{prefix}_init rc iter={iter}");
            }
        }
        eqb(&format!("{prefix} state after init iter={iter}"), &sc[..200], &sr[..200]);

        // Several updates of random length.
        let nupd = 1 + rng.below(5);
        for u in 0..nupd {
            let inlen = rng.below(300);
            let data = rng.bytes(inlen.max(1));
            unsafe {
                let rc = c_upd(sc.as_mut_ptr(), data.as_ptr(), inlen);
                let rr = r_upd(sr.as_mut_ptr(), data.as_ptr(), inlen);
                assert_eq!(rc, rr, "{prefix}_update rc iter={iter} u={u}");
            }
            eqb(&format!("{prefix} state after update iter={iter} u={u}"), &sc[..200], &sr[..200]);
        }

        // Several squeezes of random length.
        let nsq = 1 + rng.below(4);
        for q in 0..nsq {
            let outlen = rng.below(400);
            let mut oc = out_buf(outlen);
            let mut or = out_buf(outlen);
            unsafe {
                let rc = c_sq(sc.as_mut_ptr(), oc.as_mut_ptr(), outlen);
                let rr = r_sq(sr.as_mut_ptr(), or.as_mut_ptr(), outlen);
                assert_eq!(rc, rr, "{prefix}_squeeze rc iter={iter} q={q}");
            }
            eqb(&format!("{prefix}_squeeze out iter={iter} q={q}"), &oc, &or);
            eqb(&format!("{prefix} state after squeeze iter={iter} q={q}"), &sc[..200], &sr[..200]);
        }
    }
}

#[test]
fn sodium_shake_turboshake_oneshot() {
    for name in [
        "_sodium_shake128_ref",
        "_sodium_shake256_ref",
        "_sodium_turboshake128_ref",
        "_sodium_turboshake256_ref",
    ] {
        let (c, r) = sym::<XofOneShot>(name);
        let mut rng = Rng::new(SEED ^ name.len() as u64);
        for iter in 0..300 {
            let inlen = rng.below(400);
            let outlen = rng.below(400);
            let inp = rng.bytes(inlen.max(1));
            let mut oc = out_buf(outlen);
            let mut or = out_buf(outlen);
            unsafe {
                let rc = c(oc.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                let rr = r(or.as_mut_ptr(), outlen, inp.as_ptr(), inlen);
                assert_eq!(rc, rr, "{name} rc iter={iter}");
            }
            eqb(&format!("{name} out iter={iter} inlen={inlen} outlen={outlen}"), &oc, &or);
        }
    }
}

#[test]
fn sodium_shake_streaming() {
    xof_streaming_case("_sodium_shake128_ref", false, 0x30);
    xof_streaming_case("_sodium_shake256_ref", false, 0x31);
}

#[test]
fn sodium_turboshake_streaming() {
    xof_streaming_case("_sodium_turboshake128_ref", true, 0x32);
    xof_streaming_case("_sodium_turboshake256_ref", true, 0x33);
}

// ===========================================================================
// Priority 4 — blake2b internals.
//   int blake2b(uint8_t *out, const void *in, const void *key,
//               const uint8_t outlen, const uint64_t inlen, uint8_t keylen);
//   int blake2b_init(blake2b_state *S, const uint8_t outlen);
//   int blake2b_init_key(S, outlen, const void *key, uint8_t keylen);
//   int blake2b_init_param(S, const blake2b_param *P);
//   int blake2b_init_salt_personal(S, outlen, salt, personal);
//   int blake2b_init_key_salt_personal(S, outlen, key, keylen, salt, personal);
//   int blake2b_update(S, const uint8_t *in, uint64_t inlen);
//   int blake2b_final(S, uint8_t *out, uint8_t outlen);
//   int blake2b_salt_personal(out, in, key, outlen, inlen, keylen, salt, pers);
//   int blake2b_long(void *out, size_t outlen, const void *in, size_t inlen);
//   int blake2b_compress_ref(blake2b_state *S, const uint8_t block[128]);
// NOTE: out-of-range outlen (0 or >64) and keylen (>64) call sodium_misuse()
// -> abort() in the C, so those inputs are routed through same_outcome().
// blake2b_state is 8*u64 + 2*u64 + 2*u64 + 256 + size_t + u8 => ~361 bytes.
// ===========================================================================

const BLAKE2B_STATE_BYTES: usize = 512; // generous >= sizeof(blake2b_state)
const B2B_OUTBYTES: usize = 64;
const B2B_KEYBYTES: usize = 64;

type B2b = unsafe extern "C" fn(*mut u8, *const c_void, *const c_void, u8, u64, u8) -> c_int;
type B2bSaltPers =
    unsafe extern "C" fn(*mut u8, *const c_void, *const c_void, u8, u64, u8, *const c_void, *const c_void) -> c_int;
type B2bInit = unsafe extern "C" fn(*mut u8, u8) -> c_int;
type B2bInitKey = unsafe extern "C" fn(*mut u8, u8, *const c_void, u8) -> c_int;
type B2bInitParam = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type B2bInitSP = unsafe extern "C" fn(*mut u8, u8, *const c_void, *const c_void) -> c_int;
type B2bInitKSP =
    unsafe extern "C" fn(*mut u8, u8, *const c_void, u8, *const c_void, *const c_void) -> c_int;
type B2bUpdate = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type B2bFinal = unsafe extern "C" fn(*mut u8, *mut u8, u8) -> c_int;
type B2bLong = unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize) -> c_int;
type B2bCompress = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;

#[test]
fn sodium_blake2b_oneshot() {
    let (c, r) = sym::<B2b>("_sodium_blake2b");
    let mut rng = Rng::new(SEED ^ 0x40);
    for iter in 0..800 {
        let outlen = 1 + rng.below(B2B_OUTBYTES); // 1..=64 valid
        let inlen = rng.below(300);
        let keylen = rng.below(B2B_KEYBYTES + 1); // 0..=64 valid
        let inp = rng.bytes(inlen.max(1));
        let key = rng.bytes(keylen.max(1));
        let keyp = if keylen == 0 {
            ptr::null()
        } else {
            key.as_ptr() as *const c_void
        };
        let mut oc = out_buf(outlen);
        let mut or = out_buf(outlen);
        unsafe {
            let rc = c(
                oc.as_mut_ptr(),
                inp.as_ptr() as *const c_void,
                keyp,
                outlen as u8,
                inlen as u64,
                keylen as u8,
            );
            let rr = r(
                or.as_mut_ptr(),
                inp.as_ptr() as *const c_void,
                keyp,
                outlen as u8,
                inlen as u64,
                keylen as u8,
            );
            assert_eq!(rc, rr, "blake2b rc iter={iter}");
        }
        eqb(&format!("blake2b out iter={iter} outlen={outlen} keylen={keylen}"), &oc, &or);
    }
}

#[test]
fn sodium_blake2b_salt_personal() {
    let (c, r) = sym::<B2bSaltPers>("_sodium_blake2b_salt_personal");
    let mut rng = Rng::new(SEED ^ 0x41);
    for iter in 0..600 {
        let outlen = 1 + rng.below(B2B_OUTBYTES);
        let inlen = rng.below(200);
        let keylen = rng.below(B2B_KEYBYTES + 1);
        let inp = rng.bytes(inlen.max(1));
        let key = rng.bytes(keylen.max(1));
        let salt = rng.bytes(16);
        let pers = rng.bytes(16);
        let keyp = if keylen == 0 { ptr::null() } else { key.as_ptr() as *const c_void };
        let mut oc = out_buf(outlen);
        let mut or = out_buf(outlen);
        unsafe {
            let rc = c(
                oc.as_mut_ptr(), inp.as_ptr() as *const c_void, keyp,
                outlen as u8, inlen as u64, keylen as u8,
                salt.as_ptr() as *const c_void, pers.as_ptr() as *const c_void,
            );
            let rr = r(
                or.as_mut_ptr(), inp.as_ptr() as *const c_void, keyp,
                outlen as u8, inlen as u64, keylen as u8,
                salt.as_ptr() as *const c_void, pers.as_ptr() as *const c_void,
            );
            assert_eq!(rc, rr, "blake2b_salt_personal rc iter={iter}");
        }
        eqb(&format!("blake2b_salt_personal out iter={iter}"), &oc, &or);
    }
}

#[test]
fn sodium_blake2b_streaming() {
    let (c_init, r_init) = sym::<B2bInit>("_sodium_blake2b_init");
    let (c_ik, r_ik) = sym::<B2bInitKey>("_sodium_blake2b_init_key");
    let (c_isp, r_isp) = sym::<B2bInitSP>("_sodium_blake2b_init_salt_personal");
    let (c_iksp, r_iksp) = sym::<B2bInitKSP>("_sodium_blake2b_init_key_salt_personal");
    let (c_upd, r_upd) = sym::<B2bUpdate>("_sodium_blake2b_update");
    let (c_fin, r_fin) = sym::<B2bFinal>("_sodium_blake2b_final");

    let mut rng = Rng::new(SEED ^ 0x42);
    for iter in 0..400 {
        let outlen = 1 + rng.below(B2B_OUTBYTES);
        let keylen = 1 + rng.below(B2B_KEYBYTES); // 1..=64 for keyed variants
        let key = rng.bytes(keylen);
        let salt = rng.bytes(16);
        let pers = rng.bytes(16);

        // Choose one of the four init variants.
        let variant = rng.below(4);
        let mut sc = vec![0u8; BLAKE2B_STATE_BYTES];
        let mut sr = vec![0u8; BLAKE2B_STATE_BYTES];
        let tag = unsafe {
            match variant {
                0 => {
                    let rc = c_init(sc.as_mut_ptr(), outlen as u8);
                    let rr = r_init(sr.as_mut_ptr(), outlen as u8);
                    assert_eq!(rc, rr, "blake2b_init rc iter={iter}");
                    "init"
                }
                1 => {
                    let rc = c_ik(sc.as_mut_ptr(), outlen as u8, key.as_ptr() as *const c_void, keylen as u8);
                    let rr = r_ik(sr.as_mut_ptr(), outlen as u8, key.as_ptr() as *const c_void, keylen as u8);
                    assert_eq!(rc, rr, "blake2b_init_key rc iter={iter}");
                    "init_key"
                }
                2 => {
                    let rc = c_isp(sc.as_mut_ptr(), outlen as u8, salt.as_ptr() as *const c_void, pers.as_ptr() as *const c_void);
                    let rr = r_isp(sr.as_mut_ptr(), outlen as u8, salt.as_ptr() as *const c_void, pers.as_ptr() as *const c_void);
                    assert_eq!(rc, rr, "blake2b_init_salt_personal rc iter={iter}");
                    "init_salt_personal"
                }
                _ => {
                    let rc = c_iksp(sc.as_mut_ptr(), outlen as u8, key.as_ptr() as *const c_void, keylen as u8, salt.as_ptr() as *const c_void, pers.as_ptr() as *const c_void);
                    let rr = r_iksp(sr.as_mut_ptr(), outlen as u8, key.as_ptr() as *const c_void, keylen as u8, salt.as_ptr() as *const c_void, pers.as_ptr() as *const c_void);
                    assert_eq!(rc, rr, "blake2b_init_key_salt_personal rc iter={iter}");
                    "init_key_salt_personal"
                }
            }
        };
        eqb(&format!("blake2b state after {tag} iter={iter}"), &sc[..361], &sr[..361]);

        // Several updates.
        for u in 0..(1 + rng.below(4)) {
            let inlen = rng.below(300);
            let data = rng.bytes(inlen.max(1));
            unsafe {
                let rc = c_upd(sc.as_mut_ptr(), data.as_ptr(), inlen as u64);
                let rr = r_upd(sr.as_mut_ptr(), data.as_ptr(), inlen as u64);
                assert_eq!(rc, rr, "blake2b_update rc iter={iter} u={u}");
            }
            eqb(&format!("blake2b state after update iter={iter} u={u}"), &sc[..361], &sr[..361]);
        }

        // final.
        let mut oc = out_buf(outlen);
        let mut or = out_buf(outlen);
        unsafe {
            let rc = c_fin(sc.as_mut_ptr(), oc.as_mut_ptr(), outlen as u8);
            let rr = r_fin(sr.as_mut_ptr(), or.as_mut_ptr(), outlen as u8);
            assert_eq!(rc, rr, "blake2b_final rc iter={iter}");
        }
        eqb(&format!("blake2b_final out iter={iter} variant={variant}"), &oc, &or);
    }
}

#[test]
fn sodium_blake2b_init_param() {
    // blake2b_param is 64 bytes, #[repr(C, packed)]; we build a raw 64-byte
    // buffer with a valid digest_length (1..=64) and fanout/depth = 1, then
    // compare the init'd state.  Both libraries read the SAME bytes.
    let (c_ip, r_ip) = sym::<B2bInitParam>("_sodium_blake2b_init_param");
    let mut rng = Rng::new(SEED ^ 0x43);
    for iter in 0..300 {
        let mut param = [0u8; 64];
        param[0] = (1 + rng.below(B2B_OUTBYTES)) as u8; // digest_length
        param[1] = rng.below(B2B_KEYBYTES + 1) as u8; // key_length
        param[2] = 1; // fanout
        param[3] = 1; // depth
        // leaf_length (4), node_offset (8) left 0; salt/personal randomized.
        for b in param[32..64].iter_mut() {
            *b = rng.byte();
        }
        let mut sc = vec![0u8; BLAKE2B_STATE_BYTES];
        let mut sr = vec![0u8; BLAKE2B_STATE_BYTES];
        unsafe {
            let rc = c_ip(sc.as_mut_ptr(), param.as_ptr());
            let rr = r_ip(sr.as_mut_ptr(), param.as_ptr());
            assert_eq!(rc, rr, "blake2b_init_param rc iter={iter}");
        }
        eqb(&format!("blake2b_init_param state iter={iter}"), &sc[..361], &sr[..361]);
    }
}

#[test]
fn sodium_blake2b_long() {
    // blake2b_long returns -1 (no abort) for outlen > UINT32_MAX; with usize
    // inputs we stay well within range, so compare directly.
    let (c, r) = sym::<B2bLong>("_sodium_blake2b_long");
    let mut rng = Rng::new(SEED ^ 0x44);
    for iter in 0..400 {
        // Exercise both the <=64 fast path and the >64 expansion path.
        let outlen = 1 + rng.below(200);
        let inlen = rng.below(200);
        let inp = rng.bytes(inlen.max(1));
        let mut oc = out_buf(outlen);
        let mut or = out_buf(outlen);
        unsafe {
            let rc = c(oc.as_mut_ptr() as *mut c_void, outlen, inp.as_ptr() as *const c_void, inlen);
            let rr = r(or.as_mut_ptr() as *mut c_void, outlen, inp.as_ptr() as *const c_void, inlen);
            assert_eq!(rc, rr, "blake2b_long rc iter={iter} outlen={outlen}");
        }
        eqb(&format!("blake2b_long out iter={iter} outlen={outlen} inlen={inlen}"), &oc, &or);
    }
}

#[test]
fn sodium_blake2b_compress_ref() {
    // blake2b_compress_ref(state, block[128]).  We initialise a valid state via
    // blake2b_init, then compress an identical random 128-byte block into both
    // states and compare.  (Driving compress directly needs a valid state; a
    // freshly init'd state is the natural valid input.)
    let (c_init, r_init) = sym::<B2bInit>("_sodium_blake2b_init");
    let (c_cp, r_cp) = sym::<B2bCompress>("_sodium_blake2b_compress_ref");
    let mut rng = Rng::new(SEED ^ 0x45);
    for iter in 0..400 {
        let mut sc = vec![0u8; BLAKE2B_STATE_BYTES];
        let mut sr = vec![0u8; BLAKE2B_STATE_BYTES];
        unsafe {
            c_init(sc.as_mut_ptr(), 64);
            r_init(sr.as_mut_ptr(), 64);
        }
        let block = rng.bytes(128);
        unsafe {
            let rc = c_cp(sc.as_mut_ptr(), block.as_ptr());
            let rr = r_cp(sr.as_mut_ptr(), block.as_ptr());
            assert_eq!(rc, rr, "blake2b_compress_ref rc iter={iter}");
        }
        eqb(&format!("blake2b_compress_ref state iter={iter}"), &sc[..361], &sr[..361]);
    }
}

#[test]
fn sodium_blake2b_misuse_paths() {
    // Out-of-range outlen (0, 65) and keylen (65) reach sodium_misuse()->abort()
    // in the C.  Use same_outcome so both sides must abort identically.
    let (c, r) = sym::<B2b>("_sodium_blake2b");
    let inp = [0u8; 8];
    let key = [0u8; 80];
    let mkcall = move |f: B2b, outlen: u8, keylen: u8| {
        move || -> i32 {
            let mut o = vec![0u8; 64];
            let kp = if keylen == 0 { ptr::null() } else { key.as_ptr() as *const c_void };
            unsafe { f(o.as_mut_ptr(), inp.as_ptr() as *const c_void, kp, outlen, inp.len() as u64, keylen) }
        }
    };
    for (tag, outlen, keylen) in [
        ("outlen0", 0u8, 0u8),
        ("outlen65", 65u8, 0u8),
        ("keylen65", 32u8, 65u8),
        ("keylen80", 32u8, 80u8),
    ] {
        same_outcome(
            &format!("blake2b misuse {tag}"),
            mkcall(c, outlen, keylen),
            mkcall(r, outlen, keylen),
        );
    }

    // init misuse.
    let (ci, ri) = sym::<B2bInit>("_sodium_blake2b_init");
    for (tag, outlen) in [("init0", 0u8), ("init65", 65u8)] {
        let mk = move |f: B2bInit| {
            move || -> i32 {
                let mut s = vec![0u8; BLAKE2B_STATE_BYTES];
                unsafe { f(s.as_mut_ptr(), outlen) }
            }
        };
        same_outcome(&format!("blake2b_init misuse {tag}"), mk(ci), mk(ri));
    }
}

// ===========================================================================
// Priority 5 — softaes.  From private/softaes.h:
//   typedef struct SoftAesBlock { uint32_t w0,w1,w2,w3; } SoftAesBlock;  // 16B
//   void softaes_expand_key128(SoftAesBlock rkeys[11], const uint8_t key[16]);
//   void softaes_expand_key256(SoftAesBlock rkeys[15], const uint8_t key[32]);
//   SoftAesBlock softaes_inv_mix_columns(const SoftAesBlock);
//   void softaes_invert_key_schedule128(SoftAesBlock rkeys[11]);
//   void softaes_invert_key_schedule256(SoftAesBlock rkeys[15]);
//   SoftAesBlock softaes_block_encrypt(const SoftAesBlock, const SoftAesBlock rk);
//   SoftAesBlock softaes_block_decrypt(const SoftAesBlock, const SoftAesBlock rk);
//   SoftAesBlock softaes_block_encryptlast(const SoftAesBlock, const SoftAesBlock);
//   SoftAesBlock softaes_block_decryptlast(const SoftAesBlock, const SoftAesBlock);
// These pass/return the 16-byte struct BY VALUE; matched with #[repr(C)].
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SoftAesBlock {
    w0: u32,
    w1: u32,
    w2: u32,
    w3: u32,
}

impl SoftAesBlock {
    fn as_bytes(&self) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[0..4].copy_from_slice(&self.w0.to_le_bytes());
        b[4..8].copy_from_slice(&self.w1.to_le_bytes());
        b[8..12].copy_from_slice(&self.w2.to_le_bytes());
        b[12..16].copy_from_slice(&self.w3.to_le_bytes());
        b
    }
}

type AesBinop = unsafe extern "C" fn(SoftAesBlock, SoftAesBlock) -> SoftAesBlock;
type AesUnop = unsafe extern "C" fn(SoftAesBlock) -> SoftAesBlock;
type AesExpand128 = unsafe extern "C" fn(*mut SoftAesBlock, *const u8);
type AesExpand256 = unsafe extern "C" fn(*mut SoftAesBlock, *const u8);
type AesInvSched = unsafe extern "C" fn(*mut SoftAesBlock);

fn rand_block(rng: &mut Rng) -> SoftAesBlock {
    SoftAesBlock {
        w0: rng.next_u32(),
        w1: rng.next_u32(),
        w2: rng.next_u32(),
        w3: rng.next_u32(),
    }
}

#[test]
fn sodium_softaes_block_ops() {
    let (c_enc, r_enc) = sym::<AesBinop>("_sodium_softaes_block_encrypt");
    let (c_encl, r_encl) = sym::<AesBinop>("_sodium_softaes_block_encryptlast");
    let (c_dec, r_dec) = sym::<AesBinop>("_sodium_softaes_block_decrypt");
    let (c_decl, r_decl) = sym::<AesBinop>("_sodium_softaes_block_decryptlast");
    let (c_imc, r_imc) = sym::<AesUnop>("_sodium_softaes_inv_mix_columns");

    let mut rng = Rng::new(SEED ^ 0x50);
    for iter in 0..3000 {
        let blk = rand_block(&mut rng);
        let rk = rand_block(&mut rng);
        for (name, cf, rf) in [
            ("softaes_block_encrypt", c_enc, r_enc),
            ("softaes_block_encryptlast", c_encl, r_encl),
            ("softaes_block_decrypt", c_dec, r_dec),
            ("softaes_block_decryptlast", c_decl, r_decl),
        ] {
            let (rc, rr) = unsafe { (cf(blk, rk), rf(blk, rk)) };
            eqb(&format!("{name} iter={iter}"), &rc.as_bytes(), &rr.as_bytes());
        }
        let (ic, ir) = unsafe { (c_imc(blk), r_imc(blk)) };
        eqb(&format!("softaes_inv_mix_columns iter={iter}"), &ic.as_bytes(), &ir.as_bytes());
    }
}

#[test]
fn sodium_softaes_key_schedules() {
    let (c_e128, r_e128) = sym::<AesExpand128>("_sodium_softaes_expand_key128");
    let (c_e256, r_e256) = sym::<AesExpand256>("_sodium_softaes_expand_key256");
    let (c_i128, r_i128) = sym::<AesInvSched>("_sodium_softaes_invert_key_schedule128");
    let (c_i256, r_i256) = sym::<AesInvSched>("_sodium_softaes_invert_key_schedule256");

    let zero = SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 };
    let mut rng = Rng::new(SEED ^ 0x51);
    for iter in 0..1000 {
        // 128-bit: 11 round keys.
        let key16 = rng.bytes(16);
        let mut rc = [zero; 11];
        let mut rr = [zero; 11];
        unsafe {
            c_e128(rc.as_mut_ptr(), key16.as_ptr());
            r_e128(rr.as_mut_ptr(), key16.as_ptr());
        }
        let cb: Vec<u8> = rc.iter().flat_map(|b| b.as_bytes()).collect();
        let rb: Vec<u8> = rr.iter().flat_map(|b| b.as_bytes()).collect();
        eqb(&format!("softaes_expand_key128 iter={iter}"), &cb, &rb);
        // invert the (equal) schedules and re-compare.
        unsafe {
            c_i128(rc.as_mut_ptr());
            r_i128(rr.as_mut_ptr());
        }
        let cb: Vec<u8> = rc.iter().flat_map(|b| b.as_bytes()).collect();
        let rb: Vec<u8> = rr.iter().flat_map(|b| b.as_bytes()).collect();
        eqb(&format!("softaes_invert_key_schedule128 iter={iter}"), &cb, &rb);

        // 256-bit: 15 round keys.
        let key32 = rng.bytes(32);
        let mut rc = [zero; 15];
        let mut rr = [zero; 15];
        unsafe {
            c_e256(rc.as_mut_ptr(), key32.as_ptr());
            r_e256(rr.as_mut_ptr(), key32.as_ptr());
        }
        let cb: Vec<u8> = rc.iter().flat_map(|b| b.as_bytes()).collect();
        let rb: Vec<u8> = rr.iter().flat_map(|b| b.as_bytes()).collect();
        eqb(&format!("softaes_expand_key256 iter={iter}"), &cb, &rb);
        unsafe {
            c_i256(rc.as_mut_ptr());
            r_i256(rr.as_mut_ptr());
        }
        let cb: Vec<u8> = rc.iter().flat_map(|b| b.as_bytes()).collect();
        let rb: Vec<u8> = rr.iter().flat_map(|b| b.as_bytes()).collect();
        eqb(&format!("softaes_invert_key_schedule256 iter={iter}"), &cb, &rb);
    }
}

// ===========================================================================
// Priority 6 — mlkem768 reference KEM.  From kem_mlkem768_ref.h:
//   int mlkem768_ref_keypair(pk, sk);
//   int mlkem768_ref_seed_keypair(pk, sk, const seed[64]);
//   int mlkem768_ref_enc(ct, ss, const pk);
//   int mlkem768_ref_enc_deterministic(ct, ss, const pk, const seed[32]);
//   int mlkem768_ref_dec(ss, const ct, const sk);
// Sizes: pk=1184, sk=2400, ct=1088, ss=32, keypair seed=64, enc seed=32.
// The deterministic paths are byte-exact; keypair()/enc() use randombytes so
// only return codes are comparable there.
// ===========================================================================

const MLKEM_PK: usize = 1184;
const MLKEM_SK: usize = 2400;
const MLKEM_CT: usize = 1088;
const MLKEM_SS: usize = 32;

type MlSeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type MlEncDet = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int;
type MlDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type MlKp = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type MlEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;

#[test]
fn sodium_mlkem768_deterministic_roundtrip() {
    let (c_skp, r_skp) = sym::<MlSeedKp>("_sodium_mlkem768_ref_seed_keypair");
    let (c_encd, r_encd) = sym::<MlEncDet>("_sodium_mlkem768_ref_enc_deterministic");
    let (c_dec, r_dec) = sym::<MlDec>("_sodium_mlkem768_ref_dec");

    let mut rng = Rng::new(SEED ^ 0x60);
    for iter in 0..40 {
        let kseed = rng.bytes(64);
        // seed_keypair: byte-exact pk/sk.
        let mut pkc = out_buf(MLKEM_PK);
        let mut pkr = out_buf(MLKEM_PK);
        let mut skc = out_buf(MLKEM_SK);
        let mut skr = out_buf(MLKEM_SK);
        unsafe {
            let rc = c_skp(pkc.as_mut_ptr(), skc.as_mut_ptr(), kseed.as_ptr());
            let rr = r_skp(pkr.as_mut_ptr(), skr.as_mut_ptr(), kseed.as_ptr());
            assert_eq!(rc, rr, "mlkem768_ref_seed_keypair rc iter={iter}");
        }
        eqb(&format!("mlkem768_seed_keypair pk iter={iter}"), &pkc, &pkr);
        eqb(&format!("mlkem768_seed_keypair sk iter={iter}"), &skc, &skr);

        // enc_deterministic with a 32-byte coin seed: byte-exact ct/ss.
        let eseed = rng.bytes(32);
        let mut ctc = out_buf(MLKEM_CT);
        let mut ctr = out_buf(MLKEM_CT);
        let mut ssc = out_buf(MLKEM_SS);
        let mut ssr = out_buf(MLKEM_SS);
        unsafe {
            let rc = c_encd(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pkc.as_ptr(), eseed.as_ptr());
            let rr = r_encd(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pkr.as_ptr(), eseed.as_ptr());
            assert_eq!(rc, rr, "mlkem768_ref_enc_deterministic rc iter={iter}");
        }
        eqb(&format!("mlkem768_enc_deterministic ct iter={iter}"), &ctc, &ctr);
        eqb(&format!("mlkem768_enc_deterministic ss iter={iter}"), &ssc, &ssr);

        // dec on the byte-identical ct/sk: ss must be byte-exact and equal the
        // encapsulated ss.
        let mut dsc = out_buf(MLKEM_SS);
        let mut dsr = out_buf(MLKEM_SS);
        unsafe {
            let rc = c_dec(dsc.as_mut_ptr(), ctc.as_ptr(), skc.as_ptr());
            let rr = r_dec(dsr.as_mut_ptr(), ctr.as_ptr(), skr.as_ptr());
            assert_eq!(rc, rr, "mlkem768_ref_dec rc iter={iter}");
        }
        eqb(&format!("mlkem768_dec ss iter={iter}"), &dsc, &dsr);
        eqb(&format!("mlkem768_dec matches enc ss iter={iter}"), &dsc[..MLKEM_SS], &ssc[..MLKEM_SS]);
    }
}

#[test]
fn sodium_mlkem768_random_return_codes() {
    // keypair() and enc() draw from randombytes(); outputs are not comparable,
    // but return codes must match and a decap of C's own ct/sk must succeed.
    let (c_kp, r_kp) = sym::<MlKp>("_sodium_mlkem768_ref_keypair");
    let (c_enc, r_enc) = sym::<MlEnc>("_sodium_mlkem768_ref_enc");
    let (c_dec, _r_dec) = sym::<MlDec>("_sodium_mlkem768_ref_dec");

    for iter in 0..10 {
        let mut pk = out_buf(MLKEM_PK);
        let mut sk = out_buf(MLKEM_SK);
        unsafe {
            let rc = c_kp(pk.as_mut_ptr(), sk.as_mut_ptr());
            let mut pk2 = out_buf(MLKEM_PK);
            let mut sk2 = out_buf(MLKEM_SK);
            let rr = r_kp(pk2.as_mut_ptr(), sk2.as_mut_ptr());
            assert_eq!(rc, rr, "mlkem768_ref_keypair rc iter={iter}");
        }
        let mut ct = out_buf(MLKEM_CT);
        let mut ss = out_buf(MLKEM_SS);
        unsafe {
            let rc = c_enc(ct.as_mut_ptr(), ss.as_mut_ptr(), pk.as_ptr());
            let mut ct2 = out_buf(MLKEM_CT);
            let mut ss2 = out_buf(MLKEM_SS);
            let rr = r_enc(ct2.as_mut_ptr(), ss2.as_mut_ptr(), pk.as_ptr());
            assert_eq!(rc, rr, "mlkem768_ref_enc rc iter={iter}");
            // Cross-check: C's dec of C's ct/sk recovers C's ss.
            let mut ssd = out_buf(MLKEM_SS);
            let rd = c_dec(ssd.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
            assert_eq!(rd, 0, "mlkem768 self-decap iter={iter}");
            eqb(&format!("mlkem768 self-decap ss iter={iter}"), &ssd[..MLKEM_SS], &ss[..MLKEM_SS]);
        }
    }
}

// ===========================================================================
// Priority 7 — *_pick_best_implementation() and runtime/init helpers.
//   int _crypto_*_pick_best_implementation(void);
//   int _sodium_blake2b_pick_best_implementation(void);
//   const void *_sodium_runtime_get_cpu_features(void); (int-returning here)
//   int _sodium_alloc_init(void);
// Compare return codes only, and confirm that calling them does not change a
// subsequent public-API result (a public hash before and after must match).
// ===========================================================================

type PickBest = unsafe extern "C" fn() -> c_int;

#[test]
fn sodium_pick_best_implementation_return_codes() {
    for name in [
        "_crypto_aead_aegis128l_pick_best_implementation",
        "_crypto_aead_aegis256_pick_best_implementation",
        "_crypto_generichash_blake2b_pick_best_implementation",
        "_crypto_ipcrypt_pick_best_implementation",
        "_crypto_onetimeauth_poly1305_pick_best_implementation",
        "_crypto_pwhash_argon2_pick_best_implementation",
        "_crypto_scalarmult_curve25519_pick_best_implementation",
        "_crypto_stream_chacha20_pick_best_implementation",
        "_crypto_stream_salsa20_pick_best_implementation",
        "_sodium_blake2b_pick_best_implementation",
        "_sodium_runtime_get_cpu_features",
        "_sodium_alloc_init",
    ] {
        let (c, r) = sym::<PickBest>(name);
        let (rc, rr) = unsafe { (c(), r()) };
        assert_eq!(rc, rr, "{name} return code");
    }
}

#[test]
fn sodium_pick_best_does_not_disturb_public_api() {
    // Take a public hash, run every pick_best/init, take it again -> unchanged
    // and identical across libraries.  We use crypto_hash_sha256 (32-byte out).
    type Hash = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
    let (c_h, r_h) = sym::<Hash>("crypto_hash_sha256");
    let msg = b"pick-best-implementation must not perturb public results";

    let hash_once = |f: Hash| -> Vec<u8> {
        let mut o = out_buf(32);
        unsafe { f(o.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
        o
    };

    let before_c = hash_once(c_h);
    let before_r = hash_once(r_h);
    eqb("sha256 before pick_best", &before_c, &before_r);

    for name in [
        "_crypto_aead_aegis128l_pick_best_implementation",
        "_crypto_aead_aegis256_pick_best_implementation",
        "_crypto_generichash_blake2b_pick_best_implementation",
        "_crypto_ipcrypt_pick_best_implementation",
        "_crypto_onetimeauth_poly1305_pick_best_implementation",
        "_crypto_pwhash_argon2_pick_best_implementation",
        "_crypto_scalarmult_curve25519_pick_best_implementation",
        "_crypto_stream_chacha20_pick_best_implementation",
        "_crypto_stream_salsa20_pick_best_implementation",
        "_sodium_blake2b_pick_best_implementation",
        "_sodium_alloc_init",
    ] {
        let (c, r) = sym::<PickBest>(name);
        unsafe {
            c();
            r();
        }
    }

    let after_c = hash_once(c_h);
    let after_r = hash_once(r_h);
    eqb("sha256 after pick_best (C stable)", &before_c, &after_c);
    eqb("sha256 after pick_best (R stable)", &before_r, &after_r);
    eqb("sha256 after pick_best (C==R)", &after_c, &after_r);
}

// ===========================================================================
// Priority 8 — internal ed25519 sign/verify entry points.
//   int _crypto_sign_ed25519_detached(unsigned char *sig,
//         unsigned long long *siglen_p, const unsigned char *m,
//         unsigned long long mlen, const unsigned char *sk, int prehashed);
//   int _crypto_sign_ed25519_verify_detached(const unsigned char *sig,
//         const unsigned char *m, unsigned long long mlen,
//         const unsigned char *pk, int prehashed);
//   void _crypto_sign_ed25519_ref10_hinit(crypto_hash_sha512_state *hs,
//         int prehashed);   // SHA-512 state, 208 bytes.
// prehashed=0 => the plain Ed25519 path; prehashed=1 (Ed25519ph) expects m to
// be a 64-byte prehash, so we only drive prehashed=0 with arbitrary messages.
// ===========================================================================

type SignDetached =
    unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8, c_int) -> c_int;
type VerifyDetached = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8, c_int) -> c_int;
type Hinit = unsafe extern "C" fn(*mut u8, c_int);
type SignKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SignSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;

const SHA512_STATE_BYTES: usize = 256; // >= sizeof(crypto_hash_sha512_state) (208)

#[test]
fn sodium_ed25519_detached_internal() {
    let (c_sig, r_sig) = sym::<SignDetached>("_crypto_sign_ed25519_detached");
    let (c_ver, r_ver) = sym::<VerifyDetached>("_crypto_sign_ed25519_verify_detached");
    // Deterministic keypair from a seed (public API) so both libs share keys.
    let (c_skp, _) = sym::<SignSeedKeypair>("crypto_sign_ed25519_seed_keypair");

    let mut rng = Rng::new(SEED ^ 0x80);
    for iter in 0..200 {
        let seed = rng.bytes(32);
        let mut pk = out_buf(32);
        let mut sk = out_buf(64);
        unsafe {
            let rc = c_skp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            assert_eq!(rc, 0, "seed_keypair iter={iter}");
        }

        let mlen = rng.below(256);
        let msg = rng.bytes(mlen.max(1));

        // sign (prehashed = 0): sig + siglen must be byte-exact.
        let mut sigc = out_buf(64);
        let mut sigr = out_buf(64);
        let mut lc: u64 = 0;
        let mut lr: u64 = 0;
        unsafe {
            let rc = c_sig(sigc.as_mut_ptr(), &mut lc, msg.as_ptr(), mlen as u64, sk.as_ptr(), 0);
            let rr = r_sig(sigr.as_mut_ptr(), &mut lr, msg.as_ptr(), mlen as u64, sk.as_ptr(), 0);
            assert_eq!(rc, rr, "ed25519_detached rc iter={iter}");
            assert_eq!(lc, lr, "ed25519_detached siglen iter={iter}");
        }
        eqb(&format!("ed25519_detached sig iter={iter}"), &sigc, &sigr);

        // verify (prehashed = 0) with each library's own sig against the shared
        // pk: both must accept.
        unsafe {
            let vc = c_ver(sigc.as_ptr(), msg.as_ptr(), mlen as u64, pk.as_ptr(), 0);
            let vr = r_ver(sigr.as_ptr(), msg.as_ptr(), mlen as u64, pk.as_ptr(), 0);
            assert_eq!(vc, vr, "ed25519_verify_detached valid rc iter={iter}");
            assert_eq!(vc, 0, "ed25519_verify_detached should accept iter={iter}");
        }

        // Tamper: flip a byte and confirm both reject identically.
        let mut bad = sigc.clone();
        bad[iter % 64] ^= 0x01;
        unsafe {
            let vc = c_ver(bad.as_ptr(), msg.as_ptr(), mlen as u64, pk.as_ptr(), 0);
            let vr = r_ver(bad.as_ptr(), msg.as_ptr(), mlen as u64, pk.as_ptr(), 0);
            assert_eq!(vc, vr, "ed25519_verify_detached tampered rc iter={iter}");
        }
    }
}

#[test]
fn sodium_ed25519_ref10_hinit() {
    // _crypto_sign_ed25519_ref10_hinit initialises a SHA-512 state; the header
    // for the domain separator differs by prehashed flag.  Compare the whole
    // initialised SHA-512 state buffer for prehashed in {0,1}.  We then feed a
    // public crypto_hash_sha512_update/final to confirm the state is usable and
    // identical.
    let (c_hi, r_hi) = sym::<Hinit>("_crypto_sign_ed25519_ref10_hinit");
    type Sha512Update = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
    type Sha512Final = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
    let (c_up, r_up) = sym::<Sha512Update>("crypto_hash_sha512_update");
    let (c_fi, r_fi) = sym::<Sha512Final>("crypto_hash_sha512_final");

    let mut rng = Rng::new(SEED ^ 0x81);
    for prehashed in [0, 1] {
        for iter in 0..50 {
            let mut sc = vec![0u8; SHA512_STATE_BYTES];
            let mut sr = vec![0u8; SHA512_STATE_BYTES];
            unsafe {
                c_hi(sc.as_mut_ptr(), prehashed);
                r_hi(sr.as_mut_ptr(), prehashed);
            }
            eqb(&format!("ed25519_ref10_hinit state prehashed={prehashed} iter={iter}"), &sc[..208], &sr[..208]);

            // Drive the state to a digest and compare.
            let dlen = 1 + rng.below(200);
            let data = rng.bytes(dlen);
            let mut dc = out_buf(64);
            let mut dr = out_buf(64);
            unsafe {
                c_up(sc.as_mut_ptr(), data.as_ptr(), data.len() as u64);
                r_up(sr.as_mut_ptr(), data.as_ptr(), data.len() as u64);
                c_fi(sc.as_mut_ptr(), dc.as_mut_ptr());
                r_fi(sr.as_mut_ptr(), dr.as_mut_ptr());
            }
            eqb(&format!("ed25519_ref10_hinit digest prehashed={prehashed} iter={iter}"), &dc, &dr);
        }
    }
    let _ = sym::<SignKeypair>("crypto_sign_ed25519_keypair"); // ensure symbol exists
}

// ===========================================================================
// Priority 9 — exported vtable globals (function-pointer structs).
// Struct layouts come from the local implementations.h in each subtree:
//   crypto_stream_chacha20_implementation { stream, stream_ietf_ext,
//        stream_xor_ic, stream_ietf_ext_xor_ic }                     (4 fns)
//   crypto_stream_salsa20_implementation  { stream, stream_xor_ic }  (2 fns)
//   crypto_onetimeauth_poly1305_implementation { onetimeauth,
//        onetimeauth_verify, onetimeauth_init, onetimeauth_update,
//        onetimeauth_final }                                         (5 fns)
//   crypto_scalarmult_curve25519_implementation { mult, mult_base }  (2 fns)
//   aegis128l_implementation { encrypt_detached, decrypt_detached }  (2 fns)
//   aegis256_implementation  { encrypt_detached, decrypt_detached }  (2 fns)
//   ipcrypt_implementation { encrypt, decrypt, nd_encrypt, nd_decrypt,
//        ndx_encrypt, ndx_decrypt, pfx_encrypt, pfx_decrypt }        (8 fns)
// The symbol address IS the struct; fetch it as *const Struct.  Assert every
// slot is non-NULL in BOTH libraries, then CALL each pointer with identical
// randomized inputs and compare byte-for-byte.
// ===========================================================================

fn vtable<T>(name: &str) -> (*const T, *const T) {
    // The exported symbol IS the global struct object; the symbol's address is
    // therefore a pointer to the struct.  `Symbol::into_raw().into_raw()` yields
    // the raw object address, which we reinterpret as `*const T`.
    let l = libs();
    let mut n = name.as_bytes().to_vec();
    n.push(0);
    unsafe {
        let cs: libloading::Symbol<*const c_void> = l
            .c
            .get(&n)
            .unwrap_or_else(|e| panic!("C .so missing `{name}`: {e}"));
        let rs: libloading::Symbol<*const c_void> = l
            .r
            .get(&n)
            .unwrap_or_else(|e| panic!("Rust .so missing `{name}`: {e}"));
        let cp = cs.into_raw().into_raw() as *const T;
        let rp = rs.into_raw().into_raw() as *const T;
        (cp, rp)
    }
}

fn assert_all_slots_nonnull(name: &str, cp: *const usize, rp: *const usize, nslots: usize) {
    assert!(!cp.is_null(), "{name}: C vtable is NULL");
    assert!(!rp.is_null(), "{name}: Rust vtable is NULL");
    for i in 0..nslots {
        let cv = unsafe { *cp.add(i) };
        let rv = unsafe { *rp.add(i) };
        assert_ne!(cv, 0, "{name}: C slot {i} is NULL");
        assert_ne!(rv, 0, "{name}: Rust slot {i} is NULL");
    }
}

#[repr(C)]
struct ChaCha20Vt {
    stream: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int,
    stream_ietf_ext: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int,
    stream_xor_ic: unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int,
    stream_ietf_ext_xor_ic:
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u32, *const u8) -> c_int,
}

#[repr(C)]
struct Salsa20Vt {
    stream: unsafe extern "C" fn(*mut u8, u64, *const u8, *const u8) -> c_int,
    stream_xor_ic: unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, u64, *const u8) -> c_int,
}

#[repr(C)]
struct Poly1305Vt {
    onetimeauth: unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> c_int,
    onetimeauth_verify: unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int,
    onetimeauth_init: unsafe extern "C" fn(*mut u8, *const u8) -> c_int,
    onetimeauth_update: unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int,
    onetimeauth_final: unsafe extern "C" fn(*mut u8, *mut u8) -> c_int,
}

#[repr(C)]
struct IpcryptVt {
    encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    nd_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
    nd_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    ndx_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8),
    ndx_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pfx_encrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
    pfx_decrypt: unsafe extern "C" fn(*mut u8, *const u8, *const u8),
}

#[test]
fn vtable_chacha20_ref() {
    let (cp, rp) = vtable::<ChaCha20Vt>("crypto_stream_chacha20_ref_implementation");
    assert_all_slots_nonnull("chacha20_ref", cp as *const usize, rp as *const usize, 4);
    let (cv, rv) = unsafe { (&*cp, &*rp) };
    let mut rng = Rng::new(SEED ^ 0x90);
    for iter in 0..200 {
        let n = rng.bytes(8);
        let n_ietf = rng.bytes(12);
        let k = rng.bytes(32);
        let clen = 1 + rng.below(256);

        let mut oc = out_buf(clen);
        let mut or = out_buf(clen);
        unsafe {
            (cv.stream)(oc.as_mut_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
            (rv.stream)(or.as_mut_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
        }
        eqb(&format!("chacha20 vt.stream iter={iter}"), &oc, &or);

        let mut oc = out_buf(clen);
        let mut or = out_buf(clen);
        unsafe {
            (cv.stream_ietf_ext)(oc.as_mut_ptr(), clen as u64, n_ietf.as_ptr(), k.as_ptr());
            (rv.stream_ietf_ext)(or.as_mut_ptr(), clen as u64, n_ietf.as_ptr(), k.as_ptr());
        }
        eqb(&format!("chacha20 vt.stream_ietf_ext iter={iter}"), &oc, &or);

        let m = rng.bytes(clen);
        let ic = rng.next_u64();
        let mut oc = out_buf(clen);
        let mut or = out_buf(clen);
        unsafe {
            (cv.stream_xor_ic)(oc.as_mut_ptr(), m.as_ptr(), clen as u64, n.as_ptr(), ic, k.as_ptr());
            (rv.stream_xor_ic)(or.as_mut_ptr(), m.as_ptr(), clen as u64, n.as_ptr(), ic, k.as_ptr());
        }
        eqb(&format!("chacha20 vt.stream_xor_ic iter={iter}"), &oc, &or);

        let ic32 = rng.next_u32();
        let mut oc = out_buf(clen);
        let mut or = out_buf(clen);
        unsafe {
            (cv.stream_ietf_ext_xor_ic)(oc.as_mut_ptr(), m.as_ptr(), clen as u64, n_ietf.as_ptr(), ic32, k.as_ptr());
            (rv.stream_ietf_ext_xor_ic)(or.as_mut_ptr(), m.as_ptr(), clen as u64, n_ietf.as_ptr(), ic32, k.as_ptr());
        }
        eqb(&format!("chacha20 vt.stream_ietf_ext_xor_ic iter={iter}"), &oc, &or);
    }
}

#[test]
fn vtable_salsa20_ref() {
    let (cp, rp) = vtable::<Salsa20Vt>("crypto_stream_salsa20_ref_implementation");
    assert_all_slots_nonnull("salsa20_ref", cp as *const usize, rp as *const usize, 2);
    let (cv, rv) = unsafe { (&*cp, &*rp) };
    let mut rng = Rng::new(SEED ^ 0x91);
    for iter in 0..200 {
        let n = rng.bytes(8);
        let k = rng.bytes(32);
        let clen = 1 + rng.below(256);
        let mut oc = out_buf(clen);
        let mut or = out_buf(clen);
        unsafe {
            (cv.stream)(oc.as_mut_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
            (rv.stream)(or.as_mut_ptr(), clen as u64, n.as_ptr(), k.as_ptr());
        }
        eqb(&format!("salsa20 vt.stream iter={iter}"), &oc, &or);

        let m = rng.bytes(clen);
        let ic = rng.next_u64();
        let mut oc = out_buf(clen);
        let mut or = out_buf(clen);
        unsafe {
            (cv.stream_xor_ic)(oc.as_mut_ptr(), m.as_ptr(), clen as u64, n.as_ptr(), ic, k.as_ptr());
            (rv.stream_xor_ic)(or.as_mut_ptr(), m.as_ptr(), clen as u64, n.as_ptr(), ic, k.as_ptr());
        }
        eqb(&format!("salsa20 vt.stream_xor_ic iter={iter}"), &oc, &or);
    }
}

#[test]
fn vtable_poly1305_donna() {
    let (cp, rp) = vtable::<Poly1305Vt>("crypto_onetimeauth_poly1305_donna_implementation");
    assert_all_slots_nonnull("poly1305_donna", cp as *const usize, rp as *const usize, 5);
    let (cv, rv) = unsafe { (&*cp, &*rp) };
    const POLY_STATE: usize = 256;
    let mut rng = Rng::new(SEED ^ 0x92);
    for iter in 0..300 {
        let k = rng.bytes(32);
        let inlen = rng.below(256);
        let inp = rng.bytes(inlen.max(1));

        // one-shot onetimeauth
        let mut mc = out_buf(16);
        let mut mr = out_buf(16);
        unsafe {
            (cv.onetimeauth)(mc.as_mut_ptr(), inp.as_ptr(), inlen as u64, k.as_ptr());
            (rv.onetimeauth)(mr.as_mut_ptr(), inp.as_ptr(), inlen as u64, k.as_ptr());
        }
        eqb(&format!("poly1305 vt.onetimeauth iter={iter}"), &mc, &mr);

        // verify accepts each library's own tag
        unsafe {
            let vc = (cv.onetimeauth_verify)(mc.as_ptr(), inp.as_ptr(), inlen as u64, k.as_ptr());
            let vr = (rv.onetimeauth_verify)(mr.as_ptr(), inp.as_ptr(), inlen as u64, k.as_ptr());
            assert_eq!(vc, vr, "poly1305 vt.verify iter={iter}");
            assert_eq!(vc, 0, "poly1305 vt.verify accept iter={iter}");
        }

        // streaming init/update/final
        let mut sc = vec![0u8; POLY_STATE];
        let mut sr = vec![0u8; POLY_STATE];
        let mut tc = out_buf(16);
        let mut tr = out_buf(16);
        unsafe {
            (cv.onetimeauth_init)(sc.as_mut_ptr(), k.as_ptr());
            (rv.onetimeauth_init)(sr.as_mut_ptr(), k.as_ptr());
            (cv.onetimeauth_update)(sc.as_mut_ptr(), inp.as_ptr(), inlen as u64);
            (rv.onetimeauth_update)(sr.as_mut_ptr(), inp.as_ptr(), inlen as u64);
            (cv.onetimeauth_final)(sc.as_mut_ptr(), tc.as_mut_ptr());
            (rv.onetimeauth_final)(sr.as_mut_ptr(), tr.as_mut_ptr());
        }
        eqb(&format!("poly1305 vt streaming tag iter={iter}"), &tc, &tr);
        eqb(&format!("poly1305 vt streaming==oneshot iter={iter}"), &tc[..16], &mc[..16]);
    }
}

#[test]
fn vtable_ipcrypt_soft() {
    let (cp, rp) = vtable::<IpcryptVt>("ipcrypt_soft_implementation");
    assert_all_slots_nonnull("ipcrypt_soft", cp as *const usize, rp as *const usize, 8);
    let (cv, rv) = unsafe { (&*cp, &*rp) };
    let mut rng = Rng::new(SEED ^ 0x93);
    for iter in 0..300 {
        let k16 = rng.bytes(16);
        let k32 = rng.bytes(32);
        let inp16 = rng.bytes(16);
        let t8 = rng.bytes(8);
        let t16 = rng.bytes(16);

        // encrypt/decrypt: out16,in16,k16
        let mut oc = out_buf(16);
        let mut or = out_buf(16);
        unsafe {
            (cv.encrypt)(oc.as_mut_ptr(), inp16.as_ptr(), k16.as_ptr());
            (rv.encrypt)(or.as_mut_ptr(), inp16.as_ptr(), k16.as_ptr());
        }
        eqb(&format!("ipcrypt vt.encrypt iter={iter}"), &oc, &or);
        let mut dc = out_buf(16);
        let mut dr = out_buf(16);
        unsafe {
            (cv.decrypt)(dc.as_mut_ptr(), oc.as_ptr(), k16.as_ptr());
            (rv.decrypt)(dr.as_mut_ptr(), or.as_ptr(), k16.as_ptr());
        }
        eqb(&format!("ipcrypt vt.decrypt iter={iter}"), &dc, &dr);

        // nd: out24,in16,t8,k16 ; nd_decrypt out16,in24,k16
        let mut oc = out_buf(24);
        let mut or = out_buf(24);
        unsafe {
            (cv.nd_encrypt)(oc.as_mut_ptr(), inp16.as_ptr(), t8.as_ptr(), k16.as_ptr());
            (rv.nd_encrypt)(or.as_mut_ptr(), inp16.as_ptr(), t8.as_ptr(), k16.as_ptr());
        }
        eqb(&format!("ipcrypt vt.nd_encrypt iter={iter}"), &oc, &or);
        let mut dc = out_buf(16);
        let mut dr = out_buf(16);
        unsafe {
            (cv.nd_decrypt)(dc.as_mut_ptr(), oc.as_ptr(), k16.as_ptr());
            (rv.nd_decrypt)(dr.as_mut_ptr(), or.as_ptr(), k16.as_ptr());
        }
        eqb(&format!("ipcrypt vt.nd_decrypt iter={iter}"), &dc, &dr);

        // ndx: out32,in16,t16,k32 ; ndx_decrypt out16,in32,k32
        let mut oc = out_buf(32);
        let mut or = out_buf(32);
        unsafe {
            (cv.ndx_encrypt)(oc.as_mut_ptr(), inp16.as_ptr(), t16.as_ptr(), k32.as_ptr());
            (rv.ndx_encrypt)(or.as_mut_ptr(), inp16.as_ptr(), t16.as_ptr(), k32.as_ptr());
        }
        eqb(&format!("ipcrypt vt.ndx_encrypt iter={iter}"), &oc, &or);
        let mut dc = out_buf(16);
        let mut dr = out_buf(16);
        unsafe {
            (cv.ndx_decrypt)(dc.as_mut_ptr(), oc.as_ptr(), k32.as_ptr());
            (rv.ndx_decrypt)(dr.as_mut_ptr(), or.as_ptr(), k32.as_ptr());
        }
        eqb(&format!("ipcrypt vt.ndx_decrypt iter={iter}"), &dc, &dr);

        // pfx: out16,in16,k32
        let mut oc = out_buf(16);
        let mut or = out_buf(16);
        unsafe {
            (cv.pfx_encrypt)(oc.as_mut_ptr(), inp16.as_ptr(), k32.as_ptr());
            (rv.pfx_encrypt)(or.as_mut_ptr(), inp16.as_ptr(), k32.as_ptr());
        }
        eqb(&format!("ipcrypt vt.pfx_encrypt iter={iter}"), &oc, &or);
        let mut dc = out_buf(16);
        let mut dr = out_buf(16);
        unsafe {
            (cv.pfx_decrypt)(dc.as_mut_ptr(), oc.as_ptr(), k32.as_ptr());
            (rv.pfx_decrypt)(dr.as_mut_ptr(), or.as_ptr(), k32.as_ptr());
        }
        eqb(&format!("ipcrypt vt.pfx_decrypt iter={iter}"), &dc, &dr);
    }
}

#[test]
fn vtable_aegis_soft_slots() {
    // aegis128l/aegis256 soft vtables: assert both slots non-NULL in both libs.
    // We drive them through the public crypto_aead_aegis* API elsewhere; here we
    // exercise the detached function pointers directly with identical inputs.
    #[repr(C)]
    struct AegisVt {
        encrypt_detached: unsafe extern "C" fn(
            *mut u8, *mut u8, usize, *const u8, usize, *const u8, usize, *const u8, *const u8,
        ) -> c_int,
        decrypt_detached: unsafe extern "C" fn(
            *mut u8, *const u8, usize, *const u8, usize, *const u8, usize, *const u8, *const u8,
        ) -> c_int,
    }
    for (name, npub_len, key_len, mac_len) in [
        ("aegis128l_soft_implementation", 16usize, 16usize, 16usize),
        ("aegis256_soft_implementation", 32usize, 32usize, 16usize),
    ] {
        let (cp, rp) = vtable::<AegisVt>(name);
        assert_all_slots_nonnull(name, cp as *const usize, rp as *const usize, 2);
        let (cv, rv) = unsafe { (&*cp, &*rp) };
        let mut rng = Rng::new(SEED ^ 0x94 ^ name.len() as u64);
        for iter in 0..150 {
            let mlen = rng.below(200);
            let adlen = rng.below(64);
            let m = rng.bytes(mlen.max(1));
            let ad = rng.bytes(adlen.max(1));
            let npub = rng.bytes(npub_len);
            let key = rng.bytes(key_len);
            let adp = if adlen == 0 { ptr::null() } else { ad.as_ptr() };
            let mp = if mlen == 0 { ptr::null() } else { m.as_ptr() };

            let mut cc = out_buf(mlen);
            let mut cr = out_buf(mlen);
            let mut macc = out_buf(mac_len);
            let mut macr = out_buf(mac_len);
            unsafe {
                let rc = (cv.encrypt_detached)(cc.as_mut_ptr(), macc.as_mut_ptr(), mac_len, mp, mlen, adp, adlen, npub.as_ptr(), key.as_ptr());
                let rr = (rv.encrypt_detached)(cr.as_mut_ptr(), macr.as_mut_ptr(), mac_len, mp, mlen, adp, adlen, npub.as_ptr(), key.as_ptr());
                assert_eq!(rc, rr, "{name} encrypt_detached rc iter={iter}");
            }
            eqb(&format!("{name} encrypt_detached ct iter={iter}"), &cc, &cr);
            eqb(&format!("{name} encrypt_detached mac iter={iter}"), &macc, &macr);

            // decrypt_detached back to plaintext.
            let ccp = if mlen == 0 { ptr::null() } else { cc.as_ptr() };
            let mut pc = out_buf(mlen);
            let mut pr = out_buf(mlen);
            unsafe {
                let rc = (cv.decrypt_detached)(pc.as_mut_ptr(), ccp, mlen, macc.as_ptr(), mac_len, adp, adlen, npub.as_ptr(), key.as_ptr());
                let rr = (rv.decrypt_detached)(pr.as_mut_ptr(), ccp, mlen, macc.as_ptr(), mac_len, adp, adlen, npub.as_ptr(), key.as_ptr());
                assert_eq!(rc, rr, "{name} decrypt_detached rc iter={iter}");
                assert_eq!(rc, 0, "{name} decrypt_detached accept iter={iter}");
            }
            eqb(&format!("{name} decrypt_detached pt iter={iter}"), &pc, &pr);
        }
    }
}

#[test]
fn vtable_curve25519_ref10_slots() {
    #[repr(C)]
    struct X25519Vt {
        mult: unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int,
        mult_base: unsafe extern "C" fn(*mut u8, *const u8) -> c_int,
    }
    let (cp, rp) = vtable::<X25519Vt>("crypto_scalarmult_curve25519_ref10_implementation");
    assert_all_slots_nonnull("curve25519_ref10", cp as *const usize, rp as *const usize, 2);
    let (cv, rv) = unsafe { (&*cp, &*rp) };
    let mut rng = Rng::new(SEED ^ 0x95);
    for iter in 0..300 {
        let n = rng.bytes(32);
        // mult_base: q = n * basepoint
        let mut bc = out_buf(32);
        let mut br = out_buf(32);
        unsafe {
            let rc = (cv.mult_base)(bc.as_mut_ptr(), n.as_ptr());
            let rr = (rv.mult_base)(br.as_mut_ptr(), n.as_ptr());
            assert_eq!(rc, rr, "curve25519 vt.mult_base rc iter={iter}");
        }
        eqb(&format!("curve25519 vt.mult_base iter={iter}"), &bc, &br);
        // mult: q = n * p, using the freshly computed public point.
        let mut qc = out_buf(32);
        let mut qr = out_buf(32);
        unsafe {
            let rc = (cv.mult)(qc.as_mut_ptr(), n.as_ptr(), bc.as_ptr());
            let rr = (rv.mult)(qr.as_mut_ptr(), n.as_ptr(), br.as_ptr());
            assert_eq!(rc, rr, "curve25519 vt.mult rc iter={iter}");
        }
        eqb(&format!("curve25519 vt.mult iter={iter}"), &qc, &qr);
    }
}

// ===========================================================================
// Priority 10 — escrypt / argon2 helpers.
//
// escrypt (from crypto_scrypt.h / pbkdf2-sha256.h):
//   const uint8_t *escrypt_parse_setting(const uint8_t *setting,
//        uint32_t *N_log2, uint32_t *r, uint32_t *p);
//   uint8_t *escrypt_gensalt_r(uint32_t N_log2, uint32_t r, uint32_t p,
//        const uint8_t *src, size_t srclen, uint8_t *buf, size_t buflen);
//   void escrypt_PBKDF2_SHA256(passwd, passwdlen, salt, saltlen, c, buf, dkLen);
//   int escrypt_kdf_nosse(local, passwd, passwdlen, salt, saltlen,
//        uint64_t N, uint32_t r, uint32_t p, buf, buflen);
//   int  escrypt_init_local(escrypt_local_t*);  int escrypt_free_local(...);
//   void *escrypt_alloc_region(region*, size_t); int escrypt_free_region(...);
// The escrypt_local_t / escrypt_region_t is { void*base, void*aligned, size_t }
// = 24 bytes; we allocate a generous zeroed buffer.
// ===========================================================================

const ESCRYPT_LOCAL_BYTES: usize = 64; // >= sizeof(escrypt_region_t) (24)

type EscParse = unsafe extern "C" fn(*const u8, *mut u32, *mut u32, *mut u32) -> *const u8;
type EscGensalt =
    unsafe extern "C" fn(u32, u32, u32, *const u8, usize, *mut u8, usize) -> *mut u8;
type EscPbkdf2 = unsafe extern "C" fn(*const u8, usize, *const u8, usize, u64, *mut u8, usize);
type EscKdf = unsafe extern "C" fn(
    *mut u8, *const u8, usize, *const u8, usize, u64, u32, u32, *mut u8, usize,
) -> c_int;
type EscInitLocal = unsafe extern "C" fn(*mut u8) -> c_int;
type EscFreeLocal = unsafe extern "C" fn(*mut u8) -> c_int;
type EscAllocRegion = unsafe extern "C" fn(*mut u8, usize) -> *mut c_void;
type EscFreeRegion = unsafe extern "C" fn(*mut u8) -> c_int;

#[test]
fn sodium_escrypt_parse_setting() {
    let (c, r) = sym::<EscParse>("_sodium_escrypt_parse_setting");
    // Valid scrypt settings string "$7$" + params.  Also feed random/garbage.
    let mut rng = Rng::new(SEED ^ 0xA0);
    let mut cases: Vec<Vec<u8>> = vec![
        b"$7$C6..../....".to_vec(),
        b"$7$06..../....abc".to_vec(),
        b"invalid".to_vec(),
        b"".to_vec(),
        b"$7$".to_vec(),
    ];
    for _ in 0..200 {
        let vlen = 1 + rng.below(20);
        let mut v = rng.bytes(vlen);
        v.push(0); // NUL-terminate-ish; parse reads bytewise
        cases.push(v);
    }
    for (i, s0) in cases.iter().enumerate() {
        let mut s = s0.clone();
        s.push(0);
        let (mut cn, mut cr_, mut cp) = (0u32, 0u32, 0u32);
        let (mut rn, mut rr_, mut rp) = (0u32, 0u32, 0u32);
        let (cret, rret) = unsafe {
            (
                c(s.as_ptr(), &mut cn, &mut cr_, &mut cp),
                r(s.as_ptr(), &mut rn, &mut rr_, &mut rp),
            )
        };
        // Return is a pointer INTO the input (or NULL on failure); compare the
        // NULL-ness and, when non-NULL, the byte offset into the input.
        let coff = if cret.is_null() { usize::MAX } else { cret as usize - s.as_ptr() as usize };
        let roff = if rret.is_null() { usize::MAX } else { rret as usize - s.as_ptr() as usize };
        assert_eq!(coff, roff, "escrypt_parse_setting offset case={i}");
        if !cret.is_null() {
            assert_eq!((cn, cr_, cp), (rn, rr_, rp), "escrypt_parse_setting params case={i}");
        }
    }
}

#[test]
fn sodium_escrypt_gensalt_r() {
    let (c, r) = sym::<EscGensalt>("_sodium_escrypt_gensalt_r");
    let mut rng = Rng::new(SEED ^ 0xA1);
    for iter in 0..200 {
        let n_log2 = 1 + (rng.below(20) as u32);
        let rr_p = 1 + (rng.below(16) as u32);
        let p = 1 + (rng.below(4) as u32);
        let srclen = rng.below(32);
        let src = rng.bytes(srclen.max(1));
        let mut bc = out_buf(96);
        let mut br = out_buf(96);
        let (cret, rret) = unsafe {
            (
                c(n_log2, rr_p, p, src.as_ptr(), srclen, bc.as_mut_ptr(), 96),
                r(n_log2, rr_p, p, src.as_ptr(), srclen, br.as_mut_ptr(), 96),
            )
        };
        assert_eq!(cret.is_null(), rret.is_null(), "escrypt_gensalt_r null iter={iter}");
        eqb(&format!("escrypt_gensalt_r buf iter={iter}"), &bc, &br);
    }
}

#[test]
fn sodium_escrypt_pbkdf2_sha256() {
    let (c, r) = sym::<EscPbkdf2>("_sodium_escrypt_PBKDF2_SHA256");
    let mut rng = Rng::new(SEED ^ 0xA2);
    for iter in 0..300 {
        let pwlen = rng.below(64);
        let saltlen = rng.below(64);
        let pw = rng.bytes(pwlen.max(1));
        let salt = rng.bytes(saltlen.max(1));
        let rounds = 1 + rng.below(64) as u64;
        let dklen = 1 + rng.below(96);
        let mut oc = out_buf(dklen);
        let mut or = out_buf(dklen);
        unsafe {
            c(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, rounds, oc.as_mut_ptr(), dklen);
            r(pw.as_ptr(), pwlen, salt.as_ptr(), saltlen, rounds, or.as_mut_ptr(), dklen);
        }
        eqb(&format!("escrypt_PBKDF2_SHA256 iter={iter} rounds={rounds} dklen={dklen}"), &oc, &or);
    }
}

#[test]
fn sodium_escrypt_kdf_nosse() {
    let (c_kdf, r_kdf) = sym::<EscKdf>("_sodium_escrypt_kdf_nosse");
    let (c_init, r_init) = sym::<EscInitLocal>("_sodium_escrypt_init_local");
    let (c_free, r_free) = sym::<EscFreeLocal>("_sodium_escrypt_free_local");

    let mut rng = Rng::new(SEED ^ 0xA3);
    for iter in 0..30 {
        // Keep N tiny so runtime stays small (N must be a power of two).
        let n: u64 = 1 << (1 + rng.below(5)); // 2..=32
        let rp: u32 = 1 + rng.below(4) as u32;
        let p: u32 = 1 + rng.below(2) as u32;
        let pw = rng.bytes(16);
        let salt = rng.bytes(16);
        let dklen = 32;

        let mut lc = vec![0u8; ESCRYPT_LOCAL_BYTES];
        let mut lr = vec![0u8; ESCRYPT_LOCAL_BYTES];
        unsafe {
            let ic = c_init(lc.as_mut_ptr());
            let ir = r_init(lr.as_mut_ptr());
            assert_eq!(ic, ir, "escrypt_init_local iter={iter}");
        }
        let mut oc = out_buf(dklen);
        let mut or = out_buf(dklen);
        unsafe {
            let rc = c_kdf(lc.as_mut_ptr(), pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rp, p, oc.as_mut_ptr(), dklen);
            let rr = r_kdf(lr.as_mut_ptr(), pw.as_ptr(), pw.len(), salt.as_ptr(), salt.len(), n, rp, p, or.as_mut_ptr(), dklen);
            assert_eq!(rc, rr, "escrypt_kdf_nosse rc iter={iter} N={n} r={rp} p={p}");
        }
        eqb(&format!("escrypt_kdf_nosse out iter={iter} N={n} r={rp} p={p}"), &oc, &or);
        unsafe {
            let fc = c_free(lc.as_mut_ptr());
            let fr = r_free(lr.as_mut_ptr());
            assert_eq!(fc, fr, "escrypt_free_local iter={iter}");
        }
    }
}

#[test]
fn sodium_escrypt_region_helpers_nullness() {
    // alloc/init/free helpers: compare only return-code / null-ness (they touch
    // real allocations, so pointer values are not comparable).
    let (c_ir, r_ir) = sym::<EscInitLocal>("_sodium_escrypt_init_local");
    let (c_fr, r_fr) = sym::<EscFreeLocal>("_sodium_escrypt_free_local");
    let (c_ar, r_ar) = sym::<EscAllocRegion>("_sodium_escrypt_alloc_region");
    let (c_freg, r_freg) = sym::<EscFreeRegion>("_sodium_escrypt_free_region");

    for iter in 0..20 {
        let mut lc = vec![0u8; ESCRYPT_LOCAL_BYTES];
        let mut lr = vec![0u8; ESCRYPT_LOCAL_BYTES];
        unsafe {
            assert_eq!(c_ir(lc.as_mut_ptr()), r_ir(lr.as_mut_ptr()), "init_local iter={iter}");
            assert_eq!(c_fr(lc.as_mut_ptr()), r_fr(lr.as_mut_ptr()), "free_local iter={iter}");
        }
        let size = 4096 * (1 + iter);
        let mut rc_ = vec![0u8; ESCRYPT_LOCAL_BYTES];
        let mut rr_ = vec![0u8; ESCRYPT_LOCAL_BYTES];
        unsafe {
            let pc = c_ar(rc_.as_mut_ptr(), size);
            let pr = r_ar(rr_.as_mut_ptr(), size);
            assert_eq!(pc.is_null(), pr.is_null(), "alloc_region null iter={iter}");
            let dc = c_freg(rc_.as_mut_ptr());
            let dr = r_freg(rr_.as_mut_ptr());
            assert_eq!(dc, dr, "free_region iter={iter}");
        }
    }
}

// ---------------------------------------------------------------------------
// argon2 encode / decode / validate (pure functions over an argon2_context).
//   int argon2_encode_string(char *dst, size_t dst_len, argon2_context *ctx,
//                            argon2_type type);
//   int argon2_decode_string(argon2_context *ctx, const char *str,
//                            argon2_type type);
//   int argon2_validate_inputs(const argon2_context *context);
// argon2_context layout (all little-endian, natural alignment => 8-byte ptrs):
//   u8* out; u32 outlen; u8* pwd; u32 pwdlen; u8* salt; u32 saltlen;
//   u8* secret; u32 secretlen; u8* ad; u32 adlen;
//   u32 t_cost; u32 m_cost; u32 lanes; u32 threads; u32 flags;
// ---------------------------------------------------------------------------

#[repr(C)]
struct Argon2Ctx {
    out: *mut u8,
    outlen: u32,
    pwd: *mut u8,
    pwdlen: u32,
    salt: *mut u8,
    saltlen: u32,
    secret: *mut u8,
    secretlen: u32,
    ad: *mut u8,
    adlen: u32,
    t_cost: u32,
    m_cost: u32,
    lanes: u32,
    threads: u32,
    flags: u32,
}

type Argon2Validate = unsafe extern "C" fn(*const Argon2Ctx) -> c_int;
type Argon2Encode = unsafe extern "C" fn(*mut u8, usize, *mut Argon2Ctx, c_int) -> c_int;
type Argon2Decode = unsafe extern "C" fn(*mut Argon2Ctx, *const u8, c_int) -> c_int;

const ARGON2_TYPE_I: c_int = 1;
const ARGON2_TYPE_ID: c_int = 2;

#[test]
fn sodium_argon2_validate_inputs() {
    let (c, r) = sym::<Argon2Validate>("_sodium_argon2_validate_inputs");
    let mut rng = Rng::new(SEED ^ 0xA4);
    for iter in 0..500 {
        // Build randomized-but-structured contexts to hit both OK and error
        // codes.  Buffers are pinned for the duration of the call.
        let outlen = rng.below(80) as u32;
        let pwdlen = rng.below(80) as u32;
        let saltlen = rng.below(80) as u32;
        let mut out = vec![0u8; outlen.max(1) as usize];
        let mut pwd = vec![0u8; pwdlen.max(1) as usize];
        let mut salt = vec![0u8; saltlen.max(1) as usize];
        let mut ctx = Argon2Ctx {
            out: out.as_mut_ptr(),
            outlen,
            pwd: pwd.as_mut_ptr(),
            pwdlen,
            salt: salt.as_mut_ptr(),
            saltlen,
            secret: ptr::null_mut(),
            secretlen: 0,
            ad: ptr::null_mut(),
            adlen: 0,
            t_cost: 1 + rng.below(4) as u32,
            m_cost: 8 * (1 + rng.below(16) as u32),
            lanes: 1 + rng.below(4) as u32,
            threads: 1 + rng.below(4) as u32,
            flags: 0,
        };
        unsafe {
            let rc = c(&ctx);
            let rr = r(&ctx);
            assert_eq!(rc, rr, "argon2_validate_inputs iter={iter} outlen={outlen} saltlen={saltlen}");
        }
        // Keep buffers alive.
        let _ = (&mut ctx, &out, &pwd, &salt);
    }
}

#[test]
fn sodium_argon2_encode_decode_roundtrip() {
    let (c_enc, r_enc) = sym::<Argon2Encode>("_sodium_argon2_encode_string");
    let (c_dec, r_dec) = sym::<Argon2Decode>("_sodium_argon2_decode_string");

    let mut rng = Rng::new(SEED ^ 0xA5);
    for ty in [ARGON2_TYPE_I, ARGON2_TYPE_ID] {
        for iter in 0..200 {
            let outlen = 16 + rng.below(32) as u32; // >= ARGON2_MIN_OUTLEN
            let saltlen = 8 + rng.below(24) as u32; // >= ARGON2_MIN_SALT_LENGTH
            let mut out = rng.bytes(outlen as usize);
            let mut salt = rng.bytes(saltlen as usize);
            let mut ctx = Argon2Ctx {
                out: out.as_mut_ptr(),
                outlen,
                pwd: ptr::null_mut(),
                pwdlen: 0,
                salt: salt.as_mut_ptr(),
                saltlen,
                secret: ptr::null_mut(),
                secretlen: 0,
                ad: ptr::null_mut(),
                adlen: 0,
                t_cost: 1 + rng.below(4) as u32,
                m_cost: 8 * (1 + rng.below(16) as u32),
                lanes: 1,
                threads: 1,
                flags: 0,
            };

            let mut ec = vec![0u8; 512];
            let mut er = vec![0u8; 512];
            let (rc, rr) = unsafe {
                (
                    c_enc(ec.as_mut_ptr(), ec.len(), &mut ctx, ty),
                    r_enc(er.as_mut_ptr(), er.len(), &mut ctx, ty),
                )
            };
            assert_eq!(rc, rr, "argon2_encode_string rc ty={ty} iter={iter}");
            eqb(&format!("argon2_encode_string ty={ty} iter={iter}"), &ec, &er);

            if rc != 0 {
                continue;
            }
            // Decode the (identical) encoded string with matching max lengths.
            let mut oc = vec![0u8; outlen as usize];
            let mut or = vec![0u8; outlen as usize];
            let mut sc = vec![0u8; saltlen as usize];
            let mut sr = vec![0u8; saltlen as usize];
            let mut cc = Argon2Ctx {
                out: oc.as_mut_ptr(),
                outlen,
                pwd: ptr::null_mut(),
                pwdlen: 0,
                salt: sc.as_mut_ptr(),
                saltlen,
                secret: ptr::null_mut(),
                secretlen: 0,
                ad: ptr::null_mut(),
                adlen: 0,
                t_cost: 0,
                m_cost: 0,
                lanes: 0,
                threads: 0,
                flags: 0,
            };
            let mut cr = Argon2Ctx {
                out: or.as_mut_ptr(),
                outlen,
                pwd: ptr::null_mut(),
                pwdlen: 0,
                salt: sr.as_mut_ptr(),
                saltlen,
                secret: ptr::null_mut(),
                secretlen: 0,
                ad: ptr::null_mut(),
                adlen: 0,
                t_cost: 0,
                m_cost: 0,
                lanes: 0,
                threads: 0,
                flags: 0,
            };
            let (dc, dr) = unsafe {
                (
                    c_dec(&mut cc, ec.as_ptr(), ty),
                    r_dec(&mut cr, er.as_ptr(), ty),
                )
            };
            assert_eq!(dc, dr, "argon2_decode_string rc ty={ty} iter={iter}");
            if dc == 0 {
                assert_eq!(cc.t_cost, cr.t_cost, "decode t_cost ty={ty} iter={iter}");
                assert_eq!(cc.m_cost, cr.m_cost, "decode m_cost ty={ty} iter={iter}");
                assert_eq!(cc.lanes, cr.lanes, "decode lanes ty={ty} iter={iter}");
                assert_eq!(cc.outlen, cr.outlen, "decode outlen ty={ty} iter={iter}");
                assert_eq!(cc.saltlen, cr.saltlen, "decode saltlen ty={ty} iter={iter}");
                eqb(&format!("argon2_decode out ty={ty} iter={iter}"), &oc, &or);
                eqb(&format!("argon2_decode salt ty={ty} iter={iter}"), &sc, &sr);
            }
        }
    }
}
