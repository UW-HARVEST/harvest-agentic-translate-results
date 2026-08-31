//! Area 7 — `crypto_scalarmult` gap coverage plus the three area-wide rows.
//!
//! Complements `a7_scalarmult.rs`.  Adds:
//!   * the exported `crypto_scalarmult_curve25519_ref10_implementation` data
//!     symbol (the 10th "accessor" of `configs_7.md` 7.28) and, through it,
//!     direct differential coverage of the two ref10 primitives — which is the
//!     only way to observe that `crypto_scalarmult_curve25519`'s post-hoc
//!     all-zero guard (`errors_7.md` 7.9) is unreachable with ref10;
//!   * an exhaustive "`crypto_scalarmult_curve25519_base` can never fail"
//!     sweep (`errors_7.md` 7.11 / 7.12) — the `_base` entry point bypasses the
//!     wrapper entirely, so not even `n = 0` can make it return `-1`;
//!   * per-row, multi-pattern pre-fill comparison of the caller's `q` after
//!     every ed25519 / ristretto255 rejection (`errors_7.md` 7.13–7.33), since
//!     all of those stage the clamped scalar in the caller's buffer
//!     (`unsigned char *t = q;`);
//!   * `L - 1` / `L` / `L + 1` / `2L` / `8L` scalar families, the identity
//!     points and non-canonical field elements on all six entry points;
//!   * `configs_7.md` 7.129 (`crypto_kx` vs `crypto_box_beforenm` consistency)
//!     and 7.130 (build-configuration invariants, asserted through their
//!     observable consequences).
mod common;
use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, CStr};

type Mult = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type MultBase = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type IntFn = unsafe extern "C" fn() -> c_int;

// ------------------------------------------------------------------ helpers

fn hx(s: &str) -> Vec<u8> {
    assert!(s.len() % 2 == 0, "odd hex length");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}
fn h32(s: &str) -> [u8; 32] {
    let v = hx(s);
    let mut a = [0u8; 32];
    a.copy_from_slice(&v);
    a
}

/// A family of distinctive pre-fill patterns for the caller's `q`.  Using more
/// than one makes "q untouched" observable independently of the value the
/// implementation would have written.
const FILLS: [u8; 4] = [0x00, 0x5a, 0xa5, 0xff];

fn fill_of(tag: u8) -> [u8; 32] {
    let mut v = [0u8; 32];
    for (i, b) in v.iter_mut().enumerate() {
        *b = tag ^ (i as u8).wrapping_mul(0x1f);
    }
    v
}

