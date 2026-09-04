//! Differential tests for the ed25519 low-level core:
//!   * `crypto_core/ed25519/ref10/ed25519_ref10.c`  (fe25519 / ge25519 / sc25519 / ristretto255)
//!   * `crypto_core/ed25519/core_ed25519.c`
//!   * `crypto_core/ed25519/core_ristretto255.c`
//!
//! Everything is called through `dlopen`/`dlsym` on BOTH shared objects and the
//! FULL output structures (raw `fe25519 = int32_t[10]` limbs, not just canonical
//! encodings) are compared byte/limb-for-byte.
#![allow(non_snake_case)]

#[macro_use]
mod common;

use core::ffi::c_int;

// ---------------------------------------------------------------- types ------
//
// `private/ed25519_ref10.h` with no HAVE_TI_MODE:
//   typedef int32_t fe25519[10];
//   ge25519_p2   { fe25519 X, Y, Z; }          -> 30 x i32 = 120 bytes
//   ge25519_p3   { fe25519 X, Y, Z, T; }       -> 40 x i32 = 160 bytes
//   ge25519_p1p1 { fe25519 X, Y, Z, T; }       -> 40 x i32 = 160 bytes
// (`ge25519_precomp` / `ge25519_cached` are only reachable through static
//  helpers, see the notes in _v/configs/ed25519low.md)
type Fe = [i32; 10];
type P2 = [i32; 30];
type P3 = [i32; 40];
type P1p1 = [i32; 40];

const FE_ZERO: Fe = [0; 10];

// ------------------------------------------------------------- utilities -----

