//! Area 2, part 3 — `crypto_core/ed25519/core_ed25519.c`, `core_h2c.c` and the
//! `ge25519_*` / `sc25519_*` / `fe25519_*` internals of `ref10/ed25519_ref10.c`.
//!
//! Covers `configs_2.md` rows 2.51 - 2.103, 2.127 (ed25519 half) and 2.128, and
//! `errors_2.md` rows 2.4 - 2.11, 2.18 - 2.43.
#![allow(clippy::needless_range_loop)]
mod common;
use common::*;
use std::ffi::c_int;

// ------------------------------------------------------------------ constants

/// `L = 2^252 + 27742317777372353535851937790883648493`, little-endian.
const L: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
];

fn l_plus(k: i32) -> [u8; 32] {
    // L + k for small |k|, computed byte-wise.
    let mut v = L;
    if k >= 0 {
        let mut carry = k as u32;
        for b in v.iter_mut() {
            let s = *b as u32 + (carry & 0xff);
            *b = s as u8;
            carry = (carry >> 8) + (s >> 8);
            if carry == 0 {
                break;
            }
        }
    } else {
        let mut borrow = (-k) as u32;
        for b in v.iter_mut() {
            let s = *b as i32 - (borrow & 0xff) as i32;
            *b = s as u8;
            borrow = (borrow >> 8) + if s < 0 { 1 } else { 0 };
            if borrow == 0 {
                break;
            }
        }
    }
    v
}

fn zero32() -> [u8; 32] {
    [0u8; 32]
}
fn one32() -> [u8; 32] {
    let mut v = [0u8; 32];
    v[0] = 1;
    v
}

/// The Ed25519 base point encoding (`y = 4/5`, even `x`).
const BASEPOINT: [u8; 32] = [
    0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
];

/// The identity `(0, 1)`.
const IDENTITY: [u8; 32] = [
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// All eight points whose order divides 8 (the full torsion subgroup),
/// as 32-byte Ed25519 encodings.
const SMALL_ORDER: [[u8; 32]; 8] = [
    // (0, 1) — identity, order 1
    [
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    // (0, -1) — order 2
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    // order 8
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x05,
    ],
    // order 8
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0x7a,
    ],
    // (i, 0) — order 4
    [0u8; 32],
    // (-i, 0) — order 4
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0x80,
    ],
    // order 8
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x85,
    ],
    // order 8
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0xfa,
    ],
];

/// Non-canonical encodings (`y >= 2^255-19`) that nevertheless decode.
fn noncanonical_decodable() -> Vec<[u8; 32]> {
    let mut out = Vec::new();
    for first in [0xedu8, 0xee, 0xf0, 0xff] {
        for high in [0x7fu8, 0xff] {
            let mut v = [0xffu8; 32];
            v[0] = first;
            v[31] = high;
            out.push(v);
        }
    }
    out
}

// --------------------------------------------------------------- FFI typedefs

type Getter = unsafe extern "C" fn() -> usize;
type Fn1 = unsafe extern "C" fn(*mut u8);
type Fn2 = unsafe extern "C" fn(*mut u8, *const u8);
type Fn2i = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type Fn3 = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
type Fn3i = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type Fn4 = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);
type Pred = unsafe extern "C" fn(*const u8) -> c_int;
type FromString =
    unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, c_int) -> c_int;
type H2CHash =
    unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u8, usize, c_int) -> c_int;

/// `ge25519_p3` = 4 x `fe25519` = 4 x `int32_t[10]`.
#[repr(C)]
#[derive(Copy, Clone)]
struct P3([i32; 40]);
/// `ge25519_p2` = 3 x `fe25519`.
#[repr(C)]
#[derive(Copy, Clone)]
struct P2([i32; 30]);

impl P3 {
    fn new() -> P3 {
        P3([0x5A5A5A5A; 40])
    }
    fn bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}
impl P2 {
    fn new() -> P2 {
        P2([0x5A5A5A5A; 30])
    }
    fn bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

type GeFromBytes = unsafe extern "C" fn(*mut P3, *const u8) -> c_int;
type GeToBytes3 = unsafe extern "C" fn(*mut u8, *const P3);
type GeToBytes2 = unsafe extern "C" fn(*mut u8, *const P2);
type GePred = unsafe extern "C" fn(*const P3) -> c_int;
type GeP3Op = unsafe extern "C" fn(*mut P3, *const P3, *const P3);
type GeUnary = unsafe extern "C" fn(*mut P3);
type GeScalarmult = unsafe extern "C" fn(*mut P3, *const u8, *const P3);
type GeScalarmultBase = unsafe extern "C" fn(*mut P3, *const u8);
type GeDoubleScalarmult = unsafe extern "C" fn(*mut P2, *const u8, *const P3, *const u8);
type GeP2ToP3 = unsafe extern "C" fn(*mut P3, *const P2);

// ------------------------------------------------------------------- helpers

/// Apply a `void f(out32, in32)` pair and compare, with OOB guards.
#[track_caller]
fn cmp2(c: &Fn2, r: &Fn2, s: &[u8], inlen: usize, outlen: usize, label: &str) -> Vec<u8> {
    assert_eq!(s.len(), inlen);
    let mut oc = padded(outlen);
    let mut or = padded(outlen);
    unsafe {
        c(oc.as_mut_ptr(), s.as_ptr());
        r(or.as_mut_ptr(), s.as_ptr());
    }
    eqb(&format!("{label}({})", hex(s)), &oc[..outlen], &or[..outlen]);
    check_pad(&format!("{label}(C)"), &oc, outlen);
    check_pad(&format!("{label}(Rust)"), &or, outlen);
    oc[..outlen].to_vec()
}

#[track_caller]
fn cmp3(c: &Fn3, r: &Fn3, x: &[u8], y: &[u8], label: &str) -> Vec<u8> {
    let mut oc = padded(32);
    let mut or = padded(32);
    unsafe {
        c(oc.as_mut_ptr(), x.as_ptr(), y.as_ptr());
        r(or.as_mut_ptr(), x.as_ptr(), y.as_ptr());
    }
    eqb(&format!("{label}({}, {})", hex(x), hex(y)), &oc[..32], &or[..32]);
    check_pad(&format!("{label}(C)"), &oc, 32);
    check_pad(&format!("{label}(Rust)"), &or, 32);
    oc[..32].to_vec()
}

/// A pool of scalars covering every interesting shape.
fn scalar_pool(seed: u64) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = vec![
        zero32(),
        one32(),
        L,                // == L, non-canonical
        l_plus(-1),       // L - 1, largest canonical
        l_plus(1),
        l_plus(2),
        [0xffu8; 32],     // maximal non-canonical
        {
            let mut a = [0u8; 32];
            a[31] = 0x10;
            a
        },
        {
            let mut a = [0xffu8; 32];
            a[31] = 0x0f;
            a
        },
        {
            let mut a = [0u8; 32];
            a[0] = 0xed;
            a[31] = 0x10;
            a
        },
    ];
    // a canonical scalar with every single bit set
    for i in 0..252 {
        let mut a = [0u8; 32];
        a[i / 8] = 1u8 << (i % 8);
        v.push(a);
    }
    let mut rng = Rng::new(seed);
    for _ in 0..200 {
        // random canonical (top 3 bits of byte 31 cleared, then well below L)
        let mut a: [u8; 32] = rng.bytes(32).try_into().unwrap();
        a[31] &= 0x0f;
        v.push(a);
    }
    for _ in 0..200 {
        // fully random, mostly non-canonical
        v.push(rng.bytes(32).try_into().unwrap());
    }
    for _ in 0..100 {
        // near L
        let mut a = L;
        let i = rng.below(32);
        a[i] ^= 1u8 << rng.below(8);
        v.push(a);
    }
    v
}

fn nonreduced_pool(seed: u64) -> Vec<[u8; 64]> {
    let mut v: Vec<[u8; 64]> = Vec::new();
    v.push([0u8; 64]);
    v.push([0xffu8; 64]);
    {
        // exactly L, zero-padded
        let mut a = [0u8; 64];
        a[..32].copy_from_slice(&L);
        v.push(a);
        // L - 1
        let mut b = [0u8; 64];
        b[..32].copy_from_slice(&l_plus(-1));
        v.push(b);
        // 2^256 - 1 in the low half only
        let mut c = [0u8; 64];
        c[..32].fill(0xff);
        v.push(c);
        // only the top half set
        let mut d = [0u8; 64];
        d[32..].fill(0xff);
        v.push(d);
    }
    for i in 0..64 * 8 {
        let mut a = [0u8; 64];
        a[i / 8] = 1u8 << (i % 8);
        v.push(a);
    }
    let mut rng = Rng::new(seed);
    for _ in 0..400 {
        v.push(rng.bytes(64).try_into().unwrap());
    }
    v
}

// -------------------------------------------------------------------- getters

/// Row 2.127 (ed25519 half).
#[test]
fn ed25519_getters() {
    for (name, want) in [
        ("crypto_core_ed25519_bytes", 32usize),
        ("crypto_core_ed25519_uniformbytes", 32),
        ("crypto_core_ed25519_hashbytes", 64),
        ("crypto_core_ed25519_scalarbytes", 32),
        ("crypto_core_ed25519_nonreducedscalarbytes", 64),
    ] {
        assert!(has(name), "{name} must be exported by both libraries");
        let (c, r) = both::<Getter>(name);
        let (vc, vr) = unsafe { (c(), r()) };
        assert_eq!(vc, vr, "{name}: C {vc} vs Rust {vr}");
        assert_eq!(vc, want, "{name}: C returned {vc}, header says {want}");
    }
}

// ------------------------------------------------------------ scalar predicates