/// A (C, Rust) pair of 3-argument scalarmult functions.
struct F3 {
    name: String,
    c: Symbol<'static, Mult>,
    r: Symbol<'static, Mult>,
}
impl F3 {
    fn new(name: &str) -> Self {
        let (c, r) = both::<Mult>(name);
        F3 { name: name.to_string(), c, r }
    }
    /// Differential call with the output buffer pre-filled from `tag`.
    /// Returns `(rc, q, prefill)`.
    fn run(&self, n: &[u8], p: &[u8], tag: u8) -> (c_int, [u8; 32], [u8; 32]) {
        assert_eq!(n.len(), 32);
        assert_eq!(p.len(), 32);
        let f = fill_of(tag);
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(&f);
        qr[..32].copy_from_slice(&f);
        let rc = unsafe { (self.c)(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rr = unsafe { (self.r)(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eqi(&format!("{}(n={}, p={}) rc", self.name, hex(n), hex(p)), rc, rr);
        eqb(&format!("{}(n={}, p={}) q", self.name, hex(n), hex(p)), &qc, &qr);
        check_pad(&format!("{}(C)", self.name), &qc, 32);
        check_pad(&format!("{}(Rust)", self.name), &qr, 32);
        let mut q = [0u8; 32];
        q.copy_from_slice(&qc[..32]);
        (rc, q, f)
    }
    /// Assert that the call fails *early*: the caller's `q` is byte-identical
    /// to the pre-fill in both implementations, for every pattern.
    fn expect_early_reject(&self, n: &[u8], p: &[u8], what: &str) {
        for tag in FILLS {
            let (rc, q, f) = self.run(n, p, tag);
            eqi(&format!("{} [{what}] rc", self.name), rc, -1);
            eqb(
                &format!("{} [{what}] early reject leaves q untouched", self.name),
                &q,
                &f,
            );
        }
    }
    /// Assert that the call fails *late*: `q` holds `want` regardless of the
    /// pre-fill, in both implementations.
    fn expect_late_reject(&self, n: &[u8], p: &[u8], want: &[u8], what: &str) {
        for tag in FILLS {
            let (rc, q, f) = self.run(n, p, tag);
            eqi(&format!("{} [{what}] rc", self.name), rc, -1);
            eqb(
                &format!("{} [{what}] late reject wrote q", self.name),
                &q,
                want,
            );
            if want != f {
                assert_ne!(q, f, "{} [{what}]: q must have been overwritten", self.name);
            }
        }
    }
}

/// A (C, Rust) pair of 2-argument `_base` functions.
struct F2 {
    name: String,
    c: Symbol<'static, MultBase>,
    r: Symbol<'static, MultBase>,
}
impl F2 {
    fn new(name: &str) -> Self {
        let (c, r) = both::<MultBase>(name);
        F2 { name: name.to_string(), c, r }
    }
    fn run(&self, n: &[u8], tag: u8) -> (c_int, [u8; 32], [u8; 32]) {
        assert_eq!(n.len(), 32);
        let f = fill_of(tag);
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(&f);
        qr[..32].copy_from_slice(&f);
        let rc = unsafe { (self.c)(qc.as_mut_ptr(), n.as_ptr()) };
        let rr = unsafe { (self.r)(qr.as_mut_ptr(), n.as_ptr()) };
        eqi(&format!("{}(n={}) rc", self.name, hex(n)), rc, rr);
        eqb(&format!("{}(n={}) q", self.name, hex(n)), &qc, &qr);
        check_pad(&format!("{}(C)", self.name), &qc, 32);
        check_pad(&format!("{}(Rust)", self.name), &qr, 32);
        let mut q = [0u8; 32];
        q.copy_from_slice(&qc[..32]);
        (rc, q, f)
    }
    fn expect_late_reject(&self, n: &[u8], want: &[u8], what: &str) {
        for tag in FILLS {
            let (rc, q, f) = self.run(n, tag);
            eqi(&format!("{} [{what}] rc", self.name), rc, -1);
            eqb(&format!("{} [{what}] late reject wrote q", self.name), &q, want);
            if want != f {
                assert_ne!(q, f, "{} [{what}]: q must have been overwritten", self.name);
            }
        }
    }
    fn expect_ok(&self, n: &[u8], what: &str) -> [u8; 32] {
        let mut out = None;
        for tag in FILLS {
            let (rc, q, f) = self.run(n, tag);
            eqi(&format!("{} [{what}] rc must be 0", self.name), rc, 0);
            assert_ne!(q, f, "{} [{what}]: q must have been written", self.name);
            if let Some(prev) = out {
                assert_eq!(prev, q, "{} [{what}]: pre-fill leaked into output", self.name);
            }
            out = Some(q);
        }
        out.unwrap()
    }
}

// ------------------------------------------------------------------ constants

/// `L = 2^252 + 27742317777372353535851937790883648493`, little-endian.
const L_HEX: &str = "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";
const X25519_BASEPOINT: &str =
    "0900000000000000000000000000000000000000000000000000000000000000";
const ED25519_BASEPOINT: &str =
    "5866666666666666666666666666666666666666666666666666666666666666";
const ED_IDENTITY: &str =
    "0100000000000000000000000000000000000000000000000000000000000000";
const ZERO32: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// `2^255 - 19` = the canonical field modulus `p`, little-endian: the smallest
/// non-canonical field element.
const P_HEX: &str = "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f";

/// The 7 curve25519 small-order encodings on the ref10 blocklist
/// (`x25519_ref10.c:19-51`).  Together with the `| 0x80` variants exercised
/// below these are the complete set of curve25519 small-order encodings
/// reachable through the 32-byte public API.
const CURVE_SMALL_ORDER: [&str; 7] = [
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0100000000000000000000000000000000000000000000000000000000000000",
    "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800",
    "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
];

/// The 8 small-order ed25519 point encodings.
const ED_SMALL_ORDER: [&str; 8] = [
    "0100000000000000000000000000000000000000000000000000000000000000",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000080",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
];

fn clamp_curve(n: &mut [u8; 32]) {
    n[0] &= 248;
    n[31] &= 127;
    n[31] |= 64;
}

/// `k * L`, little-endian.  `k` must be `<= 7`: `L` is just above `2^252`, so
/// `8L` already exceeds `2^255` and the unconditional `t[31] &= 127` would
/// destroy the "multiple of L" property.  See `mul_l_overflow_is_not_a_multiple`
/// below, which pins that boundary down.
fn mul_l(k: u16) -> [u8; 32] {
    assert!((1..=7).contains(&k), "kL must stay below 2^255");
    let l = h32(L_HEX);
    let mut out = [0u8; 32];
    let mut carry = 0u32;
    for i in 0..32 {
        let v = l[i] as u32 * k as u32 + carry;
        out[i] = v as u8;
        carry = v >> 8;
    }
    assert_eq!(carry, 0);
    assert_eq!(out[31] & 0x80, 0, "kL must have bit 255 clear");
    out
}

/// The multiples of `L` that survive `t[31] &= 127` unchanged.
const L_MULTIPLES: [u16; 5] = [1, 2, 3, 5, 7];

/// `8L mod 2^255` — i.e. `8L` with bit 255 masked off, exactly what the four
/// ed25519 paths and the two ristretto255 paths do to the caller's scalar.
/// `8L = 2^255 + 8·c`, so the mask turns it into `8·c`, which is **not** a
/// multiple of `L`: the scalar is therefore accepted, not rejected.
fn eight_l_truncated() -> [u8; 32] {
    let l = h32(L_HEX);
    let mut out = [0u8; 32];
    let mut carry = 0u32;
    for i in 0..32 {
        let v = l[i] as u32 * 8 + carry;
        out[i] = v as u8;
        carry = v >> 8;
    }
    assert_ne!(out[31] & 0x80, 0, "8L must set bit 255");
    out[31] &= 0x7f;
    out
}

fn add_small(mut a: [u8; 32], k: u8) -> [u8; 32] {
    let mut carry = k as u16;
    for b in a.iter_mut() {
        let v = *b as u16 + carry;
        *b = v as u8;
        carry = v >> 8;
        if carry == 0 {
            break;
        }
    }
    a
}

/// A grab-bag of "interesting" 32-byte scalars, plus `extra` random ones.
fn scalar_corpus(seed: u64, extra: usize) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    v.push([0u8; 32]); // all zero
    v.push([0xffu8; 32]); // all 0xff
    v.push(h32(L_HEX)); // L
    let mut lm1 = h32(L_HEX);
    lm1[0] -= 1;
    v.push(lm1); // L - 1
    v.push(add_small(h32(L_HEX), 1)); // L + 1
    v.push(mul_l(2)); // 2L
    v.push(mul_l(7)); // 7L, the largest multiple of L below 2^255
    v.push(eight_l_truncated()); // 8L with bit 255 masked off
    v.push(h32(ED_IDENTITY)); // 1
    let mut two = [0u8; 32];
    two[0] = 2;
    v.push(two);
    v.push(h32(P_HEX)); // 2^255 - 19
    let mut hi = [0u8; 32];
    hi[31] = 0x80;
    v.push(hi); // 2^255
    let mut clamped = [0u8; 32];
    clamp_curve(&mut clamped);
    v.push(clamped); // clamp(0) == 2^254
    for s in CURVE_SMALL_ORDER {
        v.push(h32(s)); // small-order *encodings used as scalars*
    }
    let mut rng = Rng::new(seed);
    for _ in 0..extra {
        v.push(rng.bytes(32).try_into().unwrap());
    }
    v
}

// ================================================================ 7.28 / 7.127
// The exported ref10 implementation table, and the accessors re-checked as
// pure/idempotent constant functions.

#[repr(C)]
struct Ref10Impl {
    mult: Option<Mult>,
    mult_base: Option<MultBase>,
}

#[test]
fn gap_ref10_implementation_table() {
    // The dispatch table is an exported *data* symbol.  Reading the two
    // function pointers out of it and calling them is the only way to reach
    // `crypto_scalarmult_curve25519_ref10` / `..._ref10_base` directly and so
    // to distinguish the primitive from the `crypto_scalarmult_curve25519`
    // wrapper that layers the all-zero guard on top of it.
    // `libloading` hands out data symbols as `Symbol<*const T>` (the symbol's
    // address is transmuted into the pointer), so one deref gives the struct.
    let (ci, ri) = both::<*const Ref10Impl>("crypto_scalarmult_curve25519_ref10_implementation");
    let (ci, ri) = unsafe { (&**ci, &**ri) };
    let cm = ci.mult.expect("C mult is NULL");
    let cb = ci.mult_base.expect("C mult_base is NULL");
    let rm = ri.mult.expect("Rust mult is NULL");
    let rb = ri.mult_base.expect("Rust mult_base is NULL");

    let wrapper = F3::new("crypto_scalarmult_curve25519");
    let wrapper_base = F2::new("crypto_scalarmult_curve25519_base");
    let bp = h32(X25519_BASEPOINT);
    let mut rng = Rng::new(0xa101);

    let mut all_zero_seen = 0usize;
    for i in 0..160 {
        let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        // Points: the basepoint, a real public key, blocklist entries and junk.
        let p: [u8; 32] = match i % 4 {
            0 => bp,
            1 => {
                let sk: [u8; 32] = rng.bytes(32).try_into().unwrap();
                wrapper_base.expect_ok(&sk, "helper")
            }
            2 => h32(CURVE_SMALL_ORDER[i % 7]),
            _ => rng.bytes(32).try_into().unwrap(),
        };

        // mult
        let f = fill_of(0x33);
        let mut qc = padded(32);
        let mut qr = padded(32);
        qc[..32].copy_from_slice(&f);
        qr[..32].copy_from_slice(&f);
        let rcc = unsafe { cm(qc.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        let rcr = unsafe { rm(qr.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
        eqi("ref10 mult rc", rcc, rcr);
        eqb("ref10 mult q", &qc, &qr);
        check_pad("ref10 mult", &qc, 32);

        // The wrapper must agree with the primitive except that it converts an
        // all-zero output into `-1` (errors_7.md 7.9).
        let (wrc, wq, _) = wrapper.run(&n, &p, 0x33);
        if rcc == -1 {
            eqi("wrapper agrees on small-order reject", wrc, -1);
            eqb("primitive early reject leaves q untouched", &qc[..32], &f);
        } else {
            eqi("primitive succeeded", rcc, 0);
            let is_zero = qc[..32].iter().all(|&x| x == 0);
            if is_zero {
                all_zero_seen += 1;
                eqi("wrapper converts all-zero output to -1", wrc, -1);
            } else {
                eqi("wrapper passes through", wrc, 0);
            }
            eqb("wrapper output == primitive output", &wq, &qc[..32]);
        }

        // mult_base is unconditional: `return 0`, always.
        let mut bc = padded(32);
        let mut br = padded(32);
        bc[..32].copy_from_slice(&f);
        br[..32].copy_from_slice(&f);
        let brc = unsafe { cb(bc.as_mut_ptr(), n.as_ptr()) };
        let brr = unsafe { rb(br.as_mut_ptr(), n.as_ptr()) };
        eqi("ref10 mult_base rc", brc, brr);
        eqi("ref10 mult_base always returns 0", brc, 0);
        eqb("ref10 mult_base q", &bc, &br);
        check_pad("ref10 mult_base", &bc, 32);
        // `crypto_scalarmult_curve25519_base` *is* `mult_base` (no wrapper).
        let (wbrc, wbq, _) = wrapper_base.run(&n, 0x33);
        eqi("_base rc", wbrc, 0);
        eqb("_base == ref10 mult_base", &wbq, &bc[..32]);
    }
    // errors_7.md 7.9 is unreachable with ref10: every input that would give an
    // all-zero X25519 output is already caught by the small-order blocklist.
    assert_eq!(
        all_zero_seen, 0,
        "the ref10 primitive produced an all-zero output — 7.9 would be reachable"
    );
}

#[test]
fn gap_accessors_are_constant_and_pure() {
    // errors_7.md 7.127: constant returns, never NULL, stable across calls.
    let sizes = [
        ("crypto_scalarmult_bytes", 32usize),
        ("crypto_scalarmult_scalarbytes", 32),
        ("crypto_scalarmult_curve25519_bytes", 32),
        ("crypto_scalarmult_curve25519_scalarbytes", 32),
        ("crypto_scalarmult_ed25519_bytes", 32),
        ("crypto_scalarmult_ed25519_scalarbytes", 32),
        ("crypto_scalarmult_ristretto255_bytes", 32),
        ("crypto_scalarmult_ristretto255_scalarbytes", 32),
    ];
    for (f, want) in sizes {
        let (c, r) = both::<SizeFn>(f);
        for _ in 0..8 {
            let (vc, vr) = unsafe { (c(), r()) };
            assert_eq!(vc, vr, "{f}: C {vc} != Rust {vr}");
            assert_eq!(vc, want, "{f}: expected {want}, got {vc}");
        }
    }
    let (c, r) = both::<StrFn>("crypto_scalarmult_primitive");
    unsafe {
        let p1 = c();
        let p2 = c();
        assert!(!p1.is_null() && !p2.is_null(), "primitive() returned NULL");
        assert_eq!(p1, p2, "C primitive() is not a stable pointer");
        let q1 = r();
        let q2 = r();
        assert!(!q1.is_null() && !q2.is_null(), "Rust primitive() returned NULL");
        assert_eq!(q1, q2, "Rust primitive() is not a stable pointer");
        assert_eq!(CStr::from_ptr(p1).to_str().unwrap(), "curve25519");
        assert_eq!(CStr::from_ptr(q1).to_str().unwrap(), "curve25519");
    }
    // The generic aliases must report exactly the curve25519 values.
    for (g, s) in [
        ("crypto_scalarmult_bytes", "crypto_scalarmult_curve25519_bytes"),
        (
            "crypto_scalarmult_scalarbytes",
            "crypto_scalarmult_curve25519_scalarbytes",
        ),
    ] {
        let (gc, _) = both::<SizeFn>(g);
        let (sc, _) = both::<SizeFn>(s);
        unsafe { assert_eq!(gc(), sc(), "{g} != {s}") };
    }
}

// ======================================================= errors 7.11 / 7.12
// `crypto_scalarmult_curve25519_base` and `crypto_scalarmult_base` can NEVER
// fail: they call `ref10_implementation.mult_base` directly, bypassing the
// wrapper's all-zero-output guard, and `..._ref10_base` unconditionally
// `return 0`.

#[test]
fn gap_curve25519_base_never_fails() {
    let base = F2::new("crypto_scalarmult_curve25519_base");
    let generic = F2::new("crypto_scalarmult_base");
    let mult = F3::new("crypto_scalarmult_curve25519");
    let bp = h32(X25519_BASEPOINT);

    for n in scalar_corpus(0xb101, 400) {
        // Every pre-fill pattern, both entry points, always 0.
        let q = base.expect_ok(&n, "base never fails");
        let qg = generic.expect_ok(&n, "generic base never fails");
        eqb("crypto_scalarmult_base == curve25519_base", &q, &qg);
        assert!(
            q.iter().any(|&x| x != 0),
            "the _base output must not be all-zero for n={}",
            hex(&n)
        );

        // Cross-check against the Montgomery ladder over the basepoint: two
        // structurally different code paths (ge25519_scalarmult_base +
        // edwards_to_montgomery vs the ladder) that must agree.
        let (rc, ql, _) = mult.run(&n, &bp, 0x11);
        eqi("ladder over basepoint rc", rc, 0);
        eqb("mult(n, basepoint) == base(n)", &ql, &q);

        // Clamping is idempotent, and pre-clamping changes nothing.
        let mut cl = n;
        clamp_curve(&mut cl);
        let qc = base.expect_ok(&cl, "pre-clamped");
        eqb("clamp idempotent", &q, &qc);
        let mut cl2 = cl;
        clamp_curve(&mut cl2);
        assert_eq!(cl, cl2, "clamp is not idempotent");
    }

    // In-place `q == n` on the `_base` path (the documented usage: the impl
    // copies `n` into `t = q` as its first action).
    let mut rng = Rng::new(0xb102);
    for i in 0..64 {
        let n: [u8; 32] = match i {
            0 => [0u8; 32],
            1 => [0xffu8; 32],
            2 => h32(L_HEX),
            _ => rng.bytes(32).try_into().unwrap(),
        };
        let want = base.expect_ok(&n, "reference");
        for (name, f) in [
            ("crypto_scalarmult_curve25519_base", &base),
            ("crypto_scalarmult_base", &generic),
        ] {
            let mut qc = padded(32);
            let mut qr = padded(32);
            qc[..32].copy_from_slice(&n);
            qr[..32].copy_from_slice(&n);
            let rc = unsafe { (f.c)(qc.as_mut_ptr(), qc.as_ptr()) };
            let rr = unsafe { (f.r)(qr.as_mut_ptr(), qr.as_ptr()) };
            eqi(&format!("{name} in-place rc"), rc, rr);
            eqi(&format!("{name} in-place rc == 0"), rc, 0);
            eqb(&format!("{name} in-place q"), &qc, &qr);
            check_pad(name, &qc, 32);
            eqb(&format!("{name} in-place == out-of-place"), &qc[..32], &want);
        }
    }
}

// ================================================== errors 7.1–7.8 revisited
// The blocklist, with every pre-fill pattern and many random scalars, on both
// `crypto_scalarmult_curve25519` and the generic alias.

#[test]
fn gap_curve25519_blocklist_all_prefills() {
    let m = F3::new("crypto_scalarmult_curve25519");
    let g = F3::new("crypto_scalarmult");
    let mut rng = Rng::new(0xb201);
    for hs in CURVE_SMALL_ORDER {
        let p = h32(hs);
        let mut p_hi = p;
        p_hi[31] |= 0x80;
        for _ in 0..12 {
            let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
            m.expect_early_reject(&n, &p, hs);
            g.expect_early_reject(&n, &p, hs);
            // errors_7.md 7.8 — `has_small_order` masks `s[31] & 0x7f`, so the
            // top bit cannot be used to escape the blocklist.
            m.expect_early_reject(&n, &p_hi, "blocklist|0x80");
            g.expect_early_reject(&n, &p_hi, "blocklist|0x80");
        }
        // Degenerate scalars are rejected on the same path.
        for n in [[0u8; 32], [0xffu8; 32], h32(L_HEX), h32(ED_IDENTITY)] {
            m.expect_early_reject(&n, &p, "blocklist + degenerate n");
        }
        // Yet the very same byte strings are perfectly fine *as scalars*.
        let base = F2::new("crypto_scalarmult_curve25519_base");
        base.expect_ok(&p, "blocklist encoding used as a scalar");
    }
}

// ================================================ configs 7.8/7.9 revisited
// Non-canonical field elements: `fe25519_frombytes` masks bit 255 and the
// arithmetic reduces mod `p`, so `p + k` behaves exactly like `k`.

#[test]
fn gap_curve25519_noncanonical_field_elements() {
    let m = F3::new("crypto_scalarmult_curve25519");
    let base = F2::new("crypto_scalarmult_curve25519_base");
    let mut rng = Rng::new(0xb301);
    // p + k for k = 2 … 18 (k = 0, 1 are on the blocklist, and 19 wraps).
    for k in 2u16..=18 {
        let p_plus_k = add_small(h32(P_HEX), k as u8);
        let mut reduced = [0u8; 32];
        reduced[0] = k as u8;
        for _ in 0..6 {
            let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let (rc1, q1, _) = m.run(&n, &p_plus_k, 0x27);
            let (rc2, q2, _) = m.run(&n, &reduced, 0x27);
            eqi(&format!("p+{k} vs {k}: rc"), rc1, rc2);
            eqb(&format!("p+{k} reduces to {k}"), &q1, &q2);
            // And with bit 255 set on top of the non-canonical encoding.
            let mut hi = p_plus_k;
            hi[31] |= 0x80;
            let (rc3, q3, _) = m.run(&n, &hi, 0x27);
            eqi("bit255 on non-canonical rc", rc3, rc1);
            eqb("bit255 on non-canonical q", &q3, &q1);
        }
    }
    // 2^255 - 1 (= p + 18) and 2^256 - 1 (bit 255 masked -> p + 18).
    let n = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let mut m1 = [0xffu8; 32];
    m1[31] = 0x7f;
    let (rca, qa, _) = m.run(&n, &m1, 0x44);
    let (rcb, qb, _) = m.run(&n, &[0xffu8; 32], 0x44);
    eqi("2^255-1 vs 2^256-1 rc", rca, rcb);
    eqb("2^255-1 vs 2^256-1 q", &qa, &qb);
    eqi("non-canonical accepted", rca, 0);

    // A real public key with bit 255 set is accepted and identical.
    for _ in 0..40 {
        let sk: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let pk = base.expect_ok(&sk, "pk");
        let mut pk_hi = pk;
        pk_hi[31] |= 0x80;
        let (rc1, q1, _) = m.run(&n, &pk, 0x66);
        let (rc2, q2, _) = m.run(&n, &pk_hi, 0x66);
        eqi("pk|0x80 rc", rc1, rc2);
        eqi("pk accepted", rc1, 0);
        eqb("pk|0x80 q", &q1, &q2);
    }
}

// ============================================== errors 7.13–7.25 (ed25519)
// Every ed25519 rejection, per row, with all four pre-fill patterns.

#[test]
fn gap_ed25519_early_rejects_per_row() {
    let bc = F2::new("crypto_scalarmult_ed25519_base");
    let mut rng = Rng::new(0xc101);
    let valid_p = bc.expect_ok(&rng.bytes(32), "valid point");

    for mname in [
        "crypto_scalarmult_ed25519",
        "crypto_scalarmult_ed25519_noclamp",
    ] {
        let m = F3::new(mname);

        // 7.13 — `ge25519_is_canonical(p) == 0`: p[31]&0x7f == 0x7f,
        // p[1..30] == 0xff, p[0] >= 0xed.
        for lo in [0xedu8, 0xee, 0xef, 0xf0, 0xf7, 0xfe, 0xff] {
            let mut p = [0xffu8; 32];
            p[0] = lo;
            p[31] = 0x7f;
            for _ in 0..4 {
                let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
                m.expect_early_reject(&n, &p, "7.13 non-canonical y");
                let mut p_hi = p;
                p_hi[31] = 0xff;
                m.expect_early_reject(&n, &p_hi, "7.13 non-canonical y | 0x80");
            }
        }

        // 7.14 — `ge25519_frombytes` fails (x^2 is a non-square).
        let mut nondecode = 0;
        for k in 2u8..80 {
            let mut p = [0u8; 32];
            p[0] = k;
            let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let (rc, _, _) = m.run(&n, &p, 0x5a);
            if rc == -1 {
                m.expect_early_reject(&n, &p, "7.14/7.15/7.17 rejected y");
                nondecode += 1;
            }
        }
        assert!(nondecode > 0, "expected non-decodable y values");

        // 7.15 / 7.16 — the 8 torsion encodings.
        for hs in ED_SMALL_ORDER {
            let p = h32(hs);
            for _ in 0..4 {
                let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
                m.expect_early_reject(&n, &p, "7.15/7.16 small order");
            }
            // …and the same encoding with bit 255 flipped.
            let mut p_hi = p;
            p_hi[31] ^= 0x80;
            let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let (rc, _, _) = m.run(&n, &p_hi, 0x5a);
            eqi("torsion ^0x80 rc", rc, -1);
        }

        // 7.17 — a canonical, non-small-order point off the main subgroup.
        let (ca, ra) = both::<Mult>("crypto_core_ed25519_add");
        for tors in [
            ED_SMALL_ORDER[1],
            ED_SMALL_ORDER[4],
            ED_SMALL_ORDER[5],
            ED_SMALL_ORDER[6],
            ED_SMALL_ORDER[7],
        ] {
            let t = h32(tors);
            let mut oc = padded(32);
            let mut or = padded(32);
            let x = unsafe { ca(oc.as_mut_ptr(), valid_p.as_ptr(), t.as_ptr()) };
            let y = unsafe { ra(or.as_mut_ptr(), valid_p.as_ptr(), t.as_ptr()) };
            eqi("core add rc", x, y);
            eqb("core add", &oc, &or);
            if x != 0 {
                continue;
            }
            let off: [u8; 32] = oc[..32].try_into().unwrap();
            for _ in 0..4 {
                let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
                m.expect_early_reject(&n, &off, "7.17 off main subgroup");
            }
        }
    }
}

#[test]
fn gap_ed25519_late_rejects_per_row() {
    let bc = F2::new("crypto_scalarmult_ed25519_base");
    let bn = F2::new("crypto_scalarmult_ed25519_base_noclamp");
    let mc = F3::new("crypto_scalarmult_ed25519");
    let mn = F3::new("crypto_scalarmult_ed25519_noclamp");
    let inf = h32(ED_IDENTITY);
    let mut rng = Rng::new(0xc201);

    // A batch of valid points to make each row randomized.
    let mut pts: Vec<[u8; 32]> = Vec::new();
    while pts.len() < 12 {
        let s: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let (rc, q, _) = bc.run(&s, 0x77);
        if rc == 0 {
            pts.push(q);
        }
    }

    let mut clamp_zero = [0u8; 32];
    clamp_zero[0] &= 248;
    clamp_zero[31] |= 64;

    for p in &pts {
        // 7.18 — clamped, n = 0: `sodium_is_zero(n)` fires but the point is
        // *not* the identity, so `q` holds a genuine curve point.
        let (rc_ref, want, _) = mn.run(&clamp_zero, p, 0x0f);
        eqi("noclamp(clamp(0)) succeeds", rc_ref, 0);
        mc.expect_late_reject(&[0u8; 32], p, &want, "7.18 clamped n=0");

        // 7.19 — noclamp, n = 0: both disjuncts fire, `q` is the identity.
        mn.expect_late_reject(&[0u8; 32], p, &inf, "7.19 noclamp n=0");

        // 7.20 — n ≡ 0 (mod L), nonzero: only `_is_inf` fires.
        for k in L_MULTIPLES {
            let nl = mul_l(k);
            mn.expect_late_reject(&nl, p, &inf, "7.20 n = kL");
            // 7.21 — bit 255 is cleared unconditionally on both paths.
            let mut nl_hi = nl;
            nl_hi[31] |= 0x80;
            mn.expect_late_reject(&nl_hi, p, &inf, "7.21 n = kL + 2^255");
        }
        // `8L` masked back into 255 bits is *not* a multiple of L, so it is
        // accepted — the mask is genuinely lossy.
        let (rc, _, _) = mn.run(&eight_l_truncated(), p, 0x0f);
        eqi("8L mod 2^255 is accepted", rc, 0);

        // A clamped multiple-of-L scalar is *not* a multiple of L any more, so
        // the clamped entry point does not take the `_is_inf` path for it.
        let l = h32(L_HEX);
        let mut lc = l;
        clamp_curve(&mut lc);
        let (rc, q, _) = mc.run(&l, p, 0x0f);
        let (rc2, q2, _) = mn.run(&lc, p, 0x0f);
        eqi("clamped n=L rc", rc, rc2);
        eqi("clamped n=L is accepted", rc, 0);
        eqb("clamped n=L q", &q, &q2);
    }

    // 7.22 — `_base`, clamped, n = 0.
    let want = bn.expect_ok(&clamp_zero, "base_noclamp(clamp(0))");
    bc.expect_late_reject(&[0u8; 32], &want, "7.22 base clamped n=0");
    // 7.23 — `_base_noclamp`, n = 0.
    bn.expect_late_reject(&[0u8; 32], &inf, "7.23 base_noclamp n=0");
    // 7.24 — `_base_noclamp`, n = kL (bit 255 clear and set).
    for k in L_MULTIPLES {
        let nl = mul_l(k);
        bn.expect_late_reject(&nl, &inf, "7.24 base_noclamp n=kL");
        let mut hi = nl;
        hi[31] |= 0x80;
        bn.expect_late_reject(&hi, &inf, "7.24 base_noclamp n=kL+2^255");
    }
    // `L + 1` is *not* a rejection: it acts as the scalar 1.
    let lp1 = add_small(h32(L_HEX), 1);
    let q = bn.expect_ok(&lp1, "L+1");
    assert_eq!(hex(&q), ED25519_BASEPOINT, "(L+1)B must be B");
    // `L - 1` acts as `-1`.
    let mut lm1 = h32(L_HEX);
    lm1[0] -= 1;
    let negb = bn.expect_ok(&lm1, "L-1");
    assert_ne!(hex(&negb), ED25519_BASEPOINT);
    for p in &pts {
        let q1 = {
            let (rc, q, _) = mn.run(&lp1, p, 0x21);
            eqi("noclamp(L+1) rc", rc, 0);
            q
        };
        eqb("noclamp(L+1, p) == p", &q1, p);
    }

    // errors_7.md 7.25 — the shared side-effect contract, spot-checked with the
    // output buffer aliased onto `n` and onto `p`.
    for p in pts.iter().take(4) {
        // `q == n` with an all-zero scalar: because `t = q`, by the time the
        // trailing `sodium_is_zero(n, 32)` runs, `n` *is* the freshly written
        // output, so the zero-scalar guard does NOT fire.  The clamped variant
        // therefore returns 0 (the point is not the identity), while the
        // noclamp variant still returns -1 — via `_is_inf(q)` alone, because
        // `0·P` is the identity.  Both facts follow purely from the aliasing and
        // must be reproduced exactly.
        for (name, f, want_rc) in [
            ("crypto_scalarmult_ed25519", &mc, 0),
            ("crypto_scalarmult_ed25519_noclamp", &mn, -1),
        ] {
            let mut qc = padded(32);
            let mut qr = padded(32);
            let rcc = unsafe { (f.c)(qc.as_mut_ptr(), qc.as_ptr(), p.as_ptr()) };
            let rcr = unsafe { (f.r)(qr.as_mut_ptr(), qr.as_ptr(), p.as_ptr()) };
            eqi(&format!("{name} alias q==n (n=0) rc"), rcc, rcr);
            eqi(&format!("{name} alias q==n (n=0) rc value"), rcc, want_rc);
            eqb(&format!("{name} alias q==n (n=0) q"), &qc, &qr);
            check_pad(name, &qc, 32);
            // The bytes written are the same as for the non-aliased call.
            let (_, want_q, _) = f.run(&[0u8; 32], p, 0x0f);
            eqb(&format!("{name} alias q==n (n=0) bytes"), &qc[..32], &want_q);
        }
        // q == p on an early reject: `p` must survive untouched.
        let bad = {
            let mut b = [0xffu8; 32];
            b[0] = 0xed;
            b[31] = 0x7f;
            b
        };
        for (name, f) in [
            ("crypto_scalarmult_ed25519", &mc),
            ("crypto_scalarmult_ed25519_noclamp", &mn),
        ] {
            let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let mut qc = padded(32);
            let mut qr = padded(32);
            qc[..32].copy_from_slice(&bad);
            qr[..32].copy_from_slice(&bad);
            let rcc = unsafe { (f.c)(qc.as_mut_ptr(), n.as_ptr(), qc.as_ptr()) };
            let rcr = unsafe { (f.r)(qr.as_mut_ptr(), n.as_ptr(), qr.as_ptr()) };
            eqi(&format!("{name} alias q==p early reject rc"), rcc, rcr);
            eqi(&format!("{name} alias q==p early reject rc == -1"), rcc, -1);
            eqb(&format!("{name} alias q==p early reject q"), &qc, &qr);
            eqb(&format!("{name} alias q==p keeps p"), &qc[..32], &bad);
        }
    }
}

// ============================================ errors 7.26–7.33 (ristretto255)

#[test]
fn gap_ristretto255_rejects_per_row() {
    let b = F2::new("crypto_scalarmult_ristretto255_base");
    let m = F3::new("crypto_scalarmult_ristretto255");
    let (cv, rv) = both::<unsafe extern "C" fn(*const u8) -> c_int>(
        "crypto_core_ristretto255_is_valid_point",
    );
    let zero = h32(ZERO32);
    let mut rng = Rng::new(0xd101);

    // Valid non-identity points.
    let mut pts: Vec<[u8; 32]> = Vec::new();
    for k in 1u8..=10 {
        let mut s = [0u8; 32];
        s[0] = k;
        pts.push(b.expect_ok(&s, "ristretto point"));
    }

    // 7.26 — non-canonical byte strings.
    let mut noncanon: Vec<[u8; 32]> = Vec::new();
    for k in 0u8..19 {
        // p, p+1, …, p+18 = 2^255-1
        noncanon.push(add_small(h32(P_HEX), k));
    }
    for _ in 0..16 {
        let mut p: [u8; 32] = rng.bytes(32).try_into().unwrap();
        p[31] |= 0x80; // bit 255 set is never canonical
        noncanon.push(p);
    }
    for p in &noncanon {
        let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        m.expect_early_reject(&n, p, "7.26 non-canonical");
    }

    // 7.27 / 7.28 — canonical bytes off the ristretto255 image.
    let mut rejected = 0;
    for _ in 0..400 {
        let mut p: [u8; 32] = rng.bytes(32).try_into().unwrap();
        p[31] &= 0x7f;
        let (a, bb) = unsafe { (cv(p.as_ptr()), rv(p.as_ptr())) };
        eqi("ristretto is_valid_point agreement", a, bb);
        if a != 0 {
            continue;
        }
        let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        m.expect_early_reject(&n, &p, "7.27/7.28 not on the image");
        rejected += 1;
        if rejected >= 24 {
            break;
        }
    }
    assert!(rejected > 0, "expected invalid ristretto encodings");

    // 7.29 — the identity encoding is accepted by `ristretto255_frombytes` but
    // every multiple of it re-encodes to 32 zero bytes: a LATE reject.
    let (a, bb) = unsafe { (cv(zero.as_ptr()), rv(zero.as_ptr())) };
    eqi("identity validity agreement", a, bb);
    for _ in 0..12 {
        let mut n: [u8; 32] = rng.bytes(32).try_into().unwrap();
        n[31] &= 0x7f;
        m.expect_late_reject(&n, &zero, &zero, "7.29 identity point");
    }
    // …including n = 1, which would otherwise be the identity map.
    m.expect_late_reject(&h32(ED_IDENTITY), &zero, &zero, "7.29 identity, n=1");

    // 7.30 — n = 0 with a valid non-identity point (no clamping on this path).
    for p in &pts {
        m.expect_late_reject(&[0u8; 32], p, &zero, "7.30 n=0");
        // 7.31 — n ≡ 0 (mod L), bit 255 clear and set.
        for k in L_MULTIPLES {
            let nl = mul_l(k);
            m.expect_late_reject(&nl, p, &zero, "7.31 n=kL");
            let mut hi = nl;
            hi[31] |= 0x80;
            m.expect_late_reject(&hi, p, &zero, "7.31 n=kL+2^255");
        }
        // `8L` after `t[31] &= 127` is no longer a multiple of L: accepted.
        let (rc, _, _) = m.run(&eight_l_truncated(), p, 0x3c);
        eqi("ristretto 8L mod 2^255 is accepted", rc, 0);
        // `L + 1` acts as 1: the identity map, re-encoded canonically.
        let lp1 = add_small(h32(L_HEX), 1);
        let (rc, q, _) = m.run(&lp1, p, 0x3c);
        eqi("ristretto n=L+1 rc", rc, 0);
        eqb("ristretto n=L+1 is the identity map", &q, p);
        // `L - 1` acts as -1 and must differ from p.
        let mut lm1 = h32(L_HEX);
        lm1[0] -= 1;
        let (rc, q, _) = m.run(&lm1, p, 0x3c);
        eqi("ristretto n=L-1 rc", rc, 0);
        assert_ne!(&q, p, "-P must differ from P");
    }

    // 7.32 / 7.33 — the `_base` path.
    b.expect_late_reject(&[0u8; 32], &zero, "7.32 base n=0");
    for k in L_MULTIPLES {
        let nl = mul_l(k);
        b.expect_late_reject(&nl, &zero, "7.33 base n=kL");
        let mut hi = nl;
        hi[31] |= 0x80;
        b.expect_late_reject(&hi, &zero, "7.33 base n=kL+2^255");
    }
    let _ = b.expect_ok(&eight_l_truncated(), "base 8L mod 2^255");
    // `L + 1` on the base path is the basepoint again.
    let lp1 = add_small(h32(L_HEX), 1);
    let q = b.expect_ok(&lp1, "base L+1");
    let mut one = [0u8; 32];
    one[0] = 1;
    let q1 = b.expect_ok(&one, "base 1");
    eqb("base(L+1) == base(1)", &q, &q1);

    // Aliasing on both reject flavours.
    let n: [u8; 32] = rng.bytes(32).try_into().unwrap();
    let mut qc = padded(32);
    let mut qr = padded(32);
    qc[..32].copy_from_slice(&noncanon[0]);
    qr[..32].copy_from_slice(&noncanon[0]);
    let rcc = unsafe { (m.c)(qc.as_mut_ptr(), n.as_ptr(), qc.as_ptr()) };
    let rcr = unsafe { (m.r)(qr.as_mut_ptr(), n.as_ptr(), qr.as_ptr()) };
    eqi("ristretto alias q==p early reject rc", rcc, rcr);
    eqi("ristretto alias q==p early reject == -1", rcc, -1);
    eqb("ristretto alias q==p q", &qc, &qr);
    eqb("ristretto alias q==p keeps p", &qc[..32], &noncanon[0]);

    let mut qc = padded(32);
    let mut qr = padded(32);
    let rcc = unsafe { (b.c)(qc.as_mut_ptr(), qc.as_ptr()) };
    let rcr = unsafe { (b.r)(qr.as_mut_ptr(), qr.as_ptr()) };
    eqi("ristretto base alias n=0 rc", rcc, rcr);
    eqi("ristretto base alias n=0 == -1", rcc, -1);
    eqb("ristretto base alias q", &qc, &qr);
}

// ==================================================== configs 7.129
// `crypto_kx` vs `crypto_box_beforenm`: both start from the same
// `crypto_scalarmult` output but post-process it with BLAKE2b resp. HSalsa20.

type Kx = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> c_int;
type Beforenm = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type GenericHash =
    unsafe extern "C" fn(*mut u8, usize, *const u8, u64, *const u8, usize) -> c_int;
type HSalsa20 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8) -> c_int;

#[test]
fn gap_kx_vs_box_beforenm_consistency() {
    let (ckc, rkc) = both::<Kx>("crypto_kx_client_session_keys");
    let (cks, rks) = both::<Kx>("crypto_kx_server_session_keys");
    let (cbn, rbn) = both::<Beforenm>("crypto_box_beforenm");
    let (cgh, rgh) = both::<GenericHash>("crypto_generichash");
    let (chs, rhs) = both::<HSalsa20>("crypto_core_hsalsa20");
    let mult = F3::new("crypto_scalarmult");
    let base = F2::new("crypto_scalarmult_base");
    let mut rng = Rng::new(0xe101);

    for _ in 0..48 {
        let ska: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let skb: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let pka = base.expect_ok(&ska, "pka");
        let pkb = base.expect_ok(&skb, "pkb");

        // The shared X25519 secret both constructions are built on.
        let (rc, q, _) = mult.run(&ska, &pkb, 0x18);
        eqi("scalarmult rc", rc, 0);
        let (rc2, q2, _) = mult.run(&skb, &pka, 0x18);
        eqi("scalarmult rc", rc2, 0);
        eqb("DH agreement", &q, &q2);

        // --- crypto_kx: BLAKE2b-64(q ‖ client_pk ‖ server_pk), split in half.
        let mut crx = padded(32);
        let mut ctx = padded(32);
        let mut rrx = padded(32);
        let mut rtx = padded(32);
        let rc_c = unsafe {
            ckc(
                crx.as_mut_ptr(),
                ctx.as_mut_ptr(),
                pka.as_ptr(),
                ska.as_ptr(),
                pkb.as_ptr(),
            )
        };
        let rc_r = unsafe {
            rkc(
                rrx.as_mut_ptr(),
                rtx.as_mut_ptr(),
                pka.as_ptr(),
                ska.as_ptr(),
                pkb.as_ptr(),
            )
        };
        eqi("kx client rc", rc_c, rc_r);
        eqi("kx client rc == 0", rc_c, 0);
        eqb("kx client rx", &crx, &rrx);
        eqb("kx client tx", &ctx, &rtx);
        check_pad("kx client rx", &crx, 32);
        check_pad("kx client tx", &ctx, 32);

        // Recompute it independently from `q`.
        let mut msg = Vec::new();
        msg.extend_from_slice(&q);
        msg.extend_from_slice(&pka);
        msg.extend_from_slice(&pkb);
        let mut kc = padded(64);
        let mut kr = padded(64);
        unsafe {
            let x = cgh(
                kc.as_mut_ptr(),
                64,
                msg.as_ptr(),
                msg.len() as u64,
                std::ptr::null(),
                0,
            );
            let y = rgh(
                kr.as_mut_ptr(),
                64,
                msg.as_ptr(),
                msg.len() as u64,
                std::ptr::null(),
                0,
            );
            eqi("generichash rc", x, y);
        }
        eqb("generichash", &kc, &kr);
        eqb("kx client rx == keys[0..31]", &crx[..32], &kc[..32]);
        eqb("kx client tx == keys[32..63]", &ctx[..32], &kc[32..64]);

        // The server side mirrors it.
        let mut srx = padded(32);
        let mut stx = padded(32);
        let mut srx_r = padded(32);
        let mut stx_r = padded(32);
        let rc_c = unsafe {
            cks(
                srx.as_mut_ptr(),
                stx.as_mut_ptr(),
                pkb.as_ptr(),
                skb.as_ptr(),
                pka.as_ptr(),
            )
        };
        let rc_r = unsafe {
            rks(
                srx_r.as_mut_ptr(),
                stx_r.as_mut_ptr(),
                pkb.as_ptr(),
                skb.as_ptr(),
                pka.as_ptr(),
            )
        };
        eqi("kx server rc", rc_c, rc_r);
        eqi("kx server rc == 0", rc_c, 0);
        eqb("kx server rx", &srx, &srx_r);
        eqb("kx server tx", &stx, &stx_r);
        eqb("client_rx == server_tx", &crx[..32], &stx[..32]);
        eqb("client_tx == server_rx", &ctx[..32], &srx[..32]);

        // --- crypto_box_beforenm: HSalsa20(q, zero nonce).
        let mut bc = padded(32);
        let mut br = padded(32);
        let x = unsafe { cbn(bc.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr()) };
        let y = unsafe { rbn(br.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr()) };
        eqi("beforenm rc", x, y);
        eqi("beforenm rc == 0", x, 0);
        eqb("beforenm k", &bc, &br);
        check_pad("beforenm", &bc, 32);

        let zero16 = [0u8; 16];
        let mut hc = padded(32);
        let mut hr = padded(32);
        unsafe {
            let a = chs(hc.as_mut_ptr(), zero16.as_ptr(), q.as_ptr(), std::ptr::null());
            let b = rhs(hr.as_mut_ptr(), zero16.as_ptr(), q.as_ptr(), std::ptr::null());
            eqi("hsalsa20 rc", a, b);
        }
        eqb("hsalsa20", &hc, &hr);
        eqb("beforenm == HSalsa20(q, 0)", &bc[..32], &hc[..32]);

        // The two constructions must be different keys — the whole point of the
        // row is that a port cannot substitute one for the other.
        assert_ne!(&bc[..32], &crx[..32], "beforenm == kx rx");
        assert_ne!(&bc[..32], &ctx[..32], "beforenm == kx tx");
        assert_ne!(&bc[..32], &q[..], "beforenm == raw DH secret");
        assert_ne!(&crx[..32], &q[..], "kx rx == raw DH secret");

        // Both are stable across repeated calls.
        let mut bc2 = padded(32);
        unsafe { cbn(bc2.as_mut_ptr(), pkb.as_ptr(), ska.as_ptr()) };
        eqb("beforenm stable", &bc, &bc2);
        let mut crx2 = padded(32);
        let mut ctx2 = padded(32);
        unsafe {
            ckc(
                crx2.as_mut_ptr(),
                ctx2.as_mut_ptr(),
                pka.as_ptr(),
                ska.as_ptr(),
                pkb.as_ptr(),
            )
        };
        eqb("kx rx stable", &crx, &crx2);
        eqb("kx tx stable", &ctx, &ctx2);

        // beforenm is symmetric in (pk, sk).
        let mut bs = padded(32);
        let mut bs_r = padded(32);
        let x = unsafe { cbn(bs.as_mut_ptr(), pka.as_ptr(), skb.as_ptr()) };
        let y = unsafe { rbn(bs_r.as_mut_ptr(), pka.as_ptr(), skb.as_ptr()) };
        eqi("beforenm symmetric rc", x, y);
        eqb("beforenm symmetric", &bs, &bs_r);
        eqb("beforenm(pkA, skB) == beforenm(pkB, skA)", &bs[..32], &bc[..32]);
    }

    // Both constructions fail on exactly the same inputs: the small-order
    // blocklist of `crypto_scalarmult_curve25519`.
    let sk: [u8; 32] = Rng::new(0xe102).bytes(32).try_into().unwrap();
    let pk = base.expect_ok(&sk, "pk");
    for hs in CURVE_SMALL_ORDER {
        let bad = h32(hs);
        let f = fill_of(0x5a);
        let mut kc = padded(32);
        let mut kr = padded(32);
        kc[..32].copy_from_slice(&f);
        kr[..32].copy_from_slice(&f);
        let x = unsafe { cbn(kc.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        let y = unsafe { rbn(kr.as_mut_ptr(), bad.as_ptr(), sk.as_ptr()) };
        eqi("beforenm small-order rc", x, y);
        eqi("beforenm small-order rc == -1", x, -1);
        eqb("beforenm small-order k", &kc, &kr);
        eqb("beforenm small-order leaves k untouched", &kc[..32], &f);

        let mut crx = padded(32);
        let mut ctx = padded(32);
        let mut rrx = padded(32);
        let mut rtx = padded(32);
        crx[..32].copy_from_slice(&f);
        ctx[..32].copy_from_slice(&f);
        rrx[..32].copy_from_slice(&f);
        rtx[..32].copy_from_slice(&f);
        let x = unsafe {
            ckc(
                crx.as_mut_ptr(),
                ctx.as_mut_ptr(),
                pk.as_ptr(),
                sk.as_ptr(),
                bad.as_ptr(),
            )
        };
        let y = unsafe {
            rkc(
                rrx.as_mut_ptr(),
                rtx.as_mut_ptr(),
                pk.as_ptr(),
                sk.as_ptr(),
                bad.as_ptr(),
            )
        };
        eqi("kx small-order rc", x, y);
        eqi("kx small-order rc == -1", x, -1);
        eqb("kx small-order rx", &crx, &rrx);
        eqb("kx small-order tx", &ctx, &rtx);
        eqb("kx small-order leaves rx untouched", &crx[..32], &f);
        eqb("kx small-order leaves tx untouched", &ctx[..32], &f);
    }
}

// ==================================================== configs 7.130
// Build-configuration invariant: the CMake build defines no `HAVE_*` macro and
// no `ED25519_COMPAT` / `ED25519_NONDETERMINISTIC`.  Asserted through the
// observable consequences rather than the macros themselves.

type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type SignDetached =
    unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type VerifyDetached = unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> c_int;
type KemKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type KemEnc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type KemDec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

#[test]
fn gap_build_configuration_invariants() {
    // (a) sandy2x is never selected: the AVX implementation table is not even
    // linked in, and `_pick_best_implementation` is a no-op that always
    // returns 0 and never changes a result.
    for absent in [
        "crypto_scalarmult_curve25519_sandy2x_implementation",
        "crypto_scalarmult_curve25519_sandy2x",
        "crypto_scalarmult_curve25519_sandy2x_base",
    ] {
        assert!(
            !has(absent),
            "`{absent}` must not exist on a build without HAVE_AVX_ASM"
        );
    }
    let (cp, rp) = both::<IntFn>("_crypto_scalarmult_curve25519_pick_best_implementation");
    let m = F3::new("crypto_scalarmult_curve25519");
    let n = h32("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4");
    let p = h32("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c");
    let (rc0, want, _) = m.run(&n, &p, 0x12);
    eqi("baseline rc", rc0, 0);
    // (b) `fe25519_sub_lazy` is plain `fe25519_sub` (no HAVE_TI_MODE): the
    // RFC 7748 §5.2 vector still comes out right.
    assert_eq!(
        hex(&want),
        "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552",
        "RFC 7748 X25519 vector 1"
    );
    for _ in 0..4 {
        let (a, b) = unsafe { (cp(), rp()) };
        eqi("pick_best_implementation", a, b);
        eqi("pick_best_implementation == 0", a, 0);
        let (rc, got, _) = m.run(&n, &p, 0x12);
        eqi("post-dispatch rc", rc, rc0);
        eqb("dispatch never changes a result", &got, &want);
    }

    // (c) `ED25519_COMPAT` is off, so `open.c:34-42` (the strict branch) is
    // live.  Observable consequence: the malleable signature `S + L` is
    // REJECTED.  Under `ED25519_COMPAT` the only guard would be
    // `sig[63] & 224`, which is 0 for `S + L < 2^253`, and the cofactored
    // equation still holds because `L·B` is the identity — so a COMPAT build
    // would accept it.
    let (csk, rsk) = both::<SeedKeypair>("crypto_sign_seed_keypair");
    let (csd, rsd) = both::<SignDetached>("crypto_sign_detached");
    let (cvd, rvd) = both::<VerifyDetached>("crypto_sign_verify_detached");
    let l = h32(L_HEX);
    let mut rng = Rng::new(0xf101);
    for i in 0..16 {
        let seed: [u8; 32] = rng.bytes(32).try_into().unwrap();
        let mut pkc = padded(32);
        let mut skc = padded(64);
        let mut pkr = padded(32);
        let mut skr = padded(64);
        unsafe {
            let a = csk(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr());
            let b = rsk(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr());
            eqi("seed_keypair rc", a, b);
        }
        eqb("seed_keypair pk", &pkc, &pkr);
        eqb("seed_keypair sk", &skc, &skr);

        let msg = rng.bytes(i * 7 + 1);
        let mut sigc = padded(64);
        let mut sigr = padded(64);
        let mut lc: u64 = 0;
        let mut lr: u64 = 0;
        unsafe {
            let a = csd(
                sigc.as_mut_ptr(),
                &mut lc,
                msg.as_ptr(),
                msg.len() as u64,
                skc.as_ptr(),
            );
            let b = rsd(
                sigr.as_mut_ptr(),
                &mut lr,
                msg.as_ptr(),
                msg.len() as u64,
                skr.as_ptr(),
            );
            eqi("sign rc", a, b);
        }
        assert_eq!(lc, 64);
        assert_eq!(lr, 64);
        eqb("signature", &sigc, &sigr);

        // (d) `ED25519_NONDETERMINISTIC` is off: signing is a pure function.
        let mut sig2 = padded(64);
        let mut l2: u64 = 0;
        unsafe {
            csd(
                sig2.as_mut_ptr(),
                &mut l2,
                msg.as_ptr(),
                msg.len() as u64,
                skc.as_ptr(),
            )
        };
        eqb("signing is deterministic", &sigc, &sig2);

        // The unmodified signature verifies.
        unsafe {
            let a = cvd(sigc.as_ptr(), msg.as_ptr(), msg.len() as u64, pkc.as_ptr());
            let b = rvd(sigr.as_ptr(), msg.as_ptr(), msg.len() as u64, pkr.as_ptr());
            eqi("verify rc", a, b);
            eqi("verify rc == 0", a, 0);
        }

        // S + L: still a valid cofactored solution, but non-canonical.
        let mut mal = sigc[..64].to_vec();
        let mut carry = 0u16;
        for j in 0..32 {
            let v = mal[32 + j] as u16 + l[j] as u16 + carry;
            mal[32 + j] = v as u8;
            carry = v >> 8;
        }
        assert_eq!(carry, 0, "S + L overflowed 32 bytes");
        assert_ne!(mal[63] & 240, 0, "S + L must trip the sig[63] & 240 gate");
        assert_eq!(mal[63] & 224, 0, "an ED25519_COMPAT build would accept this");
        unsafe {
            let a = cvd(mal.as_ptr(), msg.as_ptr(), msg.len() as u64, pkc.as_ptr());
            let b = rvd(mal.as_ptr(), msg.as_ptr(), msg.len() as u64, pkr.as_ptr());
            eqi("malleable S+L rc", a, b);
            eqi("strict build rejects S+L", a, -1);
        }
        // S == L exactly is likewise rejected by `sc25519_is_canonical`.
        let mut sig_l = sigc[..64].to_vec();
        sig_l[32..64].copy_from_slice(&l);
        unsafe {
            let a = cvd(sig_l.as_ptr(), msg.as_ptr(), msg.len() as u64, pkc.as_ptr());
            let b = rvd(sig_l.as_ptr(), msg.as_ptr(), msg.len() as u64, pkr.as_ptr());
            eqi("S == L rc", a, b);
            eqi("S == L rejected", a, -1);
        }
        // A non-canonical `pk` is rejected too (this check does not exist under
        // ED25519_COMPAT).
        let mut bad_pk = [0xffu8; 32];
        bad_pk[0] = 0xed;
        bad_pk[31] = 0x7f;
        unsafe {
            let a = cvd(sigc.as_ptr(), msg.as_ptr(), msg.len() as u64, bad_pk.as_ptr());
            let b = rvd(sigr.as_ptr(), msg.as_ptr(), msg.len() as u64, bad_pk.as_ptr());
            eqi("non-canonical pk rc", a, b);
            eqi("non-canonical pk rejected", a, -1);
        }
    }

    // (e) No `HAVE_TMMINTRIN_H` / AES-NI: the hardware AEAD is unavailable, so
    // the portable fallbacks are what every other row was evaluated against.
    let (cav, rav) = both::<IntFn>("crypto_aead_aes256gcm_is_available");
    unsafe {
        let (a, b) = (cav(), rav());
        eqi("aes256gcm_is_available", a, b);
        eqi("no HAVE_* macros -> AES-NI unavailable", a, 0);
    }

    // (f) `cmov`'s `HAVE_INLINE_ASM` optimisation barrier is absent, which must
    // not change behaviour: ML-KEM's implicit-rejection value stays a pure
    // function of (z, ct), so a tampered ciphertext decapsulates to the same
    // pseudorandom secret every time, identically in C and Rust.
    let (ckp, rkp) = both::<KemKeypair>("crypto_kem_mlkem768_keypair");
    let (cen, ren) = both::<KemEnc>("crypto_kem_mlkem768_enc");
    let (cde, rde) = both::<KemDec>("crypto_kem_mlkem768_dec");
    for i in 0..4 {
        rng_reset();
        let mut pkc = padded(1184);
        let mut skc = padded(2400);
        let mut pkr = padded(1184);
        let mut skr = padded(2400);
        unsafe {
            let a = ckp(pkc.as_mut_ptr(), skc.as_mut_ptr());
            let b = rkp(pkr.as_mut_ptr(), skr.as_mut_ptr());
            eqi("mlkem keypair rc", a, b);
        }
        eqb("mlkem pk", &pkc, &pkr);
        eqb("mlkem sk", &skc, &skr);

        let mut ctc = padded(1088);
        let mut ssc = padded(32);
        let mut ctr = padded(1088);
        let mut ssr = padded(32);
        unsafe {
            let a = cen(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pkc.as_ptr());
            let b = ren(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pkr.as_ptr());
            eqi("mlkem enc rc", a, b);
        }
        eqb("mlkem ct", &ctc, &ctr);
        eqb("mlkem ss", &ssc, &ssr);

        // Tamper, then decapsulate twice: implicit rejection, reproducible.
        let mut bad = ctc[..1088].to_vec();
        bad[(i * 271) % 1088] ^= 0x40;
        let mut d1 = padded(32);
        let mut d2 = padded(32);
        let mut e1 = padded(32);
        unsafe {
            let a = cde(d1.as_mut_ptr(), bad.as_ptr(), skc.as_ptr());
            let a2 = cde(d2.as_mut_ptr(), bad.as_ptr(), skc.as_ptr());
            let b = rde(e1.as_mut_ptr(), bad.as_ptr(), skr.as_ptr());
            eqi("mlkem dec rc", a, b);
            eqi("mlkem dec never fails", a, 0);
            eqi("mlkem dec rc (2nd)", a2, 0);
        }
        eqb("implicit rejection is deterministic", &d1, &d2);
        eqb("implicit rejection matches", &d1, &e1);
        assert_ne!(&d1[..32], &ssc[..32], "tampered ct gave the real secret");
    }
}