fn h32(s: &str) -> [u8; 32] {
    assert_eq!(s.len(), 64);
    let mut o = [0u8; 32];
    for i in 0..32 {
        o[i] = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    o
}

fn eqlimbs(ctx: &str, c: &[i32], r: &[i32]) {
    if c != r {
        panic!("{}: limb mismatch\n  C   = {:?}\n  Rust= {:?}", ctx, c, r);
    }
}

/// A canary value so that "did not write" is distinguishable from "wrote 0".
const CANARY: i32 = 0x5A5A_5A5A_u32 as i32;

// ------------------------------------------------------------ symbol sets ----

macro_rules! fe_frombytes {
    () => {
        both!(
            "_sodium_fe25519_frombytes",
            unsafe extern "C" fn(*mut i32, *const u8)
        )
    };
}

fn c_fe_frombytes(b: &[u8; 32]) -> Fe {
    let f = getsym!(
        common::libs().c,
        "_sodium_fe25519_frombytes",
        unsafe extern "C" fn(*mut i32, *const u8)
    );
    let mut o = FE_ZERO;
    unsafe { f(o.as_mut_ptr(), b.as_ptr()) };
    o
}

fn c_scalarmult_base(a: &[u8; 32]) -> P3 {
    let f = getsym!(
        common::libs().c,
        "_sodium_ge25519_scalarmult_base",
        unsafe extern "C" fn(*mut i32, *const u8)
    );
    let mut p: P3 = [0; 40];
    unsafe { f(p.as_mut_ptr(), a.as_ptr()) };
    p
}

fn c_p3_tobytes(p: &P3) -> [u8; 32] {
    let f = getsym!(
        common::libs().c,
        "_sodium_ge25519_p3_tobytes",
        unsafe extern "C" fn(*mut u8, *const i32)
    );
    let mut s = [0u8; 32];
    unsafe { f(s.as_mut_ptr(), p.as_ptr()) };
    s
}

fn c_frombytes(s: &[u8; 32]) -> (c_int, P3) {
    let f = getsym!(
        common::libs().c,
        "_sodium_ge25519_frombytes",
        unsafe extern "C" fn(*mut i32, *const u8) -> c_int
    );
    let mut p: P3 = [0; 40];
    let rc = unsafe { f(p.as_mut_ptr(), s.as_ptr()) };
    (rc, p)
}

fn c_p3_add(p: &P3, q: &P3) -> P3 {
    let f = getsym!(
        common::libs().c,
        "_sodium_ge25519_p3_add",
        unsafe extern "C" fn(*mut i32, *const i32, *const i32)
    );
    let mut r: P3 = [0; 40];
    unsafe { f(r.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
    r
}

/// Valid, canonical, non-small-order encodings that are NOT in the prime-order
/// subgroup: `a*B + T` for every torsion point `T` that actually decodes with
/// `ge25519_frombytes` (y=0 both signs, and y=p-1).  These are the only inputs
/// that reach the `ge25519_is_on_main_subgroup(&p_p3) == 0` rejection in
/// `crypto_core_ed25519_is_valid_point`.
fn off_subgroup_encodings() -> Vec<[u8; 32]> {
    let torsion = [
        "0000000000000000000000000000000000000000000000000000000000000000",
        "0000000000000000000000000000000000000000000000000000000000000080",
        "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800",
        "e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b880",
        "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157",
        "5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f11d7",
    ];
    let mut out = Vec::new();
    let mut rng = common::Rng::new(0x7071_5310);
    for t in torsion {
        let (rct, tp3) = c_frombytes(&h32(t));
        if rct != 0 {
            continue;
        }
        for _ in 0..3 {
            let mut a = [0u8; 32];
            rng.fill(&mut a);
            a[31] &= 0x1f;
            let base = c_scalarmult_base(&a);
            out.push(c_p3_tobytes(&c_p3_add(&base, &tp3)));
        }
    }
    out
}

/// Tracks which distinct return values a predicate produced, so that a test
/// cannot silently become vacuous (e.g. "every input was rejected").
#[derive(Default)]
struct Seen(std::collections::BTreeSet<c_int>);
impl Seen {
    fn add(&mut self, v: c_int) -> c_int {
        self.0.insert(v);
        v
    }
    fn require(&self, what: &str, wanted: &[c_int]) {
        for w in wanted {
            assert!(
                self.0.contains(w),
                "{what}: return value {w} was never produced (saw {:?}) -- test corpus is too weak",
                self.0
            );
        }
    }
}

// ---------------------------------------------------------- test corpora -----

/// Interesting 32-byte point encodings: the identity, the decodable torsion
/// points, libsodium's small-order blocklist, non-canonical y values
/// (p-1, p, p+1, 2^255-1), the base point, and sign-bit variants of all of them.
fn special_encodings() -> Vec<[u8; 32]> {
    let mut v = vec![
        // y = 0  (order-4 point)
        h32("0000000000000000000000000000000000000000000000000000000000000000"),
        // y = 1  (identity / neutral element)
        h32("0100000000000000000000000000000000000000000000000000000000000000"),
        // y = 2  (not on the curve)
        h32("0200000000000000000000000000000000000000000000000000000000000000"),
        // y = p-1 = -1  (order-2 point)
        h32("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        // y = p  (non-canonical encoding of y = 0)
        h32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        // y = p+1  (non-canonical encoding of the identity)
        h32("eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f"),
        // two entries of libsodium's small-order blocklist (these particular
        // byte patterns do NOT decode with ge25519_frombytes)
        h32("e0eb7a7c3b41b8ae1656e3faf19fc46ada098deb9c32b1fd866205165f49b800"),
        h32("5f9c95bca3508c24b1d0b1559c83ef5b04445cc4581c8e86d8224eddd09f1157"),
        // more blocklist entries (non-canonical / high bit set)
        h32("d9ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        h32("ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        h32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        h32("eeffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        // 2^255-1
        h32("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        // the Ed25519 base point B
        h32("5866666666666666666666666666666666666666666666666666666666666666"),
    ];
    let n = v.len();
    for i in 0..n {
        let mut x = v[i];
        x[31] ^= 0x80;
        v.push(x);
    }
    v
}

/// Interesting 32-byte scalars: 0, 1, 2, L-1, L, L+1, 2^252, 2^252+..., 0x7f..,
/// 0xff.., single-bit patterns.
fn special_scalars() -> Vec<[u8; 32]> {
    let l = h32("edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010");
    let mut lm1 = l;
    lm1[0] -= 1;
    let mut lp1 = l;
    lp1[0] += 1;
    let mut v = vec![
        [0u8; 32],
        {
            let mut a = [0u8; 32];
            a[0] = 1;
            a
        },
        {
            let mut a = [0u8; 32];
            a[0] = 2;
            a
        },
        lm1,
        l,
        lp1,
        // 2^252
        {
            let mut a = [0u8; 32];
            a[31] = 0x10;
            a
        },
        // 2^252 - 1
        {
            let mut a = [0xffu8; 32];
            a[31] = 0x0f;
            a
        },
        // largest with the top 3 bits clear
        {
            let mut a = [0xffu8; 32];
            a[31] = 0x1f;
            a
        },
        [0x7fu8; 32],
        [0xffu8; 32],
        [0x55u8; 32],
        [0xaau8; 32],
    ];
    // single-bit scalars
    for bit in [0usize, 1, 7, 8, 127, 128, 250, 251, 252, 253, 254, 255] {
        let mut a = [0u8; 32];
        a[bit / 8] = 1 << (bit % 8);
        v.push(a);
    }
    v
}

/// Interesting 64-byte non-reduced scalars.
fn special_scalars64() -> Vec<[u8; 64]> {
    let mut v: Vec<[u8; 64]> = Vec::new();
    let push32 = |v: &mut Vec<[u8; 64]>, s: &[u8; 32]| {
        let mut a = [0u8; 64];
        a[..32].copy_from_slice(s);
        v.push(a);
    };
    for s in special_scalars() {
        push32(&mut v, &s);
    }
    v.push([0u8; 64]);
    v.push([0xffu8; 64]);
    v.push([0x80u8; 64]);
    // 2^512 - 1 with only the top byte set etc.
    let mut a = [0u8; 64];
    a[63] = 0xff;
    v.push(a);
    let mut a = [0u8; 64];
    a[63] = 0x01;
    v.push(a);
    let mut a = [0u8; 64];
    a[32] = 0x01;
    v.push(a);
    let mut a = [0xffu8; 64];
    a[63] = 0x0f;
    v.push(a);
    v
}

/// A set of `ge25519_p3` values: valid points, the identity, torsion points and
/// "synthetic" structures built from arbitrary reduced field elements (the
/// arithmetic routines are total functions on limbs, so this is well defined).
fn p3_corpus(rng: &mut common::Rng) -> Vec<(String, P3)> {
    let mut out: Vec<(String, P3)> = Vec::new();

    // valid points a*B
    for (i, a) in special_scalars().iter().enumerate() {
        out.push((format!("scalarmult_base[{i}]"), c_scalarmult_base(a)));
    }
    for i in 0..16 {
        let mut a = [0u8; 32];
        rng.fill(&mut a);
        a[31] &= 0x1f;
        out.push((format!("rand_base[{i}]"), c_scalarmult_base(&a)));
    }
    // decoded encodings (including the ones that fail to decode: the partially
    // written struct is still deterministic because we zero-initialise it)
    for (i, s) in special_encodings().iter().enumerate() {
        let (_rc, p) = c_frombytes(s);
        out.push((format!("frombytes_special[{i}]"), p));
    }
    // valid-but-outside-the-prime-order-subgroup points
    for (i, s) in off_subgroup_encodings().iter().enumerate() {
        let (_rc, p) = c_frombytes(s);
        out.push((format!("off_subgroup[{i}]"), p));
    }
    // synthetic: every limb group is a reduced field element
    for i in 0..16 {
        let mut p: P3 = [0; 40];
        for g in 0..4 {
            let mut b = [0u8; 32];
            rng.fill(&mut b);
            let fe = c_fe_frombytes(&b);
            p[g * 10..g * 10 + 10].copy_from_slice(&fe);
        }
        out.push((format!("synthetic_p3[{i}]"), p));
    }
    // all-zero structure
    out.push(("zero_p3".to_string(), [0i32; 40]));
    out
}

// ============================================================================
// fe25519
// ============================================================================

#[test]
fn fe25519_frombytes_tobytes_invert() {
    let (cfb, rfb) = fe_frombytes!();
    let (ctb, rtb) = both!(
        "_sodium_fe25519_tobytes",
        unsafe extern "C" fn(*mut u8, *const i32)
    );
    let (cinv, rinv) = both!(
        "_sodium_fe25519_invert",
        unsafe extern "C" fn(*mut i32, *const i32)
    );

    let mut inputs: Vec<[u8; 32]> = special_encodings();
    inputs.push([0u8; 32]);
    let mut rng = common::Rng::new(0xFE_2551_9001);
    for _ in 0..160 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        inputs.push(b);
    }
    // p-1, p, p+1, 2p ...
    for k in 0u8..4 {
        let mut b = h32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f");
        b[0] = b[0].wrapping_add(k);
        inputs.push(b);
    }

    for (i, b) in inputs.iter().enumerate() {
        let ctx = format!("fe25519 case {i} in={}", common::hex(b));
        let (mut cf, mut rf) = ([CANARY; 10], [CANARY; 10]);
        unsafe { cfb(cf.as_mut_ptr(), b.as_ptr()) };
        unsafe { rfb(rf.as_mut_ptr(), b.as_ptr()) };
        eqlimbs(&format!("{ctx}: frombytes"), &cf, &rf);

        let (mut cs, mut rs) = ([0xAAu8; 32], [0xAAu8; 32]);
        unsafe { ctb(cs.as_mut_ptr(), cf.as_ptr()) };
        unsafe { rtb(rs.as_mut_ptr(), rf.as_ptr()) };
        common::eqb(&format!("{ctx}: tobytes"), &cs, &rs);

        let (mut ci, mut ri) = ([CANARY; 10], [CANARY; 10]);
        unsafe { cinv(ci.as_mut_ptr(), cf.as_ptr()) };
        unsafe { rinv(ri.as_mut_ptr(), rf.as_ptr()) };
        eqlimbs(&format!("{ctx}: invert"), &ci, &ri);

        // tobytes of the inverse, too
        let (mut cs2, mut rs2) = ([0x55u8; 32], [0x55u8; 32]);
        unsafe { ctb(cs2.as_mut_ptr(), ci.as_ptr()) };
        unsafe { rtb(rs2.as_mut_ptr(), ri.as_ptr()) };
        common::eqb(&format!("{ctx}: tobytes(invert)"), &cs2, &rs2);
    }

    // invert(0) == 0 in both (documented degenerate case)
    let (mut ci, mut ri) = ([CANARY; 10], [CANARY; 10]);
    unsafe { cinv(ci.as_mut_ptr(), FE_ZERO.as_ptr()) };
    unsafe { rinv(ri.as_mut_ptr(), FE_ZERO.as_ptr()) };
    eqlimbs("fe25519_invert(0)", &ci, &ri);

    // aliasing: invert in place
    let mut cz = c_fe_frombytes(&h32(
        "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f00",
    ));
    let mut rz = cz;
    unsafe { cinv(cz.as_mut_ptr(), cz.as_ptr()) };
    unsafe { rinv(rz.as_mut_ptr(), rz.as_ptr()) };
    eqlimbs("fe25519_invert aliased", &cz, &rz);
}

// ============================================================================
// ge25519 decode / encode
// ============================================================================

#[test]
fn ge25519_frombytes_variants() {
    let (cfb, rfb) = both!(
        "_sodium_ge25519_frombytes",
        unsafe extern "C" fn(*mut i32, *const u8) -> c_int
    );
    let (cfn, rfn) = both!(
        "_sodium_ge25519_frombytes_negate_vartime",
        unsafe extern "C" fn(*mut i32, *const u8) -> c_int
    );

    let mut inputs = special_encodings();
    // valid points
    let mut rng = common::Rng::new(0x6E_2551_9002);
    for _ in 0..64 {
        let mut a = [0u8; 32];
        rng.fill(&mut a);
        a[31] &= 0x1f;
        inputs.push(c_p3_tobytes(&c_scalarmult_base(&a)));
    }
    // random garbage (mostly not on the curve)
    for _ in 0..160 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        inputs.push(b);
    }
    // valid encodings with the sign bit flipped
    for i in 0..16 {
        let mut a = [0u8; 32];
        rng.fill(&mut a);
        a[31] &= 0x1f;
        let mut s = c_p3_tobytes(&c_scalarmult_base(&a));
        s[31] ^= 0x80;
        inputs.push(s);
        let _ = i;
    }

    inputs.extend(off_subgroup_encodings());

    let mut seen_fb = Seen::default();
    let mut seen_fn = Seen::default();
    for (i, s) in inputs.iter().enumerate() {
        let ctx = format!("ge25519_frombytes case {i} s={}", common::hex(s));
        // NOTE: on the failure path the C only writes part of the struct, so
        // both buffers are pre-filled with the same canary and compared in
        // full -- this catches spurious/missing writes too.
        let (mut cp, mut rp): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        let crc = unsafe { cfb(cp.as_mut_ptr(), s.as_ptr()) };
        let rrc = unsafe { rfb(rp.as_mut_ptr(), s.as_ptr()) };
        common::eqi(&format!("{ctx}: rc"), seen_fb.add(crc), rrc);
        eqlimbs(&format!("{ctx}: p3"), &cp, &rp);

        let (mut cq, mut rq): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        let crc2 = unsafe { cfn(cq.as_mut_ptr(), s.as_ptr()) };
        let rrc2 = unsafe { rfn(rq.as_mut_ptr(), s.as_ptr()) };
        common::eqi(&format!("{ctx}: negate_vartime rc"), seen_fn.add(crc2), rrc2);
        eqlimbs(&format!("{ctx}: negate_vartime p3"), &cq, &rq);
    }
    seen_fb.require("ge25519_frombytes", &[0, -1]);
    seen_fn.require("ge25519_frombytes_negate_vartime", &[0, -1]);
}

#[test]
fn ge25519_tobytes_and_p3_tobytes() {
    let (ctb, rtb) = both!(
        "_sodium_ge25519_p3_tobytes",
        unsafe extern "C" fn(*mut u8, *const i32)
    );
    let (ct2, rt2) = both!(
        "_sodium_ge25519_tobytes",
        unsafe extern "C" fn(*mut u8, *const i32)
    );
    let (cdsm, _rdsm) = both!(
        "_sodium_ge25519_double_scalarmult_vartime",
        unsafe extern "C" fn(*mut i32, *const u8, *const i32, *const u8)
    );

    let mut rng = common::Rng::new(0x6E_2551_9003);
    for (name, p) in p3_corpus(&mut rng) {
        let (mut cs, mut rs) = ([0x33u8; 32], [0x33u8; 32]);
        unsafe { ctb(cs.as_mut_ptr(), p.as_ptr()) };
        unsafe { rtb(rs.as_mut_ptr(), p.as_ptr()) };
        common::eqb(&format!("ge25519_p3_tobytes {name}"), &cs, &rs);
    }

    // ge25519_tobytes takes a ge25519_p2: build them from
    // double_scalarmult_vartime output and from synthetic field elements.
    let mut p2s: Vec<(String, P2)> = Vec::new();
    for i in 0..12 {
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        rng.fill(&mut a);
        rng.fill(&mut b);
        a[31] &= 0x1f;
        b[31] &= 0x1f;
        let A = c_scalarmult_base(&a);
        let mut r: P2 = [0; 30];
        unsafe { cdsm(r.as_mut_ptr(), a.as_ptr(), A.as_ptr(), b.as_ptr()) };
        p2s.push((format!("dsm_p2[{i}]"), r));
    }
    for i in 0..16 {
        let mut r: P2 = [0; 30];
        for g in 0..3 {
            let mut bb = [0u8; 32];
            rng.fill(&mut bb);
            let fe = c_fe_frombytes(&bb);
            r[g * 10..g * 10 + 10].copy_from_slice(&fe);
        }
        p2s.push((format!("synthetic_p2[{i}]"), r));
    }
    p2s.push(("zero_p2".to_string(), [0i32; 30]));

    for (name, p) in &p2s {
        let (mut cs, mut rs) = ([0x33u8; 32], [0x33u8; 32]);
        unsafe { ct2(cs.as_mut_ptr(), p.as_ptr()) };
        unsafe { rt2(rs.as_mut_ptr(), p.as_ptr()) };
        common::eqb(&format!("ge25519_tobytes {name}"), &cs, &rs);
    }
}

#[test]
fn ge25519_representation_conversions() {
    let (cp1p2, rp1p2) = both!(
        "_sodium_ge25519_p1p1_to_p2",
        unsafe extern "C" fn(*mut i32, *const i32)
    );
    let (cp1p3, rp1p3) = both!(
        "_sodium_ge25519_p1p1_to_p3",
        unsafe extern "C" fn(*mut i32, *const i32)
    );
    let (cp2p3, rp2p3) = both!(
        "_sodium_ge25519_p2_to_p3",
        unsafe extern "C" fn(*mut i32, *const i32)
    );

    let mut rng = common::Rng::new(0x6E_2551_9004);

    // ge25519_p1p1 values: synthetic (all four coordinates are reduced field
    // elements) plus degenerate all-zero / one-hot cases.
    let mut p1p1s: Vec<(String, P1p1)> = Vec::new();
    for i in 0..128 {
        let mut p: P1p1 = [0; 40];
        for g in 0..4 {
            let mut b = [0u8; 32];
            rng.fill(&mut b);
            let fe = c_fe_frombytes(&b);
            p[g * 10..g * 10 + 10].copy_from_slice(&fe);
        }
        p1p1s.push((format!("rand_p1p1[{i}]"), p));
    }
    p1p1s.push(("zero_p1p1".to_string(), [0i32; 40]));
    for g in 0..4 {
        let mut p: P1p1 = [0; 40];
        p[g * 10] = 1;
        p1p1s.push((format!("onehot_p1p1[{g}]"), p));
    }

    for (name, p) in &p1p1s {
        let (mut c2, mut r2): (P2, P2) = ([CANARY; 30], [CANARY; 30]);
        unsafe { cp1p2(c2.as_mut_ptr(), p.as_ptr()) };
        unsafe { rp1p2(r2.as_mut_ptr(), p.as_ptr()) };
        eqlimbs(&format!("p1p1_to_p2 {name}"), &c2, &r2);

        let (mut c3, mut r3): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        unsafe { cp1p3(c3.as_mut_ptr(), p.as_ptr()) };
        unsafe { rp1p3(r3.as_mut_ptr(), p.as_ptr()) };
        eqlimbs(&format!("p1p1_to_p3 {name}"), &c3, &r3);

        // and p2_to_p3 on the freshly produced p2
        let (mut c4, mut r4): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        unsafe { cp2p3(c4.as_mut_ptr(), c2.as_ptr()) };
        unsafe { rp2p3(r4.as_mut_ptr(), r2.as_ptr()) };
        eqlimbs(&format!("p2_to_p3 {name}"), &c4, &r4);
    }

    // p2_to_p3 on real projective points as well
    let (cdsm, _) = both!(
        "_sodium_ge25519_double_scalarmult_vartime",
        unsafe extern "C" fn(*mut i32, *const u8, *const i32, *const u8)
    );
    for i in 0..16 {
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        rng.fill(&mut a);
        rng.fill(&mut b);
        a[31] &= 0x1f;
        b[31] &= 0x1f;
        let A = c_scalarmult_base(&a);
        let mut p2: P2 = [0; 30];
        unsafe { cdsm(p2.as_mut_ptr(), a.as_ptr(), A.as_ptr(), b.as_ptr()) };
        let (mut c3, mut r3): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        unsafe { cp2p3(c3.as_mut_ptr(), p2.as_ptr()) };
        unsafe { rp2p3(r3.as_mut_ptr(), p2.as_ptr()) };
        eqlimbs(&format!("p2_to_p3 real[{i}]"), &c3, &r3);
    }
}

#[test]
fn ge25519_add_sub_and_clear_cofactor() {
    let (cadd, radd) = both!(
        "_sodium_ge25519_p3_add",
        unsafe extern "C" fn(*mut i32, *const i32, *const i32)
    );
    let (csub, rsub) = both!(
        "_sodium_ge25519_p3_sub",
        unsafe extern "C" fn(*mut i32, *const i32, *const i32)
    );
    let (ccc, rcc) = both!(
        "_sodium_ge25519_clear_cofactor",
        unsafe extern "C" fn(*mut i32)
    );

    let mut rng = common::Rng::new(0x6E_2551_9005);
    let corpus = p3_corpus(&mut rng);

    // all pairs would be O(n^2); take a deterministic sample plus p+p / p-p.
    for (i, (n1, p)) in corpus.iter().enumerate() {
        // self add/sub
        for (op, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
            let (mut cr, mut rr): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
            unsafe { cf(cr.as_mut_ptr(), p.as_ptr(), p.as_ptr()) };
            unsafe { rf(rr.as_mut_ptr(), p.as_ptr(), p.as_ptr()) };
            eqlimbs(&format!("p3_{op} self {n1}"), &cr, &rr);
        }
        let j = (i * 7 + 3) % corpus.len();
        let k = (i * 13 + 5) % corpus.len();
        for jj in [j, k] {
            let (n2, q) = &corpus[jj];
            for (op, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
                let (mut cr, mut rr): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
                unsafe { cf(cr.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
                unsafe { rf(rr.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
                eqlimbs(&format!("p3_{op} {n1} , {n2}"), &cr, &rr);
            }
        }
        // clear_cofactor operates in place
        let mut cp = *p;
        let mut rp = *p;
        unsafe { ccc(cp.as_mut_ptr()) };
        unsafe { rcc(rp.as_mut_ptr()) };
        eqlimbs(&format!("clear_cofactor {n1}"), &cp, &rp);
    }

    // extra random pair sample
    for i in 0..128 {
        let a = &corpus[rng.below(corpus.len())].1;
        let b = &corpus[rng.below(corpus.len())].1;
        let (mut cr, mut rr): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        unsafe { cadd(cr.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        unsafe { radd(rr.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        eqlimbs(&format!("p3_add rand pair {i}"), &cr, &rr);
        let (mut cr, mut rr): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        unsafe { csub(cr.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        unsafe { rsub(rr.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        eqlimbs(&format!("p3_sub rand pair {i}"), &cr, &rr);
    }
}

#[test]
fn ge25519_scalarmults() {
    let (csb, rsb) = both!(
        "_sodium_ge25519_scalarmult_base",
        unsafe extern "C" fn(*mut i32, *const u8)
    );
    let (csm, rsm) = both!(
        "_sodium_ge25519_scalarmult",
        unsafe extern "C" fn(*mut i32, *const u8, *const i32)
    );
    let (cdsm, rdsm) = both!(
        "_sodium_ge25519_double_scalarmult_vartime",
        unsafe extern "C" fn(*mut i32, *const u8, *const i32, *const u8)
    );

    let mut rng = common::Rng::new(0x6E_2551_9006);

    // ---- scalarmult_base ----
    let mut scalars = special_scalars();
    for _ in 0..96 {
        let mut a = [0u8; 32];
        rng.fill(&mut a);
        scalars.push(a); // deliberately includes a[31] > 127
    }
    for _ in 0..32 {
        let mut a = [0u8; 32];
        rng.fill(&mut a);
        a[31] &= 0x1f;
        scalars.push(a);
    }
    for (i, a) in scalars.iter().enumerate() {
        let (mut cp, mut rp): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        unsafe { csb(cp.as_mut_ptr(), a.as_ptr()) };
        unsafe { rsb(rp.as_mut_ptr(), a.as_ptr()) };
        eqlimbs(
            &format!("scalarmult_base {i} a={}", common::hex(a)),
            &cp,
            &rp,
        );
    }

    // ---- scalarmult / double_scalarmult_vartime ----
    let corpus = p3_corpus(&mut rng);
    for (i, a) in scalars.iter().enumerate().take(40) {
        for (j, (name, p)) in corpus.iter().enumerate() {
            if (i + j) % 5 != 0 {
                continue;
            }
            let (mut cp, mut rp): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
            unsafe { csm(cp.as_mut_ptr(), a.as_ptr(), p.as_ptr()) };
            unsafe { rsm(rp.as_mut_ptr(), a.as_ptr(), p.as_ptr()) };
            eqlimbs(&format!("scalarmult a[{i}] * {name}"), &cp, &rp);
        }
    }
    for i in 0..64 {
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        rng.fill(&mut a);
        rng.fill(&mut b);
        let (name, A) = &corpus[rng.below(corpus.len())];
        let (mut cp, mut rp): (P2, P2) = ([CANARY; 30], [CANARY; 30]);
        unsafe { cdsm(cp.as_mut_ptr(), a.as_ptr(), A.as_ptr(), b.as_ptr()) };
        unsafe { rdsm(rp.as_mut_ptr(), a.as_ptr(), A.as_ptr(), b.as_ptr()) };
        eqlimbs(
            &format!("double_scalarmult_vartime rand {i} A={name}"),
            &cp,
            &rp,
        );
    }
    // boundary scalars for the sliding-window code: 0, 1, all-ff, alternating
    let dsm_scalars = [
        [0u8; 32],
        {
            let mut a = [0u8; 32];
            a[0] = 1;
            a
        },
        [0xffu8; 32],
        [0x55u8; 32],
        [0xaau8; 32],
        [0x7fu8; 32],
        [0x80u8; 32],
        h32("edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010"),
    ];
    let A = c_scalarmult_base(&{
        let mut a = [0u8; 32];
        a[0] = 9;
        a
    });
    for (i, a) in dsm_scalars.iter().enumerate() {
        for (j, b) in dsm_scalars.iter().enumerate() {
            let (mut cp, mut rp): (P2, P2) = ([CANARY; 30], [CANARY; 30]);
            unsafe { cdsm(cp.as_mut_ptr(), a.as_ptr(), A.as_ptr(), b.as_ptr()) };
            unsafe { rdsm(rp.as_mut_ptr(), a.as_ptr(), A.as_ptr(), b.as_ptr()) };
            eqlimbs(&format!("double_scalarmult_vartime edge {i},{j}"), &cp, &rp);
        }
    }
}

#[test]
fn ge25519_predicates() {
    let (cic, ric) = both!(
        "_sodium_ge25519_is_canonical",
        unsafe extern "C" fn(*const u8) -> c_int
    );
    let (coc, roc) = both!(
        "_sodium_ge25519_is_on_curve",
        unsafe extern "C" fn(*const i32) -> c_int
    );
    let (cms, rms) = both!(
        "_sodium_ge25519_is_on_main_subgroup",
        unsafe extern "C" fn(*const i32) -> c_int
    );
    let (cso, rso) = both!(
        "_sodium_ge25519_has_small_order",
        unsafe extern "C" fn(*const i32) -> c_int
    );

    // is_canonical works on encodings
    let mut encs = special_encodings();
    let mut rng = common::Rng::new(0x6E_2551_9007);
    for _ in 0..160 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        encs.push(b);
    }
    // exercise the exact boundary of the canonicality test: s = p-1, p, p+1,
    // and s[0] sweeping 0x00..0xff with all other bytes 0xff/0x7f
    for v in 0u16..=255 {
        let mut b = [0xffu8; 32];
        b[31] = 0x7f;
        b[0] = v as u8;
        encs.push(b);
        let mut b2 = b;
        b2[31] = 0xff;
        encs.push(b2);
    }
    // one byte in the middle not 0xff => canonical regardless of s[0]
    for i in 1..31 {
        let mut b = [0xffu8; 32];
        b[31] = 0x7f;
        b[i] = 0xfe;
        b[0] = 0x00;
        encs.push(b);
    }
    let mut s_ic = Seen::default();
    for (i, s) in encs.iter().enumerate() {
        let crc = unsafe { cic(s.as_ptr()) };
        let rrc = unsafe { ric(s.as_ptr()) };
        common::eqi(
            &format!("ge25519_is_canonical {i} s={}", common::hex(s)),
            s_ic.add(crc),
            rrc,
        );
    }
    s_ic.require("ge25519_is_canonical", &[0, 1]);

    let corpus = p3_corpus(&mut rng);
    let (mut s_oc, mut s_ms, mut s_so) = (Seen::default(), Seen::default(), Seen::default());
    for (name, p) in &corpus {
        common::eqi(
            &format!("is_on_curve {name}"),
            s_oc.add(unsafe { coc(p.as_ptr()) }),
            unsafe { roc(p.as_ptr()) },
        );
        common::eqi(
            &format!("is_on_main_subgroup {name}"),
            s_ms.add(unsafe { cms(p.as_ptr()) }),
            unsafe { rms(p.as_ptr()) },
        );
        common::eqi(
            &format!("has_small_order {name}"),
            s_so.add(unsafe { cso(p.as_ptr()) }),
            unsafe { rso(p.as_ptr()) },
        );
    }
    s_oc.require("ge25519_is_on_curve", &[0, 1]);
    s_ms.require("ge25519_is_on_main_subgroup", &[0, 1]);
    s_so.require("ge25519_has_small_order", &[0, 1]);
}

#[test]
fn ge25519_from_uniform_and_from_hash() {
    let (cfu, rfu) = both!(
        "_sodium_ge25519_from_uniform",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (cfh, rfh) = both!(
        "_sodium_ge25519_from_hash",
        unsafe extern "C" fn(*mut u8, *const u8)
    );

    let mut rng = common::Rng::new(0x6E_2551_9008);
    let mut us: Vec<[u8; 32]> = special_encodings();
    us.push([0u8; 32]);
    us.push([0xffu8; 32]);
    for _ in 0..160 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        us.push(b);
    }
    for (i, r) in us.iter().enumerate() {
        let (mut cs, mut rs) = ([0x11u8; 32], [0x11u8; 32]);
        unsafe { cfu(cs.as_mut_ptr(), r.as_ptr()) };
        unsafe { rfu(rs.as_mut_ptr(), r.as_ptr()) };
        common::eqb(
            &format!("ge25519_from_uniform {i} r={}", common::hex(r)),
            &cs,
            &rs,
        );
    }

    let mut hs: Vec<[u8; 64]> = vec![[0u8; 64], [0xffu8; 64], [0x80u8; 64]];
    for _ in 0..160 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        hs.push(b);
    }
    // high-bit / reduction boundaries in fe25519_reduce64
    for k in 0..4 {
        let mut b = [0u8; 64];
        b[31] = if k & 1 == 0 { 0x00 } else { 0xff };
        b[63] = if k & 2 == 0 { 0x00 } else { 0xff };
        for i in 0..31 {
            b[i] = 0xff;
            b[32 + i] = 0xff;
        }
        hs.push(b);
    }
    for (i, h) in hs.iter().enumerate() {
        let (mut cs, mut rs) = ([0x11u8; 32], [0x11u8; 32]);
        unsafe { cfh(cs.as_mut_ptr(), h.as_ptr()) };
        unsafe { rfh(rs.as_mut_ptr(), h.as_ptr()) };
        common::eqb(&format!("ge25519_from_hash {i}"), &cs, &rs);
    }
}

// ============================================================================
// sc25519
// ============================================================================

#[test]
fn sc25519_reduce_mul_muladd() {
    let (cred, rred) = both!("_sodium_sc25519_reduce", unsafe extern "C" fn(*mut u8));
    let (cmul, rmul) = both!(
        "_sodium_sc25519_mul",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8)
    );
    let (cma, rma) = both!(
        "_sodium_sc25519_muladd",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8)
    );

    let mut rng = common::Rng::new(0x5C_2551_9009);

    // ---- sc25519_reduce: in-place on a 64-byte buffer; the C only writes the
    // low 32 bytes, so the whole 64-byte buffer is compared.
    let mut r64 = special_scalars64();
    for _ in 0..200 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        r64.push(b);
    }
    for (i, s) in r64.iter().enumerate() {
        let (mut cb, mut rb) = (*s, *s);
        unsafe { cred(cb.as_mut_ptr()) };
        unsafe { rred(rb.as_mut_ptr()) };
        common::eqb(
            &format!("sc25519_reduce {i} in={}", common::hex(s)),
            &cb,
            &rb,
        );
    }

    // ---- sc25519_mul / sc25519_muladd
    let mut s32 = special_scalars();
    for _ in 0..200 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        s32.push(b);
    }
    for i in 0..s32.len() {
        let a = &s32[i];
        let b = &s32[(i * 5 + 1) % s32.len()];
        let c = &s32[(i * 11 + 7) % s32.len()];
        let (mut co, mut ro) = ([0x77u8; 32], [0x77u8; 32]);
        unsafe { cmul(co.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        unsafe { rmul(ro.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        common::eqb(
            &format!(
                "sc25519_mul {i} a={} b={}",
                common::hex(a),
                common::hex(b)
            ),
            &co,
            &ro,
        );

        let (mut co, mut ro) = ([0x77u8; 32], [0x77u8; 32]);
        unsafe { cma(co.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr()) };
        unsafe { rma(ro.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr()) };
        common::eqb(
            &format!(
                "sc25519_muladd {i} a={} b={} c={}",
                common::hex(a),
                common::hex(b),
                common::hex(c)
            ),
            &co,
            &ro,
        );
    }
    // fully random triples too
    for i in 0..200 {
        let (mut a, mut b, mut c) = ([0u8; 32], [0u8; 32], [0u8; 32]);
        rng.fill(&mut a);
        rng.fill(&mut b);
        rng.fill(&mut c);
        let (mut co, mut ro) = ([0x77u8; 32], [0x77u8; 32]);
        unsafe { cmul(co.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        unsafe { rmul(ro.as_mut_ptr(), a.as_ptr(), b.as_ptr()) };
        common::eqb(&format!("sc25519_mul rand {i}"), &co, &ro);
        let (mut co, mut ro) = ([0x77u8; 32], [0x77u8; 32]);
        unsafe { cma(co.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr()) };
        unsafe { rma(ro.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr()) };
        common::eqb(&format!("sc25519_muladd rand {i}"), &co, &ro);
    }
    // aliasing: s == a
    for i in 0..32 {
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        rng.fill(&mut a);
        rng.fill(&mut b);
        let (mut ca, mut ra) = (a, a);
        unsafe { cmul(ca.as_mut_ptr(), ca.as_ptr(), b.as_ptr()) };
        unsafe { rmul(ra.as_mut_ptr(), ra.as_ptr(), b.as_ptr()) };
        common::eqb(&format!("sc25519_mul aliased {i}"), &ca, &ra);
    }
}

#[test]
fn sc25519_invert_and_is_canonical() {
    let (cinv, rinv) = both!(
        "_sodium_sc25519_invert",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (cic, ric) = both!(
        "_sodium_sc25519_is_canonical",
        unsafe extern "C" fn(*const u8) -> c_int
    );

    let mut rng = common::Rng::new(0x5C_2551_900A);
    let mut s32 = special_scalars();
    for _ in 0..200 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        s32.push(b);
    }
    // walk s[0] over the L[0] boundary with the rest of the bytes equal to L
    let l = h32("edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010");
    for v in 0u16..=255 {
        let mut b = l;
        b[0] = v as u8;
        s32.push(b);
    }
    for i in 0..32 {
        let mut b = l;
        b[i] = b[i].wrapping_sub(1);
        s32.push(b);
        let mut b = l;
        b[i] = b[i].wrapping_add(1);
        s32.push(b);
    }

    let mut seen = Seen::default();
    for (i, s) in s32.iter().enumerate() {
        common::eqi(
            &format!("sc25519_is_canonical {i} s={}", common::hex(s)),
            seen.add(unsafe { cic(s.as_ptr()) }),
            unsafe { ric(s.as_ptr()) },
        );
        let (mut co, mut ro) = ([0x99u8; 32], [0x99u8; 32]);
        unsafe { cinv(co.as_mut_ptr(), s.as_ptr()) };
        unsafe { rinv(ro.as_mut_ptr(), s.as_ptr()) };
        common::eqb(
            &format!("sc25519_invert {i} s={}", common::hex(s)),
            &co,
            &ro,
        );
    }
    seen.require("sc25519_is_canonical", &[0, 1]);
}

// ============================================================================
// ristretto255 (internal)
// ============================================================================

#[test]
fn ristretto255_internal() {
    let (cfb, rfb) = both!(
        "_sodium_ristretto255_frombytes",
        unsafe extern "C" fn(*mut i32, *const u8) -> c_int
    );
    let (ctb, rtb) = both!(
        "_sodium_ristretto255_p3_tobytes",
        unsafe extern "C" fn(*mut u8, *const i32)
    );
    let (cfh, rfh) = both!(
        "_sodium_ristretto255_from_hash",
        unsafe extern "C" fn(*mut u8, *const u8)
    );

    let mut rng = common::Rng::new(0x21_2551_900B);

    // ---- from_hash over 64-byte inputs
    let mut hs: Vec<[u8; 64]> = vec![[0u8; 64], [0xffu8; 64], [0x80u8; 64], [0x7fu8; 64]];
    for _ in 0..160 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        hs.push(b);
    }
    let mut valid: Vec<[u8; 32]> = Vec::new();
    for (i, h) in hs.iter().enumerate() {
        let (mut cs, mut rs) = ([0x22u8; 32], [0x22u8; 32]);
        unsafe { cfh(cs.as_mut_ptr(), h.as_ptr()) };
        unsafe { rfh(rs.as_mut_ptr(), h.as_ptr()) };
        common::eqb(&format!("ristretto255_from_hash {i}"), &cs, &rs);
        valid.push(cs);
    }

    // ---- frombytes: valid, non-canonical and invalid encodings
    let mut encs: Vec<[u8; 32]> = valid.clone();
    encs.extend(special_encodings());
    encs.push([0u8; 32]); // canonical encoding of the ristretto identity
    for _ in 0..200 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        encs.push(b);
    }
    // deliberately non-canonical variants of valid encodings:
    //   * high bit set                 (e != 0)
    //   * s[0] made odd                (s[0] & 1)
    //   * s >= p
    for i in 0..16 {
        let mut s = valid[i];
        s[31] |= 0x80;
        encs.push(s);
        let mut s = valid[i];
        s[0] |= 1;
        encs.push(s);
    }
    for k in 0u8..4 {
        let mut b = h32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f");
        b[0] = b[0].wrapping_add(k);
        encs.push(b);
    }

    let mut decoded: Vec<(String, P3)> = Vec::new();
    let mut seen = Seen::default();
    for (i, s) in encs.iter().enumerate() {
        let (mut cp, mut rp): (P3, P3) = ([CANARY; 40], [CANARY; 40]);
        let crc = unsafe { cfb(cp.as_mut_ptr(), s.as_ptr()) };
        let rrc = unsafe { rfb(rp.as_mut_ptr(), s.as_ptr()) };
        common::eqi(
            &format!("ristretto255_frombytes rc {i} s={}", common::hex(s)),
            seen.add(crc),
            rrc,
        );
        eqlimbs(
            &format!("ristretto255_frombytes p3 {i} s={}", common::hex(s)),
            &cp,
            &rp,
        );
        if crc == 0 && decoded.len() < 48 {
            decoded.push((format!("decoded[{i}]"), cp));
        }
    }
    seen.require("ristretto255_frombytes", &[0, -1]);

    // Classify WHY canonical encodings were rejected, so that the three OR'd
    // sub-conditions of the single `return -(...)` site are accounted for.
    {
        let ctb = getsym!(
            common::libs().c,
            "_sodium_fe25519_tobytes",
            unsafe extern "C" fn(*mut u8, *const i32)
        );
        let ric = getsym!(
            common::libs().c,
            "_sodium_ge25519_is_canonical",
            unsafe extern "C" fn(*const u8) -> c_int
        );
        let (mut n_ns, mut n_negt, mut n_y0) = (0usize, 0usize, 0usize);
        for s in encs.iter() {
            // only canonical ristretto encodings reach the arithmetic
            if s[0] & 1 != 0 || s[31] & 0x80 != 0 || unsafe { ric(s.as_ptr()) } == 0 {
                continue;
            }
            let (mut p, mut yb, mut tb): (P3, [u8; 32], [u8; 32]) =
                ([CANARY; 40], [0; 32], [0; 32]);
            if unsafe { cfb(p.as_mut_ptr(), s.as_ptr()) } == 0 {
                continue;
            }
            unsafe { ctb(yb.as_mut_ptr(), p[10..20].as_ptr()) };
            unsafe { ctb(tb.as_mut_ptr(), p[30..40].as_ptr()) };
            if tb[0] & 1 != 0 {
                n_negt += 1;
            }
            if yb.iter().all(|&x| x == 0) {
                n_y0 += 1;
            }
            if tb[0] & 1 == 0 && yb.iter().any(|&x| x != 0) {
                n_ns += 1;
            }
        }
        println!(
            "ristretto255_frombytes rejection reasons: non-square={n_ns} isnegative(T)={n_negt} iszero(Y)={n_y0}"
        );
        // All three OR'd sub-conditions of `return -((1-notsquare) |
        // isnegative(T) | iszero(Y))` must occur in the corpus.
        assert!(n_ns > 0, "the `not a square` sub-condition was never exercised");
        assert!(n_negt > 0, "the `isnegative(T)` sub-condition was never exercised");
        assert!(n_y0 > 0, "the `iszero(Y)` sub-condition was never exercised");
    }

    // ---- p3_tobytes on ristretto-decoded points and on the general p3 corpus
    for (name, p) in &decoded {
        let (mut cs, mut rs) = ([0x22u8; 32], [0x22u8; 32]);
        unsafe { ctb(cs.as_mut_ptr(), p.as_ptr()) };
        unsafe { rtb(rs.as_mut_ptr(), p.as_ptr()) };
        common::eqb(&format!("ristretto255_p3_tobytes {name}"), &cs, &rs);
    }
    for (name, p) in p3_corpus(&mut rng) {
        let (mut cs, mut rs) = ([0x22u8; 32], [0x22u8; 32]);
        unsafe { ctb(cs.as_mut_ptr(), p.as_ptr()) };
        unsafe { rtb(rs.as_mut_ptr(), p.as_ptr()) };
        common::eqb(&format!("ristretto255_p3_tobytes {name}"), &cs, &rs);
    }
}

// ============================================================================
// public crypto_core_ed25519_* API
// ============================================================================

#[test]
fn core_ed25519_bytes_getters() {
    for (name, want) in [
        ("crypto_core_ed25519_bytes", 32usize),
        ("crypto_core_ed25519_uniformbytes", 32),
        ("crypto_core_ed25519_hashbytes", 64),
        ("crypto_core_ed25519_scalarbytes", 32),
        ("crypto_core_ed25519_nonreducedscalarbytes", 64),
        ("crypto_core_ristretto255_bytes", 32),
        ("crypto_core_ristretto255_hashbytes", 64),
        ("crypto_core_ristretto255_scalarbytes", 32),
        ("crypto_core_ristretto255_nonreducedscalarbytes", 64),
    ] {
        let l = common::libs();
        let cf = unsafe {
            *l.c.get::<unsafe extern "C" fn() -> usize>(
                format!("{name}\0").as_bytes(),
            )
            .unwrap()
        };
        let rf = unsafe {
            *l.r.get::<unsafe extern "C" fn() -> usize>(
                format!("{name}\0").as_bytes(),
            )
            .unwrap()
        };
        let (c, r) = unsafe { (cf(), rf()) };
        assert_eq!(c, r, "{name}: C {c} vs Rust {r}");
        assert_eq!(c, want, "{name}: unexpected value {c}");
    }
}

#[test]
fn core_ed25519_point_api() {
    let (civp, rivp) = both!(
        "crypto_core_ed25519_is_valid_point",
        unsafe extern "C" fn(*const u8) -> c_int
    );
    let (cadd, radd) = both!(
        "crypto_core_ed25519_add",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int
    );
    let (csub, rsub) = both!(
        "crypto_core_ed25519_sub",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int
    );
    let (crnd, rrnd) = both!("crypto_core_ed25519_random", unsafe extern "C" fn(*mut u8));

    let mut rng = common::Rng::new(0xC0_2551_900C);
    let mut encs = special_encodings();
    // valid points
    let valid_start = encs.len();
    for _ in 0..48 {
        let mut a = [0u8; 32];
        rng.fill(&mut a);
        a[31] &= 0x1f;
        encs.push(c_p3_tobytes(&c_scalarmult_base(&a)));
    }
    let some_valid = encs[valid_start];
    // canonical, on-curve, non-small-order points OUTSIDE the prime-order
    // subgroup -> the only way to reach the `is_on_main_subgroup == 0` branch
    // of crypto_core_ed25519_is_valid_point.
    encs.extend(off_subgroup_encodings());
    // valid point encodings with the sign bit flipped (still valid)
    for i in 0..8 {
        let mut s = encs[encs.len() - 1 - i];
        s[31] ^= 0x80;
        encs.push(s);
    }
    // random garbage
    for _ in 0..200 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        encs.push(b);
    }

    let mut s_ivp = Seen::default();
    for (i, s) in encs.iter().enumerate() {
        common::eqi(
            &format!("core_ed25519_is_valid_point {i} s={}", common::hex(s)),
            s_ivp.add(unsafe { civp(s.as_ptr()) }),
            unsafe { rivp(s.as_ptr()) },
        );
    }
    s_ivp.require("crypto_core_ed25519_is_valid_point", &[0, 1]);

    // Pin each of the four reachable rejection sites of is_valid_point to a
    // concrete input, using the same low-level predicates the C uses.
    {
        let cic = getsym!(
            common::libs().c,
            "_sodium_ge25519_is_canonical",
            unsafe extern "C" fn(*const u8) -> c_int
        );
        let coc = getsym!(
            common::libs().c,
            "_sodium_ge25519_is_on_curve",
            unsafe extern "C" fn(*const i32) -> c_int
        );
        let cso = getsym!(
            common::libs().c,
            "_sodium_ge25519_has_small_order",
            unsafe extern "C" fn(*const i32) -> c_int
        );
        let cms = getsym!(
            common::libs().c,
            "_sodium_ge25519_is_on_main_subgroup",
            unsafe extern "C" fn(*const i32) -> c_int
        );
        // E15: non-canonical encoding
        let s = h32("edffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f");
        assert_eq!(unsafe { cic(s.as_ptr()) }, 0, "E15 setup");
        assert_eq!(unsafe { civp(s.as_ptr()) }, 0);
        assert_eq!(unsafe { rivp(s.as_ptr()) }, 0);
        // E16: canonical but frombytes fails (y not on the curve)
        let s = h32("0200000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(unsafe { cic(s.as_ptr()) }, 1, "E16 setup: canonical");
        assert_eq!(c_frombytes(&s).0, -1, "E16 setup: frombytes fails");
        assert_eq!(unsafe { civp(s.as_ptr()) }, 0);
        assert_eq!(unsafe { rivp(s.as_ptr()) }, 0);
        // E18: canonical, decodes, on curve, but small order.
        //   y=1  -> identity (order 1)
        //   y=0  -> order 4 (x = sqrt(-1))
        //   y=p-1-> order 2
        for t in [
            "0100000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000080",
            "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
        ] {
            let s = h32(t);
            let (rc, p) = c_frombytes(&s);
            assert_eq!((rc, unsafe { cic(s.as_ptr()) }), (0, 1), "E18 setup {t}");
            assert_eq!(unsafe { coc(p.as_ptr()) }, 1, "E18 setup {t}: on curve");
            assert_eq!(unsafe { cso(p.as_ptr()) }, 1, "E18 setup {t}: small order");
            assert_eq!(unsafe { civp(s.as_ptr()) }, 0);
            assert_eq!(unsafe { rivp(s.as_ptr()) }, 0);
        }
        // E19: canonical, decodes, on curve, NOT small order, but not in the
        // prime-order subgroup -> only reachable via a*B + T
        let off = off_subgroup_encodings();
        assert!(!off.is_empty(), "E19 setup: no off-subgroup points built");
        let mut hit = 0;
        for s in &off {
            let (rc, p) = c_frombytes(s);
            if rc != 0
                || unsafe { cic(s.as_ptr()) } != 1
                || unsafe { coc(p.as_ptr()) } != 1
                || unsafe { cso(p.as_ptr()) } != 0
                || unsafe { cms(p.as_ptr()) } != 0
            {
                continue;
            }
            hit += 1;
            assert_eq!(unsafe { civp(s.as_ptr()) }, 0, "E19: C");
            assert_eq!(unsafe { rivp(s.as_ptr()) }, 0, "E19: Rust");
        }
        assert!(
            hit > 0,
            "E19: no input reached the is_on_main_subgroup==0 rejection"
        );
    }

    // explicit valid x invalid combinations for add/sub (each of the four
    // `ge25519_frombytes(..) != 0 || is_on_curve(..) == 0` short-circuits)
    let bad = h32("0200000000000000000000000000000000000000000000000000000000000000");
    let mut s_add = Seen::default();
    for (tag, p, q) in [
        ("valid,valid", some_valid, some_valid),
        ("bad,valid", bad, some_valid),
        ("valid,bad", some_valid, bad),
        ("bad,bad", bad, bad),
    ] {
        for (op, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
            let (mut co, mut ro) = ([0x44u8; 32], [0x44u8; 32]);
            let crc = unsafe { cf(co.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
            let rrc = unsafe { rf(ro.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
            common::eqi(
                &format!("core_ed25519_{op} {tag} rc"),
                s_add.add(crc),
                rrc,
            );
            common::eqb(&format!("core_ed25519_{op} {tag} out"), &co, &ro);
        }
    }
    s_add.require("crypto_core_ed25519_add/sub", &[0, -1]);

    for i in 0..encs.len() {
        let p = &encs[i];
        let q = &encs[(i * 7 + 3) % encs.len()];
        for (op, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
            let (mut co, mut ro) = ([0x44u8; 32], [0x44u8; 32]);
            let crc = unsafe { cf(co.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
            let rrc = unsafe { rf(ro.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
            common::eqi(
                &format!(
                    "core_ed25519_{op} rc {i} p={} q={}",
                    common::hex(p),
                    common::hex(q)
                ),
                crc,
                rrc,
            );
            // NOTE: on failure the C leaves `r` untouched; comparing the full
            // canary-filled buffer catches spurious writes.
            common::eqb(
                &format!(
                    "core_ed25519_{op} out {i} p={} q={}",
                    common::hex(p),
                    common::hex(q)
                ),
                &co,
                &ro,
            );
        }
        // self-add / self-sub
        for (op, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
            let (mut co, mut ro) = ([0x44u8; 32], [0x44u8; 32]);
            let crc = unsafe { cf(co.as_mut_ptr(), p.as_ptr(), p.as_ptr()) };
            let rrc = unsafe { rf(ro.as_mut_ptr(), p.as_ptr(), p.as_ptr()) };
            common::eqi(&format!("core_ed25519_{op} self rc {i}"), crc, rrc);
            common::eqb(&format!("core_ed25519_{op} self out {i}"), &co, &ro);
        }
    }

    // crypto_core_ed25519_random(): RNG-driven, so only the validity of the
    // result is compared (not the bytes).
    for i in 0..16 {
        let (mut co, mut ro) = ([0u8; 32], [0u8; 32]);
        unsafe { crnd(co.as_mut_ptr()) };
        unsafe { rrnd(ro.as_mut_ptr()) };
        // Both must produce a point in the main subgroup.
        assert_eq!(
            unsafe { civp(co.as_ptr()) },
            1,
            "C random point {i} invalid: {}",
            common::hex(&co)
        );
        assert_eq!(
            unsafe { rivp(ro.as_ptr()) },
            1,
            "Rust random point {i} invalid: {}",
            common::hex(&ro)
        );
        // cross-check: each library validates the other's point identically
        common::eqi(
            &format!("core_ed25519_random cross-validate {i}"),
            unsafe { civp(ro.as_ptr()) },
            unsafe { rivp(co.as_ptr()) },
        );
    }
}

#[test]
fn core_ed25519_scalar_api() {
    let (cred, rred) = both!(
        "crypto_core_ed25519_scalar_reduce",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (cinv, rinv) = both!(
        "crypto_core_ed25519_scalar_invert",
        unsafe extern "C" fn(*mut u8, *const u8) -> c_int
    );
    let (cneg, rneg) = both!(
        "crypto_core_ed25519_scalar_negate",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (ccmp, rcmp) = both!(
        "crypto_core_ed25519_scalar_complement",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (cadd, radd) = both!(
        "crypto_core_ed25519_scalar_add",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8)
    );
    let (csub, rsub) = both!(
        "crypto_core_ed25519_scalar_sub",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8)
    );
    let (cmul, rmul) = both!(
        "crypto_core_ed25519_scalar_mul",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8)
    );
    let (cican, rican) = both!(
        "crypto_core_ed25519_scalar_is_canonical",
        unsafe extern "C" fn(*const u8) -> c_int
    );
    let (crnd, rrnd) = both!(
        "crypto_core_ed25519_scalar_random",
        unsafe extern "C" fn(*mut u8)
    );

    let mut rng = common::Rng::new(0xC0_2551_900D);
    let mut s32 = special_scalars();
    for _ in 0..200 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        s32.push(b);
    }
    let mut s64 = special_scalars64();
    for _ in 0..200 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        s64.push(b);
    }

    for (i, s) in s64.iter().enumerate() {
        let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
        unsafe { cred(co.as_mut_ptr(), s.as_ptr()) };
        unsafe { rred(ro.as_mut_ptr(), s.as_ptr()) };
        common::eqb(&format!("scalar_reduce {i}"), &co, &ro);
    }

    let (mut s_inv, mut s_can) = (Seen::default(), Seen::default());
    for (i, s) in s32.iter().enumerate() {
        let ctx = format!("{i} s={}", common::hex(s));
        let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
        let crc = unsafe { cinv(co.as_mut_ptr(), s.as_ptr()) };
        let rrc = unsafe { rinv(ro.as_mut_ptr(), s.as_ptr()) };
        common::eqi(&format!("scalar_invert rc {ctx}"), s_inv.add(crc), rrc);
        common::eqb(&format!("scalar_invert out {ctx}"), &co, &ro);

        for (op, cf, rf) in [("negate", cneg, rneg), ("complement", ccmp, rcmp)] {
            let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
            unsafe { cf(co.as_mut_ptr(), s.as_ptr()) };
            unsafe { rf(ro.as_mut_ptr(), s.as_ptr()) };
            common::eqb(&format!("scalar_{op} {ctx}"), &co, &ro);
        }

        common::eqi(
            &format!("scalar_is_canonical {ctx}"),
            s_can.add(unsafe { cican(s.as_ptr()) }),
            unsafe { rican(s.as_ptr()) },
        );

        let y = &s32[(i * 5 + 1) % s32.len()];
        for (op, cf, rf) in [
            ("add", cadd, radd),
            ("sub", csub, rsub),
            ("mul", cmul, rmul),
        ] {
            let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
            unsafe { cf(co.as_mut_ptr(), s.as_ptr(), y.as_ptr()) };
            unsafe { rf(ro.as_mut_ptr(), s.as_ptr(), y.as_ptr()) };
            common::eqb(
                &format!("scalar_{op} {ctx} y={}", common::hex(y)),
                &co,
                &ro,
            );
        }
    }

    s_inv.require("crypto_core_ed25519_scalar_invert", &[0, -1]);
    s_can.require("crypto_core_ed25519_scalar_is_canonical", &[0, 1]);

    // fully random pairs for add/sub/mul
    for i in 0..200 {
        let (mut x, mut y) = ([0u8; 32], [0u8; 32]);
        rng.fill(&mut x);
        rng.fill(&mut y);
        for (op, cf, rf) in [
            ("add", cadd, radd),
            ("sub", csub, rsub),
            ("mul", cmul, rmul),
        ] {
            let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
            unsafe { cf(co.as_mut_ptr(), x.as_ptr(), y.as_ptr()) };
            unsafe { rf(ro.as_mut_ptr(), x.as_ptr(), y.as_ptr()) };
            common::eqb(&format!("scalar_{op} rand {i}"), &co, &ro);
        }
    }

    // scalar_random(): RNG-driven -> only structural properties are compared.
    for i in 0..32 {
        let (mut co, mut ro) = ([0u8; 32], [0u8; 32]);
        unsafe { crnd(co.as_mut_ptr()) };
        unsafe { rrnd(ro.as_mut_ptr()) };
        for (who, b) in [("C", &co), ("Rust", &ro)] {
            assert_eq!(
                unsafe { cican(b.as_ptr()) },
                1,
                "{who} scalar_random {i} not canonical: {}",
                common::hex(b)
            );
            assert!(
                b.iter().any(|&x| x != 0),
                "{who} scalar_random {i} is zero"
            );
            assert_eq!(
                b[31] & 0xe0,
                0,
                "{who} scalar_random {i} top bits set: {}",
                common::hex(b)
            );
        }
    }
}

#[test]
fn core_ed25519_from_string() {
    let (cfsnu, rfsnu) = both!(
        "crypto_core_ed25519_from_string_nu",
        unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int
    );
    let (cfs, rfs) = both!(
        "crypto_core_ed25519_from_string",
        unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int
    );
    let (csfs, rsfs) = both!(
        "crypto_core_ed25519_scalar_from_string",
        unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int
    );
    let (crfs, rrfs) = both!(
        "crypto_core_ristretto255_from_string",
        unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int
    );
    let (crsfs, rrsfs) = both!(
        "crypto_core_ristretto255_scalar_from_string",
        unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int
    );

    let mut rng = common::Rng::new(0xC0_2551_900E);
    let big_ctx = rng.bytes(300); // > 0xff -> H2C-OVERSIZE-DST- path
    let ctxs: Vec<Vec<u8>> = vec![
        vec![],
        b"A".to_vec(),
        b"QUUX-V01-CS02-with-edwards25519_XMD:SHA-512_ELL2_RO_".to_vec(),
        rng.bytes(255),
        rng.bytes(256),
        big_ctx,
    ];
    let msgs: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"abc".to_vec(),
        rng.bytes(63),
        rng.bytes(64),
        rng.bytes(65),
        rng.bytes(1000),
    ];
    // 1 = SHA256, 2 = SHA512, everything else -> -1 (EINVAL)
    let algs: [c_int; 6] = [1, 2, 0, 3, -1, 0x7fff_ffff];

    let fns: [(&str, _, _); 5] = [
        ("ed25519_from_string_nu", cfsnu, rfsnu),
        ("ed25519_from_string", cfs, rfs),
        ("ed25519_scalar_from_string", csfs, rsfs),
        ("ristretto255_from_string", crfs, rrfs),
        ("ristretto255_scalar_from_string", crsfs, rrsfs),
    ];
    for (name, cf, rf) in fns {
        let mut seen = Seen::default();
        for (ci, ctx) in ctxs.iter().enumerate() {
            for (mi, msg) in msgs.iter().enumerate() {
                for alg in algs {
                    let (mut co, mut ro) = ([0x88u8; 32], [0x88u8; 32]);
                    let crc = unsafe {
                        cf(
                            co.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctx.len(),
                            msg.as_ptr(),
                            msg.len(),
                            alg,
                        )
                    };
                    let rrc = unsafe {
                        rf(
                            ro.as_mut_ptr(),
                            ctx.as_ptr(),
                            ctx.len(),
                            msg.as_ptr(),
                            msg.len(),
                            alg,
                        )
                    };
                    let tag = format!("{name} ctx={ci} msg={mi} alg={alg}");
                    common::eqi(&format!("{tag}: rc"), seen.add(crc), rrc);
                    common::eqb(&format!("{tag}: out"), &co, &ro);
                }
            }
        }
        // NULL ctx / msg with zero length (the C prototype only marks the
        // output pointer as nonnull).
        for alg in [1, 2, 7] {
            let (mut co, mut ro) = ([0x88u8; 32], [0x88u8; 32]);
            let crc = unsafe {
                cf(
                    co.as_mut_ptr(),
                    core::ptr::null(),
                    0,
                    core::ptr::null(),
                    0,
                    alg,
                )
            };
            let rrc = unsafe {
                rf(
                    ro.as_mut_ptr(),
                    core::ptr::null(),
                    0,
                    core::ptr::null(),
                    0,
                    alg,
                )
            };
            common::eqi(&format!("{name} NULL ctx/msg alg={alg}: rc"), crc, rrc);
            common::eqb(&format!("{name} NULL ctx/msg alg={alg}: out"), &co, &ro);
        }
        seen.require(name, &[0, -1]);
    }
}

// ============================================================================
// public crypto_core_ristretto255_* API
// ============================================================================

#[test]
fn core_ristretto255_api() {
    let (civp, rivp) = both!(
        "crypto_core_ristretto255_is_valid_point",
        unsafe extern "C" fn(*const u8) -> c_int
    );
    let (cadd, radd) = both!(
        "crypto_core_ristretto255_add",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int
    );
    let (csub, rsub) = both!(
        "crypto_core_ristretto255_sub",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int
    );
    let (cfh, rfh) = both!(
        "crypto_core_ristretto255_from_hash",
        unsafe extern "C" fn(*mut u8, *const u8) -> c_int
    );
    let (crnd, rrnd) = both!(
        "crypto_core_ristretto255_random",
        unsafe extern "C" fn(*mut u8)
    );
    let (csrnd, rsrnd) = both!(
        "crypto_core_ristretto255_scalar_random",
        unsafe extern "C" fn(*mut u8)
    );
    let (cinv, rinv) = both!(
        "crypto_core_ristretto255_scalar_invert",
        unsafe extern "C" fn(*mut u8, *const u8) -> c_int
    );
    let (cneg, rneg) = both!(
        "crypto_core_ristretto255_scalar_negate",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (ccmp, rcmp) = both!(
        "crypto_core_ristretto255_scalar_complement",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (csa, rsa) = both!(
        "crypto_core_ristretto255_scalar_add",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8)
    );
    let (css, rss) = both!(
        "crypto_core_ristretto255_scalar_sub",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8)
    );
    let (csm, rsm) = both!(
        "crypto_core_ristretto255_scalar_mul",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8)
    );
    let (csr, rsr) = both!(
        "crypto_core_ristretto255_scalar_reduce",
        unsafe extern "C" fn(*mut u8, *const u8)
    );
    let (csic, rsic) = both!(
        "crypto_core_ristretto255_scalar_is_canonical",
        unsafe extern "C" fn(*const u8) -> c_int
    );

    let mut rng = common::Rng::new(0x21_2551_900F);

    // ---- from_hash (always returns 0)
    let mut valid: Vec<[u8; 32]> = Vec::new();
    let mut hs: Vec<[u8; 64]> = vec![[0u8; 64], [0xffu8; 64]];
    for _ in 0..64 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        hs.push(b);
    }
    for (i, h) in hs.iter().enumerate() {
        let (mut co, mut ro) = ([0x22u8; 32], [0x22u8; 32]);
        let crc = unsafe { cfh(co.as_mut_ptr(), h.as_ptr()) };
        let rrc = unsafe { rfh(ro.as_mut_ptr(), h.as_ptr()) };
        common::eqi(&format!("ristretto255_from_hash rc {i}"), crc, rrc);
        common::eqb(&format!("ristretto255_from_hash out {i}"), &co, &ro);
        valid.push(co);
    }

    // ---- is_valid_point / add / sub
    let mut encs: Vec<[u8; 32]> = valid.clone();
    encs.push([0u8; 32]);
    encs.extend(special_encodings());
    for _ in 0..200 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        encs.push(b);
    }
    for i in 0..16 {
        let mut s = valid[i];
        s[31] |= 0x80;
        encs.push(s);
        let mut s = valid[i];
        s[0] ^= 1;
        encs.push(s);
    }
    let mut s_ivp = Seen::default();
    for (i, s) in encs.iter().enumerate() {
        common::eqi(
            &format!("ristretto255_is_valid_point {i} s={}", common::hex(s)),
            s_ivp.add(unsafe { civp(s.as_ptr()) }),
            unsafe { rivp(s.as_ptr()) },
        );
    }
    s_ivp.require("crypto_core_ristretto255_is_valid_point", &[0, 1]);
    let mut s_ar = Seen::default();
    for i in 0..encs.len() {
        let p = &encs[i];
        let q = &encs[(i * 7 + 3) % encs.len()];
        for (op, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
            let (mut co, mut ro) = ([0x44u8; 32], [0x44u8; 32]);
            let crc = unsafe { cf(co.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
            let rrc = unsafe { rf(ro.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
            common::eqi(
                &format!(
                    "ristretto255_{op} rc {i} p={} q={}",
                    common::hex(p),
                    common::hex(q)
                ),
                s_ar.add(crc),
                rrc,
            );
            common::eqb(&format!("ristretto255_{op} out {i}"), &co, &ro);
        }
        for (op, cf, rf) in [("add", cadd, radd), ("sub", csub, rsub)] {
            let (mut co, mut ro) = ([0x44u8; 32], [0x44u8; 32]);
            let crc = unsafe { cf(co.as_mut_ptr(), p.as_ptr(), p.as_ptr()) };
            let rrc = unsafe { rf(ro.as_mut_ptr(), p.as_ptr(), p.as_ptr()) };
            common::eqi(&format!("ristretto255_{op} self rc {i}"), crc, rrc);
            common::eqb(&format!("ristretto255_{op} self out {i}"), &co, &ro);
        }
    }

    s_ar.require("crypto_core_ristretto255_add/sub", &[0, -1]);

    // ---- scalar ops (thin wrappers around the ed25519 ones)
    let mut s32 = special_scalars();
    for _ in 0..120 {
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        s32.push(b);
    }
    let mut s64 = special_scalars64();
    for _ in 0..120 {
        let mut b = [0u8; 64];
        rng.fill(&mut b);
        s64.push(b);
    }
    for (i, s) in s64.iter().enumerate() {
        let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
        unsafe { csr(co.as_mut_ptr(), s.as_ptr()) };
        unsafe { rsr(ro.as_mut_ptr(), s.as_ptr()) };
        common::eqb(&format!("ristretto255_scalar_reduce {i}"), &co, &ro);
    }
    let (mut s_inv, mut s_can) = (Seen::default(), Seen::default());
    for (i, s) in s32.iter().enumerate() {
        let ctx = format!("{i} s={}", common::hex(s));
        let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
        let crc = unsafe { cinv(co.as_mut_ptr(), s.as_ptr()) };
        let rrc = unsafe { rinv(ro.as_mut_ptr(), s.as_ptr()) };
        common::eqi(
            &format!("ristretto255_scalar_invert rc {ctx}"),
            s_inv.add(crc),
            rrc,
        );
        common::eqb(&format!("ristretto255_scalar_invert out {ctx}"), &co, &ro);

        for (op, cf, rf) in [("negate", cneg, rneg), ("complement", ccmp, rcmp)] {
            let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
            unsafe { cf(co.as_mut_ptr(), s.as_ptr()) };
            unsafe { rf(ro.as_mut_ptr(), s.as_ptr()) };
            common::eqb(&format!("ristretto255_scalar_{op} {ctx}"), &co, &ro);
        }
        common::eqi(
            &format!("ristretto255_scalar_is_canonical {ctx}"),
            s_can.add(unsafe { csic(s.as_ptr()) }),
            unsafe { rsic(s.as_ptr()) },
        );
        let y = &s32[(i * 5 + 1) % s32.len()];
        for (op, cf, rf) in [("add", csa, rsa), ("sub", css, rss), ("mul", csm, rsm)] {
            let (mut co, mut ro) = ([0x66u8; 32], [0x66u8; 32]);
            unsafe { cf(co.as_mut_ptr(), s.as_ptr(), y.as_ptr()) };
            unsafe { rf(ro.as_mut_ptr(), s.as_ptr(), y.as_ptr()) };
            common::eqb(&format!("ristretto255_scalar_{op} {ctx}"), &co, &ro);
        }
    }

    s_inv.require("crypto_core_ristretto255_scalar_invert", &[0, -1]);
    s_can.require("crypto_core_ristretto255_scalar_is_canonical", &[0, 1]);

    // ---- RNG-driven entry points: compare only validity
    for i in 0..16 {
        let (mut co, mut ro) = ([0u8; 32], [0u8; 32]);
        unsafe { crnd(co.as_mut_ptr()) };
        unsafe { rrnd(ro.as_mut_ptr()) };
        assert_eq!(unsafe { civp(co.as_ptr()) }, 1, "C ristretto random {i}");
        assert_eq!(unsafe { rivp(ro.as_ptr()) }, 1, "Rust ristretto random {i}");
        common::eqi(
            &format!("ristretto255_random cross-validate {i}"),
            unsafe { civp(ro.as_ptr()) },
            unsafe { rivp(co.as_ptr()) },
        );
    }
    for i in 0..16 {
        let (mut co, mut ro) = ([0u8; 32], [0u8; 32]);
        unsafe { csrnd(co.as_mut_ptr()) };
        unsafe { rsrnd(ro.as_mut_ptr()) };
        for (who, b) in [("C", &co), ("Rust", &ro)] {
            assert_eq!(
                unsafe { csic(b.as_ptr()) },
                1,
                "{who} ristretto scalar_random {i} not canonical"
            );
            assert!(b.iter().any(|&x| x != 0));
        }
    }
}

// ============================================================================
// cross-checks that tie the layers together
// ============================================================================

#[test]
fn cross_layer_consistency() {
    // scalarmult_base(a) must agree with scalarmult(a, B) for scalars with the
    // documented precondition a[31] <= 127, and the public add/sub API must
    // agree with the low-level p3_add/p3_sub + p3_tobytes composition.
    let (csb, rsb) = both!(
        "_sodium_ge25519_scalarmult_base",
        unsafe extern "C" fn(*mut i32, *const u8)
    );
    let (csm, rsm) = both!(
        "_sodium_ge25519_scalarmult",
        unsafe extern "C" fn(*mut i32, *const u8, *const i32)
    );
    let (ctb, rtb) = both!(
        "_sodium_ge25519_p3_tobytes",
        unsafe extern "C" fn(*mut u8, *const i32)
    );
    let (cadd, radd) = both!(
        "crypto_core_ed25519_add",
        unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int
    );
    let (cladd, rladd) = both!(
        "_sodium_ge25519_p3_add",
        unsafe extern "C" fn(*mut i32, *const i32, *const i32)
    );
    let (cfb, rfb) = both!(
        "_sodium_ge25519_frombytes",
        unsafe extern "C" fn(*mut i32, *const u8) -> c_int
    );

    let B = c_scalarmult_base(&{
        let mut a = [0u8; 32];
        a[0] = 1;
        a
    });

    let mut rng = common::Rng::new(0xCC_2551_9010);
    for i in 0..32 {
        let mut a = [0u8; 32];
        rng.fill(&mut a);
        a[31] &= 0x7f;

        let (mut c1, mut r1): (P3, P3) = ([0; 40], [0; 40]);
        unsafe { csb(c1.as_mut_ptr(), a.as_ptr()) };
        unsafe { rsb(r1.as_mut_ptr(), a.as_ptr()) };
        let (mut c2, mut r2): (P3, P3) = ([0; 40], [0; 40]);
        unsafe { csm(c2.as_mut_ptr(), a.as_ptr(), B.as_ptr()) };
        unsafe { rsm(r2.as_mut_ptr(), a.as_ptr(), B.as_ptr()) };

        let (mut cs1, mut rs1, mut cs2, mut rs2) =
            ([0u8; 32], [0u8; 32], [0u8; 32], [0u8; 32]);
        unsafe { ctb(cs1.as_mut_ptr(), c1.as_ptr()) };
        unsafe { rtb(rs1.as_mut_ptr(), r1.as_ptr()) };
        unsafe { ctb(cs2.as_mut_ptr(), c2.as_ptr()) };
        unsafe { rtb(rs2.as_mut_ptr(), r2.as_ptr()) };
        common::eqb(&format!("cross {i}: base vs generic (C)"), &cs1, &cs2);
        common::eqb(&format!("cross {i}: base vs generic (Rust)"), &rs1, &rs2);
        common::eqb(&format!("cross {i}: C vs Rust base"), &cs1, &rs1);

        // public add == low-level add
        let mut b = [0u8; 32];
        rng.fill(&mut b);
        b[31] &= 0x1f;
        let pb = c_p3_tobytes(&c_scalarmult_base(&b));
        let (mut co, mut ro) = ([0u8; 32], [0u8; 32]);
        assert_eq!(
            unsafe { cadd(co.as_mut_ptr(), cs1.as_ptr(), pb.as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { radd(ro.as_mut_ptr(), rs1.as_ptr(), pb.as_ptr()) },
            0
        );
        common::eqb(&format!("cross {i}: public add"), &co, &ro);

        let (mut cp, mut rp): (P3, P3) = ([0; 40], [0; 40]);
        let (mut cq, mut rq): (P3, P3) = ([0; 40], [0; 40]);
        assert_eq!(unsafe { cfb(cp.as_mut_ptr(), cs1.as_ptr()) }, 0);
        assert_eq!(unsafe { rfb(rp.as_mut_ptr(), rs1.as_ptr()) }, 0);
        assert_eq!(unsafe { cfb(cq.as_mut_ptr(), pb.as_ptr()) }, 0);
        assert_eq!(unsafe { rfb(rq.as_mut_ptr(), pb.as_ptr()) }, 0);
        let (mut cr, mut rr): (P3, P3) = ([0; 40], [0; 40]);
        unsafe { cladd(cr.as_mut_ptr(), cp.as_ptr(), cq.as_ptr()) };
        unsafe { rladd(rr.as_mut_ptr(), rp.as_ptr(), rq.as_ptr()) };
        let (mut cs3, mut rs3) = ([0u8; 32], [0u8; 32]);
        unsafe { ctb(cs3.as_mut_ptr(), cr.as_ptr()) };
        unsafe { rtb(rs3.as_mut_ptr(), rr.as_ptr()) };
        common::eqb(&format!("cross {i}: low-level add == public add (C)"), &cs3, &co);
        common::eqb(&format!("cross {i}: low-level add == public add (Rust)"), &rs3, &ro);
    }
}