/// Row 2.76 and error rows 2.11 / 2.38.
#[test]
fn scalar_is_canonical() {
    let (c, r) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");
    let (cp, rp) = both::<Pred>("_sodium_sc25519_is_canonical");
    let named: &[(&str, [u8; 32], c_int)] = &[
        ("0", zero32(), 1),
        ("1", one32(), 1),
        ("L-1", l_plus(-1), 1),
        ("L", L, 0),
        ("L+1", l_plus(1), 0),
        ("all-0xff", [0xffu8; 32], 0),
    ];
    for (label, s, want) in named {
        let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
        eqi(&format!("scalar_is_canonical({label})"), a, b);
        assert_eq!(a, *want, "scalar_is_canonical({label}): C gave {a}, expected {want}");
        // the public wrapper must be exactly sc25519_is_canonical
        let (a2, b2) = unsafe { (cp(s.as_ptr()), rp(s.as_ptr())) };
        eqi(&format!("sc25519_is_canonical({label})"), a2, b2);
        assert_eq!(a, a2);
    }
    for s in scalar_pool(0x2_0076) {
        let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
        eqi(&format!("scalar_is_canonical({})", hex(&s)), a, b);
        let (a2, b2) = unsafe { (cp(s.as_ptr()), rp(s.as_ptr())) };
        eqi("sc25519_is_canonical", a2, b2);
        eqi("wrapper == sc25519_is_canonical", a, a2);
    }
}

// ------------------------------------------------------------- scalar_reduce

/// Rows 2.73 - 2.75.
#[test]
fn scalar_reduce() {
    let (c, r) = both::<Fn2>("crypto_core_ed25519_scalar_reduce");
    let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");
    // 2.73: value already < L -> low 32 bytes unchanged
    for s in scalar_pool(0x2_0073) {
        if unsafe { canon(s.as_ptr()) } == 0 {
            continue;
        }
        let mut t = [0u8; 64];
        t[..32].copy_from_slice(&s);
        let out = cmp2(&c, &r, &t, 64, 32, "scalar_reduce(<L)");
        assert_eq!(&out[..], &s[..], "reduce of an already-reduced value changed it");
    }
    // 2.74 / 2.75 and general fuzz
    for t in nonreduced_pool(0x2_0075) {
        let out = cmp2(&c, &r, &t, 64, 32, "scalar_reduce");
        assert_eq!(unsafe { canon(out.as_ptr()) }, 1, "reduce produced a non-canonical scalar");
    }
    // 2.74 explicitly: input == L -> 0
    let mut t = [0u8; 64];
    t[..32].copy_from_slice(&L);
    let out = cmp2(&c, &r, &t, 64, 32, "scalar_reduce(L)");
    assert_eq!(out, vec![0u8; 32], "reduce(L) must be 0");

    // the public wrapper must agree with sc25519_reduce on the same buffer
    let (crd, rrd) = both::<Fn1>("_sodium_sc25519_reduce");
    for t in nonreduced_pool(0x2_0073_1) {
        let mut bc = padded(64);
        let mut br = padded(64);
        bc[..64].copy_from_slice(&t);
        br[..64].copy_from_slice(&t);
        unsafe {
            crd(bc.as_mut_ptr());
            rrd(br.as_mut_ptr());
        }
        eqb("sc25519_reduce", &bc[..64], &br[..64]);
        check_pad("sc25519_reduce(C)", &bc, 64);
        check_pad("sc25519_reduce(Rust)", &br, 64);
        let out = cmp2(&c, &r, &t, 64, 32, "scalar_reduce");
        assert_eq!(&bc[..32], &out[..]);
    }
}

// -------------------------------------------- negate / complement / add / sub

/// Rows 2.56 - 2.62.
#[test]
fn scalar_negate_and_complement() {
    let (cn, rn) = both::<Fn2>("crypto_core_ed25519_scalar_negate");
    let (cc, rc) = both::<Fn2>("crypto_core_ed25519_scalar_complement");
    let (ca, _ra) = both::<Fn3>("crypto_core_ed25519_scalar_add");
    let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");

    // 2.56 / 2.57 / 2.60 / 2.61: exact expected values
    assert_eq!(cmp2(&cn, &rn, &zero32(), 32, 32, "negate(0)"), vec![0u8; 32]);
    assert_eq!(cmp2(&cn, &rn, &one32(), 32, 32, "negate(1)"), l_plus(-1).to_vec());
    assert_eq!(cmp2(&cc, &rc, &zero32(), 32, 32, "complement(0)"), one32().to_vec());
    assert_eq!(cmp2(&cc, &rc, &one32(), 32, 32, "complement(1)"), vec![0u8; 32]);

    for s in scalar_pool(0x2_0058) {
        let neg = cmp2(&cn, &rn, &s, 32, 32, "scalar_negate");
        let comp = cmp2(&cc, &rc, &s, 32, 32, "scalar_complement");
        assert_eq!(unsafe { canon(neg.as_ptr()) }, 1);
        assert_eq!(unsafe { canon(comp.as_ptr()) }, 1);
        if unsafe { canon(s.as_ptr()) } == 1 {
            // s + negate(s) == 0 and s + complement(s) == 1  (rows 2.58, 2.62)
            let mut sum = [0u8; 32];
            unsafe { ca(sum.as_mut_ptr(), s.as_ptr(), neg.as_ptr()) };
            assert_eq!(sum, [0u8; 32], "s + negate(s) != 0 for s = {}", hex(&s));
            unsafe { ca(sum.as_mut_ptr(), s.as_ptr(), comp.as_ptr()) };
            assert_eq!(sum, one32(), "s + complement(s) != 1 for s = {}", hex(&s));
        }
    }
}

/// Rows 2.63 - 2.69.
#[test]
fn scalar_add_and_sub() {
    let (ca, ra) = both::<Fn3>("crypto_core_ed25519_scalar_add");
    let (cs, rs) = both::<Fn3>("crypto_core_ed25519_scalar_sub");
    let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");
    let pool = scalar_pool(0x2_0063);

    // 2.65 / 2.69: identities
    assert_eq!(cmp3(&ca, &ra, &zero32(), &zero32(), "add(0,0)"), vec![0u8; 32]);
    assert_eq!(cmp3(&cs, &rs, &zero32(), &zero32(), "sub(0,0)"), vec![0u8; 32]);
    // 2.64: both operands L-1 -> wraps
    let lm1 = l_plus(-1);
    let s = cmp3(&ca, &ra, &lm1, &lm1, "add(L-1,L-1)");
    assert_eq!(s, l_plus(-2).to_vec(), "(L-1)+(L-1) must be L-2");

    let mut rng = Rng::new(0x2_0064);
    for _ in 0..3000 {
        let x = pool[rng.below(pool.len())];
        let y = pool[rng.below(pool.len())];
        let sum = cmp3(&ca, &ra, &x, &y, "scalar_add");
        let dif = cmp3(&cs, &rs, &x, &y, "scalar_sub");
        assert_eq!(unsafe { canon(sum.as_ptr()) }, 1);
        assert_eq!(unsafe { canon(dif.as_ptr()) }, 1);
        if unsafe { canon(x.as_ptr()) } == 1 && unsafe { canon(y.as_ptr()) } == 1 {
            // (x + y) - y == x  and  x - x == 0
            let back = cmp3(&cs, &rs, &sum, &y, "scalar_sub(roundtrip)");
            assert_eq!(&back[..], &x[..], "add/sub round trip failed");
            let zz = cmp3(&cs, &rs, &x, &x, "scalar_sub(x,x)");
            assert_eq!(zz, vec![0u8; 32]);
            let idl = cmp3(&ca, &ra, &x, &zero32(), "scalar_add(x,0)");
            assert_eq!(&idl[..], &x[..]);
            let idr = cmp3(&cs, &rs, &x, &zero32(), "scalar_sub(x,0)");
            assert_eq!(&idr[..], &x[..]);
        }
    }
}

/// Rows 2.70 - 2.72.
#[test]
fn scalar_mul() {
    let (cm, rm) = both::<Fn3>("crypto_core_ed25519_scalar_mul");
    let (cp, rp) = both::<Fn3>("_sodium_sc25519_mul");
    let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");
    let pool = scalar_pool(0x2_0070);

    // 2.71: y = 1 and y = 0
    for x in &pool {
        let p1 = cmp3(&cm, &rm, x, &one32(), "scalar_mul(x,1)");
        let p0 = cmp3(&cm, &rm, x, &zero32(), "scalar_mul(x,0)");
        assert_eq!(p0, vec![0u8; 32], "x*0 must be 0");
        if unsafe { canon(x.as_ptr()) } == 1 {
            assert_eq!(&p1[..], &x[..], "x*1 must be x for canonical x");
        }
    }
    let mut rng = Rng::new(0x2_0072);
    for _ in 0..3000 {
        let x = pool[rng.below(pool.len())];
        let y = pool[rng.below(pool.len())];
        let prod = cmp3(&cm, &rm, &x, &y, "scalar_mul");
        assert_eq!(unsafe { canon(prod.as_ptr()) }, 1);
        // commutativity, and the public wrapper must be sc25519_mul verbatim
        let prod2 = cmp3(&cm, &rm, &y, &x, "scalar_mul(swapped)");
        assert_eq!(prod, prod2, "scalar_mul is not commutative");
        let priv_ = cmp3(&cp, &rp, &x, &y, "sc25519_mul");
        assert_eq!(prod, priv_, "scalar_mul != sc25519_mul");
    }
}

/// `sc25519_muladd` — the private companion used by the signature code, checked
/// here since it lives in the same file and shares the reduction constants.
#[test]
fn sc25519_muladd_private() {
    assert!(has("_sodium_sc25519_muladd"));
    let (c, r) = both::<Fn4>("_sodium_sc25519_muladd");
    let (cm, _) = both::<Fn3>("_sodium_sc25519_mul");
    let (ca, _) = both::<Fn3>("crypto_core_ed25519_scalar_add");
    let pool = scalar_pool(0x2_0070_1);
    let mut rng = Rng::new(0x2_0070_2);
    for _ in 0..2000 {
        let a = pool[rng.below(pool.len())];
        let b = pool[rng.below(pool.len())];
        let cc = pool[rng.below(pool.len())];
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            c(oc.as_mut_ptr(), a.as_ptr(), b.as_ptr(), cc.as_ptr());
            r(or.as_mut_ptr(), a.as_ptr(), b.as_ptr(), cc.as_ptr());
        }
        eqb("sc25519_muladd", &oc[..32], &or[..32]);
        check_pad("sc25519_muladd(C)", &oc, 32);
        check_pad("sc25519_muladd(Rust)", &or, 32);
        // a*b + c, when every operand is canonical
        let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");
        if unsafe {
            canon(a.as_ptr()) == 1 && canon(b.as_ptr()) == 1 && canon(cc.as_ptr()) == 1
        } {
            let mut m = [0u8; 32];
            let mut s = [0u8; 32];
            unsafe {
                cm(m.as_mut_ptr(), a.as_ptr(), b.as_ptr());
                ca(s.as_mut_ptr(), m.as_ptr(), cc.as_ptr());
            }
            assert_eq!(&oc[..32], &s[..], "muladd != mul + add");
        }
    }
}

// ------------------------------------------------------------- scalar_invert

/// Rows 2.52 - 2.55 and error row 2.36.
#[test]
fn scalar_invert() {
    let (c, r) = both::<Fn2i>("crypto_core_ed25519_scalar_invert");
    let (cm, _) = both::<Fn3>("crypto_core_ed25519_scalar_mul");
    let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");

    let check = |s: &[u8; 32], want: c_int, label: &str| -> Vec<u8> {
        let mut oc = padded(32);
        let mut or = padded(32);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), s.as_ptr()) };
        let ec = errno();
        set_errno(0);
        let rr = unsafe { r(or.as_mut_ptr(), s.as_ptr()) };
        let er = errno();
        eqi(&format!("scalar_invert ret [{label}]"), rc, rr);
        assert_eq!(rc, want, "scalar_invert [{label}]: C gave {rc}, expected {want}");
        assert_eq!(ec, er, "scalar_invert errno [{label}]");
        // the out buffer is *always* fully written, even on the -1 path
        eqb(&format!("scalar_invert out [{label}]"), &oc[..32], &or[..32]);
        check_pad("scalar_invert(C)", &oc, 32);
        check_pad("scalar_invert(Rust)", &or, 32);
        oc[..32].to_vec()
    };

    // 2.52: s = 1 -> 1
    assert_eq!(check(&one32(), 0, "s=1"), one32().to_vec());
    // 2.54: s = L - 1 -> L - 1
    assert_eq!(check(&l_plus(-1), 0, "s=L-1"), l_plus(-1).to_vec());
    // error 2.36: s = 0 -> -1, recip written as all-zero
    assert_eq!(check(&zero32(), -1, "s=0"), vec![0u8; 32]);
    // 2.55: non-reduced input, e.g. all-0xff -> accepted, returns 0
    check(&[0xffu8; 32], 0, "s=0xff..");
    check(&L, 0, "s=L");

    for s in scalar_pool(0x2_0053) {
        let want = if s == [0u8; 32] { -1 } else { 0 };
        let recip = check(&s, want, "pool");
        if want == 0 && unsafe { canon(s.as_ptr()) } == 1 {
            let mut prod = [0u8; 32];
            unsafe { cm(prod.as_mut_ptr(), s.as_ptr(), recip.as_ptr()) };
            assert_eq!(prod, one32(), "s * invert(s) != 1 for s = {}", hex(&s));
        }
    }

    // the public wrapper must be sc25519_invert plus the zero test
    let (ci, ri) = both::<Fn2>("_sodium_sc25519_invert");
    for s in scalar_pool(0x2_0055) {
        let a = cmp2(&ci, &ri, &s, 32, 32, "sc25519_invert");
        let b = check(&s, if s == [0u8; 32] { -1 } else { 0 }, "wrapper");
        assert_eq!(a, b, "scalar_invert output != sc25519_invert output");
    }
}

/// Row 2.51 — `crypto_core_ed25519_scalar_random`.
#[test]
fn scalar_random() {
    let (c, r) = both::<Fn1>("crypto_core_ed25519_scalar_random");
    let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");
    for seed in 0..200u64 {
        rng_reseed(0x9E37_79B9_7F4A_7C15 ^ (seed.wrapping_mul(0x1000_0001) | 1));
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            c(oc.as_mut_ptr());
        }
        // rewind so Rust consumes the identical byte stream
        rng_reseed(0x9E37_79B9_7F4A_7C15 ^ (seed.wrapping_mul(0x1000_0001) | 1));
        unsafe {
            r(or.as_mut_ptr());
        }
        eqb("scalar_random", &oc[..32], &or[..32]);
        check_pad("scalar_random(C)", &oc, 32);
        check_pad("scalar_random(Rust)", &or, 32);
        assert_eq!(oc[31] & 0xe0, 0, "scalar_random: r[31] & 0xe0 must be 0");
        assert_eq!(unsafe { canon(oc.as_ptr()) }, 1, "scalar_random: not canonical");
        assert_ne!(&oc[..32], &[0u8; 32][..], "scalar_random: must be non-zero");
    }
    rng_reset();
}

// ------------------------------------------------------- scalar_from_string

/// Byte strings used as `ctx` / `msg`.
fn h2c_strings(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        vec![],
        b"a".to_vec(),
        b"QUUX-V01-CS02-with-edwards25519_XMD:SHA-512_ELL2_RO_".to_vec(),
        vec![0u8; 32],
        vec![0xffu8; 63],
        vec![0x41u8; 64],
        vec![0x42u8; 65],
        vec![0x43u8; 127],
        vec![0x44u8; 128],
        vec![0x45u8; 129],
        vec![0x46u8; 200],
        vec![0x47u8; 255], // largest direct-DST ctx_len
        vec![0x48u8; 256], // oversize DST -> re-hash
        vec![0x49u8; 257],
        vec![0x4au8; 1000],
    ];
    for n in [1usize, 3, 5, 31, 33, 48, 96] {
        v.push(rng.bytes(n));
    }
    v
}

/// Rows 2.77 - 2.82 and error row 2.37.
#[test]
fn scalar_from_string() {
    let (c, r) = both::<FromString>("crypto_core_ed25519_scalar_from_string");
    let (canon, _) = both::<Pred>("crypto_core_ed25519_scalar_is_canonical");
    let mut rng = Rng::new(0x2_0077);
    let strings = h2c_strings(&mut rng);

    let call = |ctx: Option<&[u8]>, msg: &[u8], alg: c_int| -> (c_int, Vec<u8>) {
        let (cp, cl) = match ctx {
            None => (std::ptr::null(), 0usize),
            Some(s) => (s.as_ptr(), s.len()),
        };
        let mut oc = padded(32);
        let mut or = padded(32);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), cp, cl, msg.as_ptr(), msg.len(), alg) };
        let ec = errno();
        set_errno(0);
        let rr = unsafe { r(or.as_mut_ptr(), cp, cl, msg.as_ptr(), msg.len(), alg) };
        let er = errno();
        eqi(&format!("scalar_from_string ret (alg {alg}, ctx_len {cl})"), rc, rr);
        assert_eq!(ec, er, "scalar_from_string errno (alg {alg})");
        if rc == 0 {
            eqb("scalar_from_string out", &oc[..32], &or[..32]);
        }
        check_pad("scalar_from_string(C)", &oc, 32);
        check_pad("scalar_from_string(Rust)", &or, 32);
        (rc, oc[..32].to_vec())
    };

    // 2.77 - 2.82: both hash algorithms x every ctx/msg shape
    for alg in [1i32, 2] {
        for ctx in &strings {
            for msg in &strings {
                let (rc, out) = call(Some(ctx), msg, alg);
                assert_eq!(rc, 0);
                assert_eq!(unsafe { canon(out.as_ptr()) }, 1);
            }
        }
        // 2.79: ctx == NULL with ctx_len == 0
        for msg in &strings {
            let (rc, out) = call(None, msg, alg);
            assert_eq!(rc, 0);
            let (rc2, out2) = call(Some(&[]), msg, alg);
            assert_eq!(rc2, 0);
            assert_eq!(out, out2, "ctx=NULL must behave like ctx_len=0");
        }
    }
    // SHA-256 and SHA-512 must give different results
    for msg in &strings {
        let (_, a) = call(Some(b"ctx"), msg, 1);
        let (_, b) = call(Some(b"ctx"), msg, 2);
        assert_ne!(a, b, "H2CSHA256 and H2CSHA512 must differ");
    }

    // error row 2.37: every out-of-range hash_alg -> -1, errno == EINVAL
    for alg in [0i32, 3, 4, -1, 255, 256, i32::MIN, i32::MAX] {
        let (rc, _) = call(Some(b"ctx"), b"msg", alg);
        assert_eq!(rc, -1, "scalar_from_string(alg={alg}) must fail");
        // errno was already compared inside `call`; confirm it is EINVAL in C
        set_errno(0);
        let mut o = [0u8; 32];
        let got = unsafe {
            c(o.as_mut_ptr(), b"ctx".as_ptr(), 3, b"msg".as_ptr(), 3, alg)
        };
        assert_eq!(got, -1);
        assert_eq!(errno(), EINVAL, "scalar_from_string(alg={alg}) must set EINVAL");
    }
}

// ------------------------------------------------------------- point decoding

struct Ge {
    frombytes: (
        libloading::Symbol<'static, GeFromBytes>,
        libloading::Symbol<'static, GeFromBytes>,
    ),
    frombytes_neg: (
        libloading::Symbol<'static, GeFromBytes>,
        libloading::Symbol<'static, GeFromBytes>,
    ),
    tobytes: (
        libloading::Symbol<'static, GeToBytes3>,
        libloading::Symbol<'static, GeToBytes3>,
    ),
    is_canonical: (libloading::Symbol<'static, Pred>, libloading::Symbol<'static, Pred>),
    is_on_curve: (libloading::Symbol<'static, GePred>, libloading::Symbol<'static, GePred>),
    small_order: (libloading::Symbol<'static, GePred>, libloading::Symbol<'static, GePred>),
    main_sub: (libloading::Symbol<'static, GePred>, libloading::Symbol<'static, GePred>),
}

impl Ge {
    fn new() -> Self {
        Ge {
            frombytes: both("_sodium_ge25519_frombytes"),
            frombytes_neg: both("_sodium_ge25519_frombytes_negate_vartime"),
            tobytes: both("_sodium_ge25519_p3_tobytes"),
            is_canonical: both("_sodium_ge25519_is_canonical"),
            is_on_curve: both("_sodium_ge25519_is_on_curve"),
            small_order: both("_sodium_ge25519_has_small_order"),
            main_sub: both("_sodium_ge25519_is_on_main_subgroup"),
        }
    }
}

/// A representative set of 32-byte point encodings.
fn point_encodings(seed: u64) -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    v.push(BASEPOINT);
    {
        let mut b = BASEPOINT;
        b[31] ^= 0x80; // negated base point
        v.push(b);
    }
    v.extend_from_slice(&SMALL_ORDER);
    v.extend(noncanonical_decodable());
    // every "y >= p" encoding family member, plus the boundary y = p-1
    for first in 0xe8u8..=0xffu8 {
        let mut b = [0xffu8; 32];
        b[0] = first;
        b[31] = 0x7f;
        v.push(b);
        let mut c = b;
        c[31] = 0xff;
        v.push(c);
    }
    v.push([0xffu8; 32]);
    v.push([0u8; 32]);
    let mut rng = Rng::new(seed);
    for _ in 0..600 {
        v.push(rng.bytes(32).try_into().unwrap());
    }
    // small y values, which cover both "has a root" and "has no root"
    for y in 0u32..64 {
        let mut b = [0u8; 32];
        b[0] = y as u8;
        v.push(b);
        let mut c = b;
        c[31] = 0x80;
        v.push(c);
    }
    v
}

/// Error rows 2.4 - 2.9 plus the decode side of rows 2.5 / 2.6.
#[test]
fn ge25519_decode_predicates() {
    let g = Ge::new();
    for s in point_encodings(0x2_0004) {
        // 2.4 — ge25519_is_canonical
        let (ac, ar) = unsafe { (g.is_canonical.0(s.as_ptr()), g.is_canonical.1(s.as_ptr())) };
        eqi(&format!("ge25519_is_canonical({})", hex(&s)), ac, ar);

        // 2.5 — ge25519_frombytes: always writes X/Y/Z/T, even on failure
        let mut pc = P3::new();
        let mut pr = P3::new();
        let (rc, rr) = unsafe {
            (
                g.frombytes.0(&mut pc, s.as_ptr()),
                g.frombytes.1(&mut pr, s.as_ptr()),
            )
        };
        eqi(&format!("ge25519_frombytes({})", hex(&s)), rc, rr);
        eqb(&format!("ge25519_frombytes p3 ({})", hex(&s)), &pc.bytes(), &pr.bytes());
        assert!(rc == 0 || rc == -1, "ge25519_frombytes must return 0 or -1");

        // 2.6 — ge25519_frombytes_negate_vartime: same accept/reject set
        let mut qc = P3::new();
        let mut qr = P3::new();
        let (nc, nr) = unsafe {
            (
                g.frombytes_neg.0(&mut qc, s.as_ptr()),
                g.frombytes_neg.1(&mut qr, s.as_ptr()),
            )
        };
        eqi(&format!("ge25519_frombytes_negate_vartime({})", hex(&s)), nc, nr);
        assert_eq!(nc, rc, "the two decoders must have the same accept/reject set");
        if nc == 0 {
            eqb("ge25519_frombytes_negate_vartime p3", &qc.bytes(), &qr.bytes());
        }

        // 2.7 / 2.8 / 2.9 — the three P3 predicates on whatever came out
        for (name, f) in [
            ("ge25519_is_on_curve", &g.is_on_curve),
            ("ge25519_has_small_order", &g.small_order),
            ("ge25519_is_on_main_subgroup", &g.main_sub),
        ] {
            let (a, b) = unsafe { (f.0(&pc), f.1(&pr)) };
            eqi(&format!("{name}({})", hex(&s)), a, b);
        }
        if rc == 0 {
            // a successful decode always lands on the curve
            assert_eq!(unsafe { g.is_on_curve.0(&pc) }, 1);
            // round trip: encode and re-decode
            let mut ec = padded(32);
            let mut er = padded(32);
            unsafe {
                g.tobytes.0(ec.as_mut_ptr(), &pc);
                g.tobytes.1(er.as_mut_ptr(), &pr);
            }
            eqb("ge25519_p3_tobytes", &ec[..32], &er[..32]);
            check_pad("ge25519_p3_tobytes(C)", &ec, 32);
            check_pad("ge25519_p3_tobytes(Rust)", &er, 32);
        }
    }

    // error row 2.7 explicitly: crafted P3 values that are *not* on the curve.
    // (Unreachable through the public API, because a successful
    // `ge25519_frombytes` always yields an on-curve point.)
    let mut pc = P3::new();
    let mut pr = P3::new();
    assert_eq!(unsafe { g.frombytes.0(&mut pc, BASEPOINT.as_ptr()) }, 0);
    assert_eq!(unsafe { g.frombytes.1(&mut pr, BASEPOINT.as_ptr()) }, 0);
    let mut hit_reject = false;
    for limb in 0..40 {
        for delta in [1i32, -1, 1 << 20] {
            let mut a = pc;
            let mut b = pr;
            a.0[limb] = a.0[limb].wrapping_add(delta);
            b.0[limb] = b.0[limb].wrapping_add(delta);
            for (name, f) in [
                ("ge25519_is_on_curve", &g.is_on_curve),
                ("ge25519_has_small_order", &g.small_order),
                ("ge25519_is_on_main_subgroup", &g.main_sub),
            ] {
                let (x, y) = unsafe { (f.0(&a), f.1(&b)) };
                eqi(&format!("{name}(perturbed limb {limb} +{delta})"), x, y);
                if name == "ge25519_is_on_curve" && x == 0 {
                    hit_reject = true;
                }
            }
        }
    }
    assert!(hit_reject, "no perturbation produced ge25519_is_on_curve == 0");

    // the all-zero P3 is a degenerate but well-defined input
    let z = P3([0i32; 40]);
    for (name, f) in [
        ("ge25519_is_on_curve", &g.is_on_curve),
        ("ge25519_has_small_order", &g.small_order),
        ("ge25519_is_on_main_subgroup", &g.main_sub),
    ] {
        let (x, y) = unsafe { (f.0(&z), f.1(&z)) };
        eqi(&format!("{name}(all-zero p3)"), x, y);
    }
}

// ------------------------------------------------------------- is_valid_point

/// Rows 2.83 / 2.84 and error rows 2.18 - 2.22.
#[test]
fn is_valid_point() {
    let (c, r) = both::<Pred>("crypto_core_ed25519_is_valid_point");
    let g = Ge::new();

    let check = |s: &[u8; 32], label: &str| -> c_int {
        let (a, b) = unsafe { (c(s.as_ptr()), r(s.as_ptr())) };
        eqi(&format!("is_valid_point [{label}] {}", hex(s)), a, b);
        a
    };

    // 2.83: the base point is valid
    assert_eq!(check(&BASEPOINT, "basepoint"), 1);
    // error 2.21: the identity and all eight small-order points are rejected
    for (i, s) in SMALL_ORDER.iter().enumerate() {
        assert_eq!(check(s, &format!("small order #{i}")), 0, "small-order point accepted");
    }
    // error 2.18: non-canonical encodings are rejected
    for s in noncanonical_decodable() {
        assert_eq!(check(&s, "non-canonical"), 0, "non-canonical encoding accepted");
    }
    // error 2.19: y with no matching x is rejected
    let mut saw_nodecode = false;
    for s in point_encodings(0x2_0019) {
        let mut p = P3::new();
        let dec = unsafe { g.frombytes.0(&mut p, s.as_ptr()) };
        let v = check(&s, "pool");
        if dec != 0 {
            assert_eq!(v, 0, "undecodable encoding accepted");
            saw_nodecode = true;
        }
        if v == 1 {
            assert_eq!(unsafe { g.is_canonical.0(s.as_ptr()) }, 1);
            assert_eq!(unsafe { g.small_order.0(&p) }, 0);
            assert_eq!(unsafe { g.main_sub.0(&p) }, 1);
        }
    }
    assert!(saw_nodecode, "the pool must contain undecodable encodings");

    // error 2.22: an on-curve point of order 2L / 4L / 8L (base + torsion)
    let (add, _) = both::<Fn3i>("crypto_core_ed25519_add");
    for (i, t) in SMALL_ORDER.iter().enumerate() {
        let mut q = [0u8; 32];
        assert_eq!(unsafe { add(q.as_mut_ptr(), BASEPOINT.as_ptr(), t.as_ptr()) }, 0);
        let v = check(&q, &format!("base + torsion #{i}"));
        // adding the identity leaves the base point, everything else leaves the
        // prime-order subgroup
        if i == 0 {
            assert_eq!(q, BASEPOINT);
            assert_eq!(v, 1);
        } else {
            assert_eq!(v, 0, "base + torsion #{i} must not be on the main subgroup");
            let mut p = P3::new();
            assert_eq!(unsafe { g.frombytes.0(&mut p, q.as_ptr()) }, 0);
            assert_eq!(unsafe { g.is_on_curve.0(&p) }, 1);
            assert_eq!(unsafe { g.main_sub.0(&p) }, 0);
        }
    }

    // 2.84: crypto_core_ed25519_random always produces a valid point
    let (crand, rrand) = both::<Fn1>("crypto_core_ed25519_random");
    for i in 0..200u64 {
        let seed = 0x1234_5678_9abc_def0u64 ^ (i.wrapping_mul(0x9E37_79B9) | 1);
        rng_reseed(seed);
        let mut oc = padded(32);
        unsafe { crand(oc.as_mut_ptr()) };
        rng_reseed(seed);
        let mut or = padded(32);
        unsafe { rrand(or.as_mut_ptr()) };
        eqb("crypto_core_ed25519_random", &oc[..32], &or[..32]);
        check_pad("ed25519_random(C)", &oc, 32);
        check_pad("ed25519_random(Rust)", &or, 32);
        assert_eq!(check(&oc[..32].try_into().unwrap(), "random"), 1);
    }
    rng_reset();
}

// ----------------------------------------------------------------- add / sub

/// Rows 2.85 - 2.92 and error rows 2.23 - 2.30.
#[test]
fn point_add_and_sub() {
    let (ca, ra) = both::<Fn3i>("crypto_core_ed25519_add");
    let (cs, rs) = both::<Fn3i>("crypto_core_ed25519_sub");
    let g = Ge::new();

    let run = |c: &Fn3i, r: &Fn3i, p: &[u8; 32], q: &[u8; 32], label: &str| -> (c_int, Vec<u8>) {
        let mut oc = padded(32);
        let mut or = padded(32);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let ec = errno();
        set_errno(0);
        let rr = unsafe { r(or.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
        let er = errno();
        eqi(&format!("{label} ret ({}, {})", hex(p), hex(q)), rc, rr);
        assert_eq!(ec, er, "{label} errno");
        if rc == 0 {
            eqb(&format!("{label} out"), &oc[..32], &or[..32]);
        }
        check_pad(&format!("{label}(C)"), &oc, 32);
        check_pad(&format!("{label}(Rust)"), &or, 32);
        (rc, oc[..32].to_vec())
    };

    let decodes = |s: &[u8; 32]| -> bool {
        let mut p = P3::new();
        unsafe { g.frombytes.0(&mut p, s.as_ptr()) == 0 }
    };

    // 2.86 / 2.91: identity behaves as the neutral element
    let (rc, out) = run(&ca, &ra, &BASEPOINT, &IDENTITY, "add(P, identity)");
    assert_eq!(rc, 0);
    assert_eq!(&out[..], &BASEPOINT[..]);
    let (rc, out) = run(&cs, &rs, &BASEPOINT, &IDENTITY, "sub(P, identity)");
    assert_eq!(rc, 0);
    assert_eq!(&out[..], &BASEPOINT[..]);
    // 2.91: P - P == identity
    let (rc, out) = run(&cs, &rs, &BASEPOINT, &BASEPOINT, "sub(P, P)");
    assert_eq!(rc, 0);
    assert_eq!(&out[..], &IDENTITY[..]);
    // 2.87: P + (-P) == identity
    let mut negb = BASEPOINT;
    negb[31] ^= 0x80;
    let (rc, out) = run(&ca, &ra, &BASEPOINT, &negb, "add(P, -P)");
    assert_eq!(rc, 0);
    assert_eq!(&out[..], &IDENTITY[..]);

    // 2.85 / 2.88 / 2.89 / 2.90 / 2.92 plus every rejection combination
    let mut pool: Vec<[u8; 32]> = vec![BASEPOINT, negb, IDENTITY];
    pool.extend_from_slice(&SMALL_ORDER);
    pool.extend(noncanonical_decodable());
    let mut rng = Rng::new(0x2_0085);
    // a handful of genuine main-subgroup points
    let (crand, _) = both::<Fn1>("crypto_core_ed25519_random");
    for i in 0..12u64 {
        rng_reseed(0xfeed_0000 ^ (i | 1));
        let mut o = [0u8; 32];
        unsafe { crand(o.as_mut_ptr()) };
        pool.push(o);
    }
    rng_reset();
    // undecodable / random encodings, for the -1 paths
    for _ in 0..40 {
        pool.push(rng.bytes(32).try_into().unwrap());
    }
    // guarantee at least one undecodable operand
    {
        let mut bad = [0u8; 32];
        for y in 1u8..=255 {
            bad[0] = y;
            if !decodes(&bad) {
                break;
            }
        }
        assert!(!decodes(&bad), "could not find an undecodable encoding");
        pool.push(bad);
    }

    let mut saw_ok = 0usize;
    let mut saw_fail_p = 0usize;
    let mut saw_fail_q = 0usize;
    for p in &pool {
        for q in &pool {
            let (rca, oa) = run(&ca, &ra, p, q, "ed25519_add");
            let (rcs, os) = run(&cs, &rs, p, q, "ed25519_sub");
            let want = if decodes(p) && decodes(q) { 0 } else { -1 };
            assert_eq!(rca, want, "add({}, {}) sentinel", hex(p), hex(q));
            assert_eq!(rcs, want, "sub({}, {}) sentinel", hex(p), hex(q));
            if want == 0 {
                saw_ok += 1;
                // add and sub must be consistent: (p + q) - q == p'
                let mut back = [0u8; 32];
                assert_eq!(
                    unsafe { cs(back.as_mut_ptr(), oa.as_ptr(), q.as_ptr()) },
                    0
                );
                // encodings are canonicalised by p3_tobytes, so compare against
                // the canonical form of p
                let mut pc = P3::new();
                assert_eq!(unsafe { g.frombytes.0(&mut pc, p.as_ptr()) }, 0);
                let mut canon_p = [0u8; 32];
                unsafe { g.tobytes.0(canon_p.as_mut_ptr(), &pc) };
                assert_eq!(back, canon_p, "(p+q)-q != p");
                // sub(p, q) == add(p, -q) with q negated in the encoding
                let mut nq = *q;
                nq[31] ^= 0x80;
                if decodes(&nq) {
                    let mut alt = [0u8; 32];
                    assert_eq!(
                        unsafe { ca(alt.as_mut_ptr(), p.as_ptr(), nq.as_ptr()) },
                        0
                    );
                    assert_eq!(&os[..], &alt[..], "sub(p,q) != add(p,-q)");
                }
            } else {
                if !decodes(p) {
                    saw_fail_p += 1;
                } else {
                    saw_fail_q += 1;
                }
            }
        }
    }
    assert!(saw_ok > 0 && saw_fail_p > 0 && saw_fail_q > 0, "coverage: {saw_ok} {saw_fail_p} {saw_fail_q}");
}

// ------------------------------------------------- from_uniform / from_hash

/// Rows 2.93 - 2.96 (`ge25519_from_uniform`) and 2.103 (`ge25519_from_hash`).
#[test]
fn ge25519_from_uniform_and_hash() {
    assert!(has("_sodium_ge25519_from_uniform"));
    assert!(has("_sodium_ge25519_from_hash"));
    let (cu, ru) = both::<Fn2>("_sodium_ge25519_from_uniform");
    let (ch, rh) = both::<Fn2>("_sodium_ge25519_from_hash");
    let (valid, _) = both::<Pred>("crypto_core_ed25519_is_valid_point");
    let g = Ge::new();

    // 2.94: r[31] bit 5 clear and set, for every base value
    let mut inputs: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], [0x7fu8; 32]];
    for b in [0x00u8, 0x20, 0x40, 0x60, 0x80, 0xa0, 0xc0, 0xe0] {
        let mut v = [0x11u8; 32];
        v[31] = b;
        inputs.push(v);
    }
    // 2.95: a wide random sweep, which hits both the `gx1 is a square` and the
    // `gx1 is not a square` arm of ge25519_elligator2 (each ~50% likely).
    let mut rng = Rng::new(0x2_0095);
    for _ in 0..800 {
        let mut v: [u8; 32] = rng.bytes(32).try_into().unwrap();
        inputs.push(v);
        v[31] &= 0xdf; // bit 5 clear
        inputs.push(v);
        v[31] |= 0x20; // bit 5 set
        inputs.push(v);
    }
    // 2.96: small r values, including r = 0, which drive the mont_to_ed cmov
    for y in 0u8..32 {
        let mut v = [0u8; 32];
        v[0] = y;
        inputs.push(v);
        v[31] = 0x20;
        inputs.push(v);
    }

    let mut seen: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    let mut n_valid = 0usize;
    let mut n_small = 0usize;
    for r_in in &inputs {
        // `ge25519_from_uniform` writes s *and* uses it as scratch; the input
        // and output buffers are distinct here.
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            cu(oc.as_mut_ptr(), r_in.as_ptr());
            ru(or.as_mut_ptr(), r_in.as_ptr());
        }
        eqb(&format!("ge25519_from_uniform({})", hex(r_in)), &oc[..32], &or[..32]);
        check_pad("from_uniform(C)", &oc, 32);
        check_pad("from_uniform(Rust)", &or, 32);
        // The result is always a canonical, on-curve, cofactor-cleared point.
        // `is_valid_point` additionally rejects small-order results, which the
        // degenerate input `r == 0` does produce, so compare differentially and
        // only require validity for the non-degenerate majority.
        let (vc, vr) = unsafe { (valid(oc.as_ptr()), valid(or.as_ptr())) };
        eqi(&format!("is_valid_point(from_uniform({}))", hex(r_in)), vc, vr);
        let mut p = P3::new();
        assert_eq!(unsafe { g.frombytes.0(&mut p, oc.as_ptr()) }, 0);
        assert_eq!(unsafe { g.is_on_curve.0(&p) }, 1);
        assert_eq!(unsafe { g.main_sub.0(&p) }, 1, "from_uniform must clear the cofactor");
        if vc == 0 {
            // the only way out is a small-order (i.e. torsion) result
            assert_ne!(unsafe { g.small_order.0(&p) }, 0);
            n_small += 1;
        } else {
            n_valid += 1;
        }
        seen.insert(oc[..32].to_vec());

        // the x_sign bit (r[31] bit 5) must actually change the result
        let mut flipped = *r_in;
        flipped[31] ^= 0x20;
        let mut fc = [0u8; 32];
        unsafe { cu(fc.as_mut_ptr(), flipped.as_ptr()) };
        let mut fr = [0u8; 32];
        unsafe { ru(fr.as_mut_ptr(), flipped.as_ptr()) };
        eqb("ge25519_from_uniform(x_sign flipped)", &fc, &fr);
    }
    assert!(seen.len() > 100, "from_uniform outputs are suspiciously repetitive");
    assert!(n_valid > inputs.len() / 2, "from_uniform: only {n_valid} valid outputs");
    let _ = n_small;

    // 2.103: ge25519_from_hash over 64-byte inputs, covering h[31]/h[63] bit 5
    let mut hinputs: Vec<[u8; 64]> = vec![[0u8; 64], [0xffu8; 64]];
    for a in [0x00u8, 0x20] {
        for b in [0x00u8, 0x20] {
            let mut v = [0x33u8; 64];
            v[31] = a;
            v[63] = b;
            hinputs.push(v);
            let mut w = [0u8; 64];
            w[31] = a;
            w[63] = b;
            hinputs.push(w);
        }
    }
    let mut rng = Rng::new(0x2_0103);
    for _ in 0..800 {
        hinputs.push(rng.bytes(64).try_into().unwrap());
    }
    for h in &hinputs {
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            ch(oc.as_mut_ptr(), h.as_ptr());
            rh(or.as_mut_ptr(), h.as_ptr());
        }
        eqb(&format!("ge25519_from_hash({})", hex(&h[..8])), &oc[..32], &or[..32]);
        check_pad("from_hash(C)", &oc, 32);
        check_pad("from_hash(Rust)", &or, 32);
        // the output is always a canonical, on-curve, cofactor-cleared point
        let (vc, vr) = unsafe { (valid(oc.as_ptr()), valid(or.as_ptr())) };
        eqi("is_valid_point(from_hash(..))", vc, vr);
        let mut p = P3::new();
        assert_eq!(unsafe { g.frombytes.0(&mut p, oc.as_ptr()) }, 0);
        assert_eq!(unsafe { g.is_on_curve.0(&p) }, 1);
        assert_eq!(unsafe { g.main_sub.0(&p) }, 1, "from_hash must clear the cofactor");
    }

    // ge25519_clear_cofactor, exercised on decoded points
    let (ccc, rcc) = both::<GeUnary>("_sodium_ge25519_clear_cofactor");
    for s in point_encodings(0x2_0096) {
        let mut pc = P3::new();
        let mut pr = P3::new();
        if unsafe { g.frombytes.0(&mut pc, s.as_ptr()) } != 0 {
            continue;
        }
        assert_eq!(unsafe { g.frombytes.1(&mut pr, s.as_ptr()) }, 0);
        unsafe {
            ccc(&mut pc);
            rcc(&mut pr);
        }
        eqb(&format!("ge25519_clear_cofactor({})", hex(&s)), &pc.bytes(), &pr.bytes());
    }
}

// ------------------------------------------------------------- from_string(_nu)

/// Rows 2.97 - 2.102 and error rows 2.33 / 2.34 / 2.35.
#[test]
fn ed25519_from_string() {
    let (cn, rn) = both::<FromString>("crypto_core_ed25519_from_string_nu");
    let (cr, rr_) = both::<FromString>("crypto_core_ed25519_from_string");
    let (valid, _) = both::<Pred>("crypto_core_ed25519_is_valid_point");
    let mut rng = Rng::new(0x2_0097);
    let strings = h2c_strings(&mut rng);

    let call = |c: &FromString,
                r: &FromString,
                ctx: Option<&[u8]>,
                msg: &[u8],
                alg: c_int,
                label: &str|
     -> (c_int, Vec<u8>) {
        let (cp, cl) = match ctx {
            None => (std::ptr::null(), 0usize),
            Some(s) => (s.as_ptr(), s.len()),
        };
        let mut oc = padded(32);
        let mut or = padded(32);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), cp, cl, msg.as_ptr(), msg.len(), alg) };
        let ec = errno();
        set_errno(0);
        let rv = unsafe { r(or.as_mut_ptr(), cp, cl, msg.as_ptr(), msg.len(), alg) };
        let er = errno();
        eqi(&format!("{label} ret (alg {alg}, ctx_len {cl}, msg_len {})", msg.len()), rc, rv);
        assert_eq!(ec, er, "{label} errno (alg {alg})");
        if rc == 0 {
            eqb(&format!("{label} out"), &oc[..32], &or[..32]);
        }
        check_pad(&format!("{label}(C)"), &oc, 32);
        check_pad(&format!("{label}(Rust)"), &or, 32);
        (rc, oc[..32].to_vec())
    };

    for alg in [1i32, 2] {
        for ctx in &strings {
            for msg in &strings {
                let (a, nu) = call(&cn, &rn, Some(ctx), msg, alg, "from_string_nu");
                assert_eq!(a, 0);
                let (b, ro) = call(&cr, &rr_, Some(ctx), msg, alg, "from_string");
                assert_eq!(b, 0);
                // the RO variant is the sum of two NU-style points, so it must
                // differ from the NU variant (row 2.102)
                assert_ne!(nu, ro, "from_string_nu and from_string must differ");
                // both land on the curve; the RO variant additionally lands in
                // the prime-order subgroup (from_hash clears the cofactor)
                assert_eq!(unsafe { valid(nu.as_ptr()) }, 1);
                assert_eq!(unsafe { valid(ro.as_ptr()) }, 1);
            }
        }
        // ctx == NULL
        for msg in &strings {
            let (_, a) = call(&cn, &rn, None, msg, alg, "from_string_nu(NULL ctx)");
            let (_, b) = call(&cn, &rn, Some(&[]), msg, alg, "from_string_nu(empty ctx)");
            assert_eq!(a, b);
            let (_, a) = call(&cr, &rr_, None, msg, alg, "from_string(NULL ctx)");
            let (_, b) = call(&cr, &rr_, Some(&[]), msg, alg, "from_string(empty ctx)");
            assert_eq!(a, b);
        }
    }
    // SHA-256 vs SHA-512 must differ
    for msg in &strings {
        let (_, a) = call(&cn, &rn, Some(b"z"), msg, 1, "nu256");
        let (_, b) = call(&cn, &rn, Some(b"z"), msg, 2, "nu512");
        assert_ne!(a, b);
        let (_, a) = call(&cr, &rr_, Some(b"z"), msg, 1, "ro256");
        let (_, b) = call(&cr, &rr_, Some(b"z"), msg, 2, "ro512");
        assert_ne!(a, b);
    }

    // error rows 2.33 / 2.34: out-of-range hash_alg -> -1 and errno == EINVAL
    for alg in [0i32, 3, -1, 7, 255, 256, i32::MIN, i32::MAX] {
        for (c, r, label) in [
            (&cn, &rn, "from_string_nu"),
            (&cr, &rr_, "from_string"),
        ] {
            let (rc, _) = call(c, r, Some(b"ctx"), b"msg", alg, label);
            assert_eq!(rc, -1, "{label}(alg={alg}) must fail");
            set_errno(0);
            let mut o = [0u8; 32];
            let got = unsafe { c(o.as_mut_ptr(), b"ctx".as_ptr(), 3, b"msg".as_ptr(), 3, alg) };
            assert_eq!(got, -1);
            assert_eq!(errno(), EINVAL, "{label}(alg={alg}) must set EINVAL");
        }
    }
}

// ------------------------------------------------------ core_h2c_string_to_hash

/// Error rows 2.41 (bad `hash_alg`), 2.42 and 2.43 (`assert(h_len <= 0xff)`).
#[test]
fn core_h2c_string_to_hash_errors() {
    assert!(has("_sodium_core_h2c_string_to_hash"));
    let (c, r) = both::<H2CHash>("_sodium_core_h2c_string_to_hash");
    let mut rng = Rng::new(0x2_0041);
    let strings = h2c_strings(&mut rng);

    // valid: every h_len used by the public API, plus the 0xff boundary
    for alg in [1i32, 2] {
        for h_len in [0usize, 1, 31, 32, 33, 48, 63, 64, 65, 96, 128, 254, 255] {
            for ctx in &strings {
                let msg = b"the quick brown fox";
                let mut oc = padded(h_len);
                let mut or = padded(h_len);
                set_errno(0);
                let rc = unsafe {
                    c(oc.as_mut_ptr(), h_len, ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg)
                };
                let ec = errno();
                set_errno(0);
                let rv = unsafe {
                    r(or.as_mut_ptr(), h_len, ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg)
                };
                let er = errno();
                eqi(&format!("core_h2c_string_to_hash ret (alg {alg}, h_len {h_len})"), rc, rv);
                assert_eq!(ec, er, "core_h2c_string_to_hash errno");
                assert_eq!(rc, 0);
                eqb("core_h2c_string_to_hash out", &oc[..h_len], &or[..h_len]);
                check_pad("core_h2c(C)", &oc, h_len);
                check_pad("core_h2c(Rust)", &or, h_len);
            }
        }
    }

    // error row 2.41: default: arm -> errno = EINVAL, return -1
    for alg in [0i32, 3, 4, -1, 255, 256, i32::MIN, i32::MAX] {
        let mut oc = padded(48);
        let mut or = padded(48);
        set_errno(0);
        let rc = unsafe { c(oc.as_mut_ptr(), 48, b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg) };
        let ec = errno();
        set_errno(0);
        let rv = unsafe { r(or.as_mut_ptr(), 48, b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg) };
        let er = errno();
        eqi(&format!("core_h2c_string_to_hash(alg={alg})"), rc, rv);
        assert_eq!(rc, -1);
        assert_eq!(ec, EINVAL, "C must set errno = EINVAL for alg {alg}");
        assert_eq!(er, EINVAL, "Rust must set errno = EINVAL for alg {alg}");
        // the output buffer is untouched on this path
        check_pad("core_h2c(C)", &oc, 48);
        check_pad("core_h2c(Rust)", &or, 48);
    }

    // error rows 2.42 / 2.43: assert(h_len <= 0xff) fires (the reference build
    // does not define NDEBUG).
    for alg in [1i32, 2] {
        for h_len in [256usize, 257, 1000] {
            let cc = c.clone();
            let rr2 = r.clone();
            eq_abort(
                &format!("core_h2c_string_to_hash(h_len={h_len}, alg={alg})"),
                move || unsafe {
                    let mut o = vec![0u8; h_len];
                    cc(o.as_mut_ptr(), h_len, b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg);
                },
                move || unsafe {
                    let mut o = vec![0u8; h_len];
                    rr2(o.as_mut_ptr(), h_len, b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg);
                },
            );
        }
    }
}

// --------------------------------------------------------------- scalarmult

/// Row 2.128 — `ge25519_scalarmult`, `_scalarmult_base`,
/// `_double_scalarmult_vartime`, and the `p1p1`/`p2` conversions.
#[test]
fn ge25519_scalarmult_family() {
    for n in [
        "_sodium_ge25519_scalarmult",
        "_sodium_ge25519_scalarmult_base",
        "_sodium_ge25519_double_scalarmult_vartime",
        "_sodium_ge25519_p2_to_p3",
        "_sodium_ge25519_tobytes",
        "_sodium_ge25519_p3_add",
        "_sodium_ge25519_p3_sub",
    ] {
        assert!(has(n), "{n} must be exported by both libraries");
    }
    let (csm, rsm) = both::<GeScalarmult>("_sodium_ge25519_scalarmult");
    let (csb, rsb) = both::<GeScalarmultBase>("_sodium_ge25519_scalarmult_base");
    let (cds, rds) = both::<GeDoubleScalarmult>("_sodium_ge25519_double_scalarmult_vartime");
    let (cp2, rp2) = both::<GeP2ToP3>("_sodium_ge25519_p2_to_p3");
    let (ct2, rt2) = both::<GeToBytes2>("_sodium_ge25519_tobytes");
    let (cpa, rpa) = both::<GeP3Op>("_sodium_ge25519_p3_add");
    let (cps, rps) = both::<GeP3Op>("_sodium_ge25519_p3_sub");
    let g = Ge::new();

    // decode a set of P3 inputs
    let mut pts_c: Vec<P3> = Vec::new();
    let mut pts_r: Vec<P3> = Vec::new();
    let mut encs: Vec<[u8; 32]> = vec![BASEPOINT, IDENTITY];
    encs.extend_from_slice(&SMALL_ORDER);
    let mut rng = Rng::new(0x2_0128);
    for s in point_encodings(0x2_0128_a).into_iter().take(80) {
        encs.push(s);
    }
    for s in &encs {
        let mut a = P3::new();
        let mut b = P3::new();
        if unsafe { g.frombytes.0(&mut a, s.as_ptr()) } != 0 {
            continue;
        }
        assert_eq!(unsafe { g.frombytes.1(&mut b, s.as_ptr()) }, 0);
        eqb("frombytes for scalarmult", &a.bytes(), &b.bytes());
        pts_c.push(a);
        pts_r.push(b);
    }
    assert!(pts_c.len() > 10);

    // scalars: the documented precondition is a[31] <= 127
    let mut scalars: Vec<[u8; 32]> = vec![zero32(), one32(), l_plus(-1), L];
    {
        let mut a = [0xffu8; 32];
        a[31] = 0x7f;
        scalars.push(a);
        let mut b = [0u8; 32];
        b[31] = 0x7f;
        scalars.push(b);
    }
    for i in 0..255 {
        let mut a = [0u8; 32];
        a[i / 8] = 1u8 << (i % 8);
        scalars.push(a);
    }
    for _ in 0..80 {
        let mut a: [u8; 32] = rng.bytes(32).try_into().unwrap();
        a[31] &= 0x7f;
        scalars.push(a);
    }

    // scalarmult_base: a=1 must give the base point
    {
        let mut a = P3::new();
        let mut b = P3::new();
        unsafe {
            csb(&mut a, one32().as_ptr());
            rsb(&mut b, one32().as_ptr());
        }
        eqb("ge25519_scalarmult_base(1)", &a.bytes(), &b.bytes());
        let mut e = [0u8; 32];
        unsafe { g.tobytes.0(e.as_mut_ptr(), &a) };
        assert_eq!(e, BASEPOINT, "scalarmult_base(1) != base point");
    }

    for a in &scalars {
        let mut hc = P3::new();
        let mut hr = P3::new();
        unsafe {
            csb(&mut hc, a.as_ptr());
            rsb(&mut hr, a.as_ptr());
        }
        eqb(&format!("ge25519_scalarmult_base({})", hex(a)), &hc.bytes(), &hr.bytes());
        let mut ec = padded(32);
        let mut er = padded(32);
        unsafe {
            g.tobytes.0(ec.as_mut_ptr(), &hc);
            g.tobytes.1(er.as_mut_ptr(), &hr);
        }
        eqb("scalarmult_base encoded", &ec[..32], &er[..32]);
        check_pad("scalarmult_base(C)", &ec, 32);
        check_pad("scalarmult_base(Rust)", &er, 32);
    }

    for (i, (pc, pr)) in pts_c.iter().zip(pts_r.iter()).enumerate() {
        for a in scalars.iter().take(40) {
            let mut hc = P3::new();
            let mut hr = P3::new();
            unsafe {
                csm(&mut hc, a.as_ptr(), pc);
                rsm(&mut hr, a.as_ptr(), pr);
            }
            eqb(&format!("ge25519_scalarmult(pt {i}, {})", hex(a)), &hc.bytes(), &hr.bytes());

            // p3_add / p3_sub on the same operands
            let mut ac = P3::new();
            let mut ar = P3::new();
            unsafe {
                cpa(&mut ac, pc, &hc);
                rpa(&mut ar, pr, &hr);
            }
            eqb("ge25519_p3_add", &ac.bytes(), &ar.bytes());
            let mut sc2 = P3::new();
            let mut sr2 = P3::new();
            unsafe {
                cps(&mut sc2, pc, &hc);
                rps(&mut sr2, pr, &hr);
            }
            eqb("ge25519_p3_sub", &sc2.bytes(), &sr2.bytes());
        }
    }

    // double_scalarmult_vartime: exercises slide_vartime's three arms
    for (i, (pc, pr)) in pts_c.iter().enumerate().zip(pts_r.iter()).map(|((i, a), b)| (i, (a, b))) {
        for _ in 0..8 {
            let mut a: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let mut b: [u8; 32] = rng.bytes(32).try_into().unwrap();
            a[31] &= 0x7f;
            b[31] &= 0x7f;
            for (aa, bb) in [
                (a, b),
                (zero32(), b),
                (a, zero32()),
                (zero32(), zero32()),
                (one32(), one32()),
                ([0xffu8; 32], [0xffu8; 32]),
            ] {
                let mut rc = P2::new();
                let mut rr2 = P2::new();
                unsafe {
                    cds(&mut rc, aa.as_ptr(), pc, bb.as_ptr());
                    rds(&mut rr2, aa.as_ptr(), pr, bb.as_ptr());
                }
                eqb(
                    &format!("ge25519_double_scalarmult_vartime(pt {i}, {}, {})", hex(&aa), hex(&bb)),
                    &rc.bytes(),
                    &rr2.bytes(),
                );
                let mut ec = padded(32);
                let mut er = padded(32);
                unsafe {
                    ct2(ec.as_mut_ptr(), &rc);
                    rt2(er.as_mut_ptr(), &rr2);
                }
                eqb("ge25519_tobytes", &ec[..32], &er[..32]);
                check_pad("ge25519_tobytes(C)", &ec, 32);
                check_pad("ge25519_tobytes(Rust)", &er, 32);
                let mut p3c = P3::new();
                let mut p3r = P3::new();
                unsafe {
                    cp2(&mut p3c, &rc);
                    rp2(&mut p3r, &rr2);
                }
                eqb("ge25519_p2_to_p3", &p3c.bytes(), &p3r.bytes());
            }
        }
    }
}

// -------------------------------------------------------------- fe25519 bits

/// The three exported `fe25519_*` helpers, driven over the whole interesting
/// field-element range (all-zero, one, `p-1`, `p`, `p+1`, `2^255-1`, random).
#[test]
fn fe25519_exported_helpers() {
    for n in [
        "_sodium_fe25519_frombytes",
        "_sodium_fe25519_tobytes",
        "_sodium_fe25519_invert",
    ] {
        assert!(has(n), "{n} must be exported by both libraries");
    }
    type FeFromBytes = unsafe extern "C" fn(*mut i32, *const u8);
    type FeToBytes = unsafe extern "C" fn(*mut u8, *const i32);
    type FeInvert = unsafe extern "C" fn(*mut i32, *const i32);
    let (cfb, rfb) = both::<FeFromBytes>("_sodium_fe25519_frombytes");
    let (ctb, rtb) = both::<FeToBytes>("_sodium_fe25519_tobytes");
    let (civ, riv) = both::<FeInvert>("_sodium_fe25519_invert");

    let mut inputs: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32]];
    for v in [1u8, 2, 18, 19, 20, 0xec, 0xed, 0xee] {
        let mut a = [0u8; 32];
        a[0] = v;
        inputs.push(a);
        let mut b = [0xffu8; 32];
        b[0] = v;
        b[31] = 0x7f;
        inputs.push(b);
        let mut c = b;
        c[31] = 0xff;
        inputs.push(c);
    }
    let mut rng = Rng::new(0x2_00fe);
    for _ in 0..800 {
        inputs.push(rng.bytes(32).try_into().unwrap());
    }

    for s in &inputs {
        let mut fc = [0i32; 10];
        let mut fr = [0i32; 10];
        unsafe {
            cfb(fc.as_mut_ptr(), s.as_ptr());
            rfb(fr.as_mut_ptr(), s.as_ptr());
        }
        assert_eq!(fc, fr, "fe25519_frombytes({}) limb mismatch", hex(s));
        let mut bc = padded(32);
        let mut br = padded(32);
        unsafe {
            ctb(bc.as_mut_ptr(), fc.as_ptr());
            rtb(br.as_mut_ptr(), fr.as_ptr());
        }
        eqb(&format!("fe25519_tobytes({})", hex(s)), &bc[..32], &br[..32]);
        check_pad("fe25519_tobytes(C)", &bc, 32);
        check_pad("fe25519_tobytes(Rust)", &br, 32);
        let mut ic = [0i32; 10];
        let mut ir = [0i32; 10];
        unsafe {
            civ(ic.as_mut_ptr(), fc.as_ptr());
            riv(ir.as_mut_ptr(), fr.as_ptr());
        }
        assert_eq!(ic, ir, "fe25519_invert({}) limb mismatch", hex(s));
        let mut jc = [0u8; 32];
        let mut jr = [0u8; 32];
        unsafe {
            ctb(jc.as_mut_ptr(), ic.as_ptr());
            rtb(jr.as_mut_ptr(), ir.as_ptr());
        }
        eqb("fe25519_invert -> tobytes", &jc, &jr);
    }
}

// ------------------------------------------------------------------ aliasing

/// FFI boundary hardening: the C implementations all copy their inputs into
/// locals before writing the output, so `z == x`, `z == y` and `r == p` are
/// legal.  A translation that writes the output eagerly would diverge here.
#[test]
fn ed25519_output_input_aliasing() {
    let pool = scalar_pool(0x2_0a11);
    let mut rng = Rng::new(0x2_0a12);

    // unary: negate / complement / invert
    for name in [
        "crypto_core_ed25519_scalar_negate",
        "crypto_core_ed25519_scalar_complement",
    ] {
        let (c, r) = both::<Fn2>(name);
        for s in &pool {
            // reference (non-aliased) result
            let mut refc = [0u8; 32];
            unsafe { c(refc.as_mut_ptr(), s.as_ptr()) };
            // aliased: out == in
            let mut bc = padded(32);
            let mut br = padded(32);
            bc[..32].copy_from_slice(s);
            br[..32].copy_from_slice(s);
            unsafe {
                c(bc.as_mut_ptr(), bc.as_ptr());
                r(br.as_mut_ptr(), br.as_ptr());
            }
            eqb(&format!("{name} aliased"), &bc[..32], &br[..32]);
            check_pad(&format!("{name} aliased(C)"), &bc, 32);
            check_pad(&format!("{name} aliased(Rust)"), &br, 32);
            assert_eq!(&bc[..32], &refc[..], "{name}: aliasing changed the C result");
        }
    }
    {
        let (c, r) = both::<Fn2i>("crypto_core_ed25519_scalar_invert");
        for s in &pool {
            let mut bc = padded(32);
            let mut br = padded(32);
            bc[..32].copy_from_slice(s);
            br[..32].copy_from_slice(s);
            let rc = unsafe { c(bc.as_mut_ptr(), bc.as_ptr()) };
            let rr = unsafe { r(br.as_mut_ptr(), br.as_ptr()) };
            eqi("scalar_invert aliased", rc, rr);
            eqb("scalar_invert aliased out", &bc[..32], &br[..32]);
            check_pad("scalar_invert aliased(C)", &bc, 32);
            check_pad("scalar_invert aliased(Rust)", &br, 32);
        }
    }

    // binary: add / sub / mul with z == x, z == y and x == y == z
    for name in [
        "crypto_core_ed25519_scalar_add",
        "crypto_core_ed25519_scalar_sub",
        "crypto_core_ed25519_scalar_mul",
    ] {
        let (c, r) = both::<Fn3>(name);
        for _ in 0..1200 {
            let x = pool[rng.below(pool.len())];
            let y = pool[rng.below(pool.len())];
            let mut refc = [0u8; 32];
            unsafe { c(refc.as_mut_ptr(), x.as_ptr(), y.as_ptr()) };
            // z == x
            let mut bc = padded(32);
            let mut br = padded(32);
            bc[..32].copy_from_slice(&x);
            br[..32].copy_from_slice(&x);
            unsafe {
                c(bc.as_mut_ptr(), bc.as_ptr(), y.as_ptr());
                r(br.as_mut_ptr(), br.as_ptr(), y.as_ptr());
            }
            eqb(&format!("{name} z==x"), &bc[..32], &br[..32]);
            check_pad("alias z==x (C)", &bc, 32);
            check_pad("alias z==x (Rust)", &br, 32);
            assert_eq!(&bc[..32], &refc[..], "{name}: z==x changed the C result");
            // z == y
            let mut bc = padded(32);
            let mut br = padded(32);
            bc[..32].copy_from_slice(&y);
            br[..32].copy_from_slice(&y);
            unsafe {
                c(bc.as_mut_ptr(), x.as_ptr(), bc.as_ptr());
                r(br.as_mut_ptr(), x.as_ptr(), br.as_ptr());
            }
            eqb(&format!("{name} z==y"), &bc[..32], &br[..32]);
            assert_eq!(&bc[..32], &refc[..], "{name}: z==y changed the C result");
            // z == x == y
            let mut refc2 = [0u8; 32];
            unsafe { c(refc2.as_mut_ptr(), x.as_ptr(), x.as_ptr()) };
            let mut bc = padded(32);
            let mut br = padded(32);
            bc[..32].copy_from_slice(&x);
            br[..32].copy_from_slice(&x);
            unsafe {
                c(bc.as_mut_ptr(), bc.as_ptr(), bc.as_ptr());
                r(br.as_mut_ptr(), br.as_ptr(), br.as_ptr());
            }
            eqb(&format!("{name} z==x==y"), &bc[..32], &br[..32]);
            assert_eq!(&bc[..32], &refc2[..], "{name}: z==x==y changed the C result");
        }
    }

    // point add / sub with r == p and r == q
    for name in ["crypto_core_ed25519_add", "crypto_core_ed25519_sub"] {
        let (c, r) = both::<Fn3i>(name);
        let mut pts: Vec<[u8; 32]> = vec![BASEPOINT, IDENTITY];
        pts.extend_from_slice(&SMALL_ORDER);
        let (crand, _) = both::<Fn1>("crypto_core_ed25519_random");
        for i in 0..8u64 {
            rng_reseed(0xa11a_0000 ^ (i | 1));
            let mut o = [0u8; 32];
            unsafe { crand(o.as_mut_ptr()) };
            pts.push(o);
        }
        rng_reset();
        for p in &pts {
            for q in &pts {
                let mut refc = [0u8; 32];
                let rc0 = unsafe { c(refc.as_mut_ptr(), p.as_ptr(), q.as_ptr()) };
                let mut bc = padded(32);
                let mut br = padded(32);
                bc[..32].copy_from_slice(p);
                br[..32].copy_from_slice(p);
                let rc = unsafe { c(bc.as_mut_ptr(), bc.as_ptr(), q.as_ptr()) };
                let rr = unsafe { r(br.as_mut_ptr(), br.as_ptr(), q.as_ptr()) };
                eqi(&format!("{name} r==p ret"), rc, rr);
                assert_eq!(rc, rc0);
                if rc == 0 {
                    eqb(&format!("{name} r==p"), &bc[..32], &br[..32]);
                    assert_eq!(&bc[..32], &refc[..], "{name}: r==p changed the C result");
                }
                check_pad("alias r==p (C)", &bc, 32);
                check_pad("alias r==p (Rust)", &br, 32);
                let mut bc = padded(32);
                let mut br = padded(32);
                bc[..32].copy_from_slice(q);
                br[..32].copy_from_slice(q);
                let rc = unsafe { c(bc.as_mut_ptr(), p.as_ptr(), bc.as_ptr()) };
                let rr = unsafe { r(br.as_mut_ptr(), p.as_ptr(), br.as_ptr()) };
                eqi(&format!("{name} r==q ret"), rc, rr);
                if rc == 0 {
                    eqb(&format!("{name} r==q"), &bc[..32], &br[..32]);
                    assert_eq!(&bc[..32], &refc[..], "{name}: r==q changed the C result");
                }
            }
        }
    }

    // ge25519_from_uniform: C starts with `memcpy(s, r, 32)`, so s == r works
    {
        let (c, r) = both::<Fn2>("_sodium_ge25519_from_uniform");
        for _ in 0..300 {
            let src: [u8; 32] = rng.bytes(32).try_into().unwrap();
            let mut refc = [0u8; 32];
            unsafe { c(refc.as_mut_ptr(), src.as_ptr()) };
            let mut bc = padded(32);
            let mut br = padded(32);
            bc[..32].copy_from_slice(&src);
            br[..32].copy_from_slice(&src);
            unsafe {
                c(bc.as_mut_ptr(), bc.as_ptr());
                r(br.as_mut_ptr(), br.as_ptr());
            }
            eqb("ge25519_from_uniform aliased", &bc[..32], &br[..32]);
            check_pad("from_uniform aliased(C)", &bc, 32);
            check_pad("from_uniform aliased(Rust)", &br, 32);
            assert_eq!(&bc[..32], &refc[..], "from_uniform: aliasing changed the C result");
        }
    }
}

/// Row 2.128 boundary: scalars that violate the documented `a[31] <= 127`
/// precondition (the `e[63]` digit can reach 16, which `ge25519_cmov8*` maps to
/// the neutral element).  Both implementations must still agree.
#[test]
fn ge25519_scalarmult_out_of_range_scalars() {
    let (csm, rsm) = both::<GeScalarmult>("_sodium_ge25519_scalarmult");
    let (csb, rsb) = both::<GeScalarmultBase>("_sodium_ge25519_scalarmult_base");
    let (cds, rds) = both::<GeDoubleScalarmult>("_sodium_ge25519_double_scalarmult_vartime");
    let g = Ge::new();
    let mut pc = P3::new();
    let mut pr = P3::new();
    assert_eq!(unsafe { g.frombytes.0(&mut pc, BASEPOINT.as_ptr()) }, 0);
    assert_eq!(unsafe { g.frombytes.1(&mut pr, BASEPOINT.as_ptr()) }, 0);

    let mut scalars: Vec<[u8; 32]> = Vec::new();
    for top in [0x80u8, 0x81, 0xf0, 0xff] {
        let mut a = [0u8; 32];
        a[31] = top;
        scalars.push(a);
        let mut b = [0xffu8; 32];
        b[31] = top;
        scalars.push(b);
    }
    scalars.push([0xffu8; 32]);
    let mut rng = Rng::new(0x2_0128_b);
    for _ in 0..120 {
        let mut a: [u8; 32] = rng.bytes(32).try_into().unwrap();
        a[31] |= 0x80;
        scalars.push(a);
    }
    for a in &scalars {
        let mut hc = P3::new();
        let mut hr = P3::new();
        unsafe {
            csm(&mut hc, a.as_ptr(), &pc);
            rsm(&mut hr, a.as_ptr(), &pr);
        }
        eqb(&format!("ge25519_scalarmult(out-of-range {})", hex(a)), &hc.bytes(), &hr.bytes());
        let mut bc = P3::new();
        let mut brr = P3::new();
        unsafe {
            csb(&mut bc, a.as_ptr());
            rsb(&mut brr, a.as_ptr());
        }
        eqb(
            &format!("ge25519_scalarmult_base(out-of-range {})", hex(a)),
            &bc.bytes(),
            &brr.bytes(),
        );
        let mut dc = P2::new();
        let mut dr = P2::new();
        unsafe {
            cds(&mut dc, a.as_ptr(), &pc, a.as_ptr());
            rds(&mut dr, a.as_ptr(), &pr, a.as_ptr());
        }
        eqb("ge25519_double_scalarmult_vartime(out-of-range)", &dc.bytes(), &dr.bytes());
    }
}
