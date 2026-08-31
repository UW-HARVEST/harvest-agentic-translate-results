//! Area 2, part 5 — gap closure for the `configs_2.md` / `errors_2.md` rows that
//! ask for *both arms* of a branch inside a **static** (non-exported) helper of
//! `crypto_core/ed25519/ref10/ed25519_ref10.c`:
//!
//! * config 2.94 — `ge25519_from_uniform`: the `x_sign` conditional negation.
//! * config 2.95 — `ge25519_elligator2`: `fe25519_notsquare(gx1) == 0` and `== 1`.
//! * config 2.96 — `ge25519_mont_to_ed`: the `fe25519_iszero(x_plus_one_y_inv)` cmov.
//! * config 2.103 — `fe25519_reduce64`: the `h[31]` / `h[63]` high-bit contributions.
//! * config 2.113 — `ristretto255_elligator`: `sqrt_ratio_m1` returning 1 and 0.
//! * config 2.114 — `ristretto255_p3_tobytes`: `rotate` 0 and 1, and the
//!   `fe25519_isnegative(x_z_inv)` conditional negation of `y_`.
//! * error 2.12 / 2.14 / 2.15 / 2.16 / 2.17 — the four *independent* rejection
//!   arms that `ristretto255_frombytes` folds into a single `-1`.
//!
//! None of those helpers is exported, and every one of the rejections above is
//! observationally indistinguishable from the others through the public API.  So
//! this file carries a **test-side** reimplementation of `F_p` (`p = 2^255-19`)
//! and of the exact statement sequence of each helper, uses it to *classify*
//! inputs, and asserts that every arm is actually taken by the inputs that are
//! then fed differentially to both libraries.
//!
//! The classifier is not trusted blindly: every replica is validated end-to-end
//! against the real library through exported entry points
//! (`_sodium_fe25519_invert`, `_sodium_ge25519_from_uniform`,
//! `_sodium_ge25519_from_hash`, `_sodium_ristretto255_frombytes`,
//! `_sodium_ristretto255_p3_tobytes`, `_sodium_ge25519_p3_add`,
//! `_sodium_ge25519_clear_cofactor`, `crypto_core_ristretto255_from_hash`), so a
//! bug in the classifier makes this file fail rather than silently mis-classify.
#![allow(dead_code)]
mod common;
use common::*;
use std::ffi::c_int;

// ===========================================================================
// F_p arithmetic, p = 2^255 - 19.  Test-side only: never linked against either
// library, used purely to predict which branch the C code will take.
// ===========================================================================

mod fp {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Fe(pub [u64; 4]);

    pub const P: [u64; 4] = [
        0xffff_ffff_ffff_ffed,
        0xffff_ffff_ffff_ffff,
        0xffff_ffff_ffff_ffff,
        0x7fff_ffff_ffff_ffff,
    ];

    // sqrt(-1)
    pub const SQRTM1: Fe = Fe([0xc4ee1b274a0ea0b0, 0x2f431806ad2fe478, 0x2b4d00993dfbd7a7, 0x2b8324804fc1df0b]);
    // sqrt(-486664)
    pub const SQRTAM2: Fe = Fe([0xcc6e04aaff457e06, 0xc5a1d3d14b7d1a82, 0xd27b08dc03fc4f7e, 0x0f26edf460a006bb]);
    // the Edwards `d`
    pub const D: Fe = Fe([0x75eb4dca135978a3, 0x00700a4d4141d8ab, 0x8cc740797779e898, 0x52036cee2b6ffe73]);
    // sqrt(a*d - 1), a = -1
    pub const SQRTADM1: Fe = Fe([0x7e97f6a0497b2e1b, 0xaf9d8e0c1b7854bd, 0x0f3cfcc931f5d1fd, 0x376931bf2b8348ac]);
    // 1 / sqrt(a - d)
    pub const INVSQRTAMD: Fe = Fe([0x99c8fdaa805d40ea, 0x9d2f16175a4172be, 0x16c27b91fe01d840, 0x786c8905cfaffca2]);
    // 1 - d^2
    pub const ONEMSQD: Fe = Fe([0xe27c09c1945fc176, 0x2c81a138cd5e350f, 0x9994abddbe70dfe4, 0x029072a8b2b3e0d7]);
    // (d - 1)^2
    pub const SQDMONE: Fe = Fe([0x31ad5aaa44ed4d20, 0xd29e4a2cb01e1999, 0x4cdcd32f529b4eeb, 0x5968b37af66c2241]);

    /// `A = 486662`, the Montgomery coefficient of Curve25519.
    pub const A_32: u64 = 486662;

    const E_INV: [u64; 4] = [0xffffffffffffffeb, 0xffffffffffffffff, 0xffffffffffffffff, 0x7fffffffffffffff];
    const E_22523: [u64; 4] = [0xfffffffffffffffd, 0xffffffffffffffff, 0xffffffffffffffff, 0x0fffffffffffffff];
    const E_HALF: [u64; 4] = [0xfffffffffffffff6, 0xffffffffffffffff, 0xffffffffffffffff, 0x3fffffffffffffff];

    fn ge(a: &[u64; 4], b: &[u64; 4]) -> bool {
        for i in (0..4).rev() {
            if a[i] != b[i] {
                return a[i] > b[i];
            }
        }
        true
    }

    fn sub_in_place(a: &mut [u64; 4], b: &[u64; 4]) {
        let mut borrow = 0u128;
        for i in 0..4 {
            let v = (a[i] as u128).wrapping_sub(b[i] as u128).wrapping_sub(borrow);
            a[i] = v as u64;
            borrow = (v >> 64) & 1;
        }
    }

    fn canon(mut r: [u64; 4]) -> Fe {
        for _ in 0..3 {
            if ge(&r, &P) {
                let p = P;
                sub_in_place(&mut r, &p);
            }
        }
        Fe(r)
    }

    fn reduce_wide(mut t: [u64; 8]) -> Fe {
        while t[4] != 0 || t[5] != 0 || t[6] != 0 || t[7] != 0 {
            let hi = [t[4], t[5], t[6], t[7]];
            let mut acc = [0u64; 8];
            // acc = 38 * hi   (2^256 == 38 mod p)
            let mut carry = 0u128;
            for i in 0..4 {
                let v = 38u128 * hi[i] as u128 + carry;
                acc[i] = v as u64;
                carry = v >> 64;
            }
            acc[4] = carry as u64;
            // acc += low half
            let mut c = 0u128;
            for i in 0..5 {
                let lo = if i < 4 { t[i] as u128 } else { 0 };
                let v = acc[i] as u128 + lo + c;
                acc[i] = v as u64;
                c = v >> 64;
            }
            acc[5] = acc[5].wrapping_add(c as u64);
            t = acc;
        }
        canon([t[0], t[1], t[2], t[3]])
    }

    impl Fe {
        pub fn zero() -> Fe {
            Fe([0; 4])
        }
        pub fn one() -> Fe {
            Fe([1, 0, 0, 0])
        }
        pub fn from_u64(v: u64) -> Fe {
            Fe([v, 0, 0, 0])
        }
        pub fn is_zero(self) -> bool {
            self.0 == [0u64; 4]
        }
        /// `fe25519_frombytes`: 32 little-endian bytes, bit 255 ignored.
        pub fn from_bytes(b: &[u8]) -> Fe {
            assert_eq!(b.len(), 32);
            let mut x = [0u8; 32];
            x.copy_from_slice(b);
            x[31] &= 0x7f;
            let mut l = [0u64; 4];
            for i in 0..4 {
                l[i] = u64::from_le_bytes(x[i * 8..i * 8 + 8].try_into().unwrap());
            }
            canon(l)
        }
        /// `fe25519_tobytes`: the fully reduced little-endian encoding.
        pub fn to_bytes(self) -> [u8; 32] {
            let mut o = [0u8; 32];
            for i in 0..4 {
                o[i * 8..i * 8 + 8].copy_from_slice(&self.0[i].to_le_bytes());
            }
            o
        }
        /// `fe25519_isnegative`.
        pub fn is_negative(self) -> u32 {
            (self.to_bytes()[0] & 1) as u32
        }
    }

    pub fn add(a: Fe, b: Fe) -> Fe {
        let mut r = [0u64; 4];
        let mut c = 0u128;
        for i in 0..4 {
            let v = a.0[i] as u128 + b.0[i] as u128 + c;
            r[i] = v as u64;
            c = v >> 64;
        }
        assert_eq!(c, 0, "fp::add overflowed 256 bits");
        canon(r)
    }

    pub fn sub(a: Fe, b: Fe) -> Fe {
        // a + (p - b); both operands are < p, so a + (p - b) < 2p.
        let mut nb = P;
        sub_in_place(&mut nb, &b.0);
        add(a, Fe(nb))
    }

    pub fn neg(a: Fe) -> Fe {
        sub(Fe::zero(), a)
    }

    pub fn mul(a: Fe, b: Fe) -> Fe {
        let mut t = [0u64; 8];
        for i in 0..4 {
            let mut carry = 0u128;
            for j in 0..4 {
                let cur = t[i + j] as u128 + a.0[i] as u128 * b.0[j] as u128 + carry;
                t[i + j] = cur as u64;
                carry = cur >> 64;
            }
            let mut k = i + 4;
            while carry != 0 {
                let cur = t[k] as u128 + carry;
                t[k] = cur as u64;
                carry = cur >> 64;
                k += 1;
            }
        }
        reduce_wide(t)
    }

    pub fn sq(a: Fe) -> Fe {
        mul(a, a)
    }

    /// `fe25519_mul32` / `fe25519_sq2` style small-scalar multiply.
    pub fn mul_small(a: Fe, k: u64) -> Fe {
        mul(a, Fe::from_u64(k))
    }

    fn pow(a: Fe, e: &[u64; 4]) -> Fe {
        let mut r = Fe::one();
        for i in (0..4).rev() {
            for j in (0..64).rev() {
                r = sq(r);
                if (e[i] >> j) & 1 == 1 {
                    r = mul(r, a);
                }
            }
        }
        r
    }

    /// `fe25519_invert`; `invert(0) == 0`, exactly like the C version.
    pub fn invert(a: Fe) -> Fe {
        pow(a, &E_INV)
    }

    /// `fe25519_pow22523`, i.e. `a^((p-5)/8)`.
    pub fn pow22523(a: Fe) -> Fe {
        pow(a, &E_22523)
    }

    pub fn cmov(a: Fe, b: Fe, c: u32) -> Fe {
        if c != 0 {
            b
        } else {
            a
        }
    }

    /// `fe25519_abs`.
    pub fn abs(a: Fe) -> Fe {
        cmov(a, neg(a), a.is_negative())
    }

    /// `fe25519_notsquare`: the Jacobi symbol, returned as `s[1] & 1` of
    /// `x^((p-1)/2)`.  That is `1` exactly for quadratic non-residues, and `0`
    /// for squares *and* for zero.
    pub fn notsquare(x: Fe) -> u32 {
        let t = pow(x, &E_HALF);
        (t.to_bytes()[1] & 1) as u32
    }

    /// `fe25519_unchecked_sqrt` followed by the `fe25519_sqrt` check; returns
    /// `(root, ok)` where `ok == true` iff the C function returned `0`.
    pub fn sqrt(x2: Fe) -> (Fe, bool) {
        let e = pow22523(x2);
        let p_root = mul(e, x2);
        let m_root = mul(p_root, SQRTM1);
        let m_root2 = sq(m_root);
        let e2 = sub(x2, m_root2);
        let x = cmov(p_root, m_root, e2.is_zero() as u32);
        let check = sub(sq(x), x2);
        (x, check.is_zero())
    }
}

use fp::Fe;

// ===========================================================================
// `fe25519` <-> `fe_25_5` limb conversion (10 limbs, offsets 0,26,51,...,230)
// ===========================================================================

const OFF: [u32; 10] = [0, 26, 51, 77, 102, 128, 153, 179, 204, 230];
const WID: [u32; 10] = [26, 25, 26, 25, 26, 25, 26, 25, 26, 25];

fn pow2(k: u32) -> Fe {
    let mut l = [0u64; 4];
    l[(k / 64) as usize] = 1u64 << (k % 64);
    Fe(l)
}

/// Interpret ten signed `fe_25_5` limbs as a field element.
fn limbs_to_fe(l: &[i32]) -> Fe {
    let mut acc = Fe::zero();
    for i in 0..10 {
        let v = l[i] as i64;
        let t = fp::mul(Fe::from_u64(v.unsigned_abs()), pow2(OFF[i]));
        acc = if v >= 0 { fp::add(acc, t) } else { fp::sub(acc, t) };
    }
    acc
}

/// Encode a field element as ten `fe_25_5` limbs.
///
/// The limbs are put into the *balanced* form that `fe25519_reduce` (and hence
/// every `fe25519_mul` / `_sq` / `_add` output) produces, i.e. `|l[even]| <= 2^25`
/// and `|l[odd]| <= 2^24`.  The plain unsigned 26/25-bit split that
/// `fe25519_frombytes` emits is *twice* as large, which violates the documented
/// input bounds of `fe25519_sq` once the value has been through one
/// `fe25519_add` — e.g. inside `ge25519_p2_dbl` — and then silently produces
/// wrong results.  Encoding must therefore match what the C code's own
/// arithmetic hands to those routines, not what `frombytes` hands to them.
fn fe_to_limbs(x: Fe) -> [i32; 10] {
    let b = x.to_bytes();
    let bit = |i: u32| -> i64 { ((b[(i / 8) as usize] >> (i % 8)) & 1) as i64 };
    let mut l = [0i64; 10];
    for i in 0..10 {
        let mut v: i64 = 0;
        for j in 0..WID[i] {
            v |= bit(OFF[i] + j) << j;
        }
        l[i] = v;
    }
    // Two balanced-carry passes.  A carry out of limb 9 wraps into limb 0 with
    // the factor 19, because `2^255 == 19 (mod p)`; the value is preserved.
    for _ in 0..2 {
        for i in 0..10 {
            let w = WID[i];
            let half = 1i64 << (w - 1);
            let carry = if l[i] >= half {
                l[i] -= 1i64 << w;
                1
            } else if l[i] < -half {
                l[i] += 1i64 << w;
                -1
            } else {
                0
            };
            if carry != 0 {
                if i == 9 {
                    l[0] += 19 * carry;
                } else {
                    l[i + 1] += carry;
                }
            }
        }
    }
    let mut out = [0i32; 10];
    for i in 0..10 {
        out[i] = l[i] as i32;
    }
    out
}

// ===========================================================================
// FFI
// ===========================================================================

/// `ge25519_p3` = 4 x `fe25519` = 4 x `int32_t[10]`.
#[repr(C)]
#[derive(Copy, Clone)]
struct P3([i32; 40]);

impl P3 {
    fn new() -> P3 {
        P3([0x5A5A_5A5A; 40])
    }
    fn from_fe(x: Fe, y: Fe, z: Fe, t: Fe) -> P3 {
        let mut o = [0i32; 40];
        for (i, f) in [x, y, z, t].into_iter().enumerate() {
            o[i * 10..i * 10 + 10].copy_from_slice(&fe_to_limbs(f));
        }
        P3(o)
    }
    fn get(&self, which: usize) -> Fe {
        limbs_to_fe(&self.0[which * 10..which * 10 + 10])
    }
    fn x(&self) -> Fe {
        self.get(0)
    }
    fn y(&self) -> Fe {
        self.get(1)
    }
    fn z(&self) -> Fe {
        self.get(2)
    }
    fn t(&self) -> Fe {
        self.get(3)
    }
    fn bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

type Fn2 = unsafe extern "C" fn(*mut u8, *const u8);
type Fn2i = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type FeFromBytes = unsafe extern "C" fn(*mut i32, *const u8);
type FeToBytes = unsafe extern "C" fn(*mut u8, *const i32);
type FeInvert = unsafe extern "C" fn(*mut i32, *const i32);
type GeFromBytes = unsafe extern "C" fn(*mut P3, *const u8) -> c_int;
type GeToBytes3 = unsafe extern "C" fn(*mut u8, *const P3);
type GeUnary = unsafe extern "C" fn(*mut P3);
type GeP3Op = unsafe extern "C" fn(*mut P3, *const P3, *const P3);
type Pred = unsafe extern "C" fn(*const u8) -> c_int;

// ===========================================================================
// Test-side replicas of the static helpers
// ===========================================================================

/// `ge25519_elligator2`.  Returns `(x, y, notsquare)`.
fn elligator2(r: Fe) -> (Fe, Fe, u32) {
    let mut rr2 = fp::mul_small(fp::sq(r), 2); // fe25519_sq2
    rr2 = fp::add(rr2, Fe::one()); // rr2[0]++
    rr2 = fp::invert(rr2);
    let mut x = fp::neg(fp::mul_small(rr2, fp::A_32)); // x = x1
    let x2 = fp::sq(x);
    let x3 = fp::mul(x, x2);
    let ax2 = fp::mul_small(x2, fp::A_32);
    let gx1 = fp::add(fp::add(x3, x), ax2);

    let notsquare = fp::notsquare(gx1);

    let negx = fp::neg(x);
    x = fp::cmov(x, negx, notsquare);
    let sub_a = fp::cmov(Fe::zero(), Fe::from_u64(fp::A_32), notsquare);
    x = fp::sub(x, sub_a);

    // ge25519_xmont_to_ymont
    let xx = fp::sq(x);
    let xxx = fp::mul(x, xx);
    let axx = fp::mul_small(xx, fp::A_32);
    let y2 = fp::add(fp::add(xxx, x), axx);
    let (y, ok) = fp::sqrt(y2);
    assert!(ok, "ge25519_xmont_to_ymont would abort() — mathematically impossible");
    (x, y, notsquare)
}

/// `ge25519_mont_to_ed`.  Returns `(xed, yed, cmov_fired)` where `cmov_fired`
/// is the `fe25519_iszero(x_plus_one_y_inv)` flag of `configs_2.md` row 2.96.
fn mont_to_ed(x: Fe, y: Fe) -> (Fe, Fe, u32) {
    let one = Fe::one();
    let x_plus_one = fp::add(x, one);
    let x_minus_one = fp::sub(x, one);
    let xpyi = fp::invert(fp::mul(x_plus_one, y));
    let mut xed = fp::mul(x, fp::SQRTAM2);
    xed = fp::mul(xed, xpyi);
    xed = fp::mul(xed, x_plus_one);
    let mut yed = fp::mul(xpyi, y);
    yed = fp::mul(yed, x_minus_one);
    let fired = xpyi.is_zero() as u32;
    yed = fp::cmov(yed, one, fired);
    (xed, yed, fired)
}

/// `ge25519_from_uniform`, up to (but excluding) `ge25519_clear_cofactor`.
/// Returns the pre-cofactor-clearing `p3` plus the branch flags.
struct UniformTrace {
    p3: P3,
    notsquare: u32,
    x_sign: u32,
    mont_cmov: u32,
    x_negated: u32,
}

fn from_uniform_trace(r: &[u8; 32]) -> UniformTrace {
    let x_sign = ((r[31] >> 5) >> 2) as u32; // == r[31] >> 7
    let mut s = *r;
    s[31] &= 0x7f;
    let r_fe = Fe::from_bytes(&s);
    let (x, y, notsquare) = elligator2(r_fe);
    let (xed, yed, mont_cmov) = mont_to_ed(x, y);
    let negxed = fp::neg(xed);
    let cond = xed.is_negative() ^ x_sign;
    let bx = fp::cmov(xed, negxed, cond);
    let p3 = P3::from_fe(bx, yed, Fe::one(), fp::mul(bx, yed));
    UniformTrace { p3, notsquare, x_sign, mont_cmov, x_negated: cond }
}

/// `fe25519_reduce64`.
fn reduce64(h: &[u8; 64]) -> Fe {
    let mut fl = [0u8; 32];
    let mut gl = [0u8; 32];
    fl.copy_from_slice(&h[..32]);
    gl.copy_from_slice(&h[32..]);
    fl[31] &= 0x7f;
    gl[31] &= 0x7f;
    let f = Fe::from_bytes(&fl);
    let g = Fe::from_bytes(&gl);
    let bump = ((h[31] >> 5) >> 2) as u64 * 19 + ((h[63] >> 5) >> 2) as u64 * 722;
    fp::add(fp::add(f, Fe::from_u64(bump)), fp::mul_small(g, 38))
}

struct HashTrace {
    p3: P3,
    notsquare: u32,
    mont_cmov: u32,
    y_negated: u32,
}

/// `ge25519_from_hash`, up to (but excluding) `ge25519_clear_cofactor`.
fn from_hash_trace(h: &[u8; 64]) -> HashTrace {
    let fe_f = reduce64(h);
    let (x, y, notsquare) = elligator2(fe_f);
    let y_sign = notsquare ^ 1;
    let negy = fp::neg(y);
    let cond = y.is_negative() ^ y_sign;
    let y = fp::cmov(y, negy, cond);
    let (xed, yed, mont_cmov) = mont_to_ed(x, y);
    let p3 = P3::from_fe(xed, yed, Fe::one(), fp::mul(xed, yed));
    HashTrace { p3, notsquare, mont_cmov, y_negated: cond }
}

/// `ristretto255_sqrt_ratio_m1`.  Returns `(x, ret)`.
fn sqrt_ratio_m1(u: Fe, v: Fe) -> (Fe, u32) {
    let mut v3 = fp::sq(v);
    v3 = fp::mul(v3, v);
    let mut x = fp::sq(v3);
    x = fp::mul(x, u);
    x = fp::mul(x, v);
    x = fp::pow22523(x);
    x = fp::mul(x, v3);
    x = fp::mul(x, u);
    let mut vxx = fp::sq(x);
    vxx = fp::mul(vxx, v);
    let m_root_check = fp::sub(vxx, u);
    let p_root_check = fp::add(vxx, u);
    let f_root_check = fp::add(vxx, fp::mul(u, fp::SQRTM1));
    let has_m = m_root_check.is_zero() as u32;
    let has_p = p_root_check.is_zero() as u32;
    let has_f = f_root_check.is_zero() as u32;
    let x_sqrtm1 = fp::mul(x, fp::SQRTM1);
    x = fp::cmov(x, x_sqrtm1, has_p | has_f);
    x = fp::abs(x);
    (x, has_m | has_p)
}

/// `ristretto255_is_canonical`.
fn rist_is_canonical(s: &[u8; 32]) -> u32 {
    let mut c: u8 = (s[31] & 0x7f) ^ 0x7f;
    for i in (1..=30).rev() {
        c |= s[i] ^ 0xff;
    }
    let c = ((c as u32).wrapping_sub(1) >> 8) as u8;
    let d = ((0xedu32).wrapping_sub(1).wrapping_sub(s[0] as u32) >> 8) as u8;
    let e = (s[31] >> 5) >> 2;
    1 - ((((c & d) | e | s[0]) & 1) as u32)
}

/// `ristretto255_frombytes`, with the four rejection reasons kept separate.
#[derive(Debug, Default, Clone, Copy)]
struct RistDecode {
    ret: c_int,
    noncanonical: bool,
    notsquare: bool,
    t_negative: bool,
    y_zero: bool,
}

fn rist_frombytes(s: &[u8; 32]) -> (Option<P3>, RistDecode) {
    let mut d = RistDecode::default();
    if rist_is_canonical(s) == 0 {
        d.ret = -1;
        d.noncanonical = true;
        return (None, d);
    }
    let s_ = Fe::from_bytes(s);
    let ss = fp::sq(s_);
    let u1 = fp::sub(Fe::one(), ss);
    let u1u1 = fp::sq(u1);
    let u2 = fp::add(Fe::one(), ss);
    let u2u2 = fp::sq(u2);
    let mut v = fp::neg(fp::mul(fp::D, u1u1));
    v = fp::sub(v, u2u2);
    let v_u2u2 = fp::mul(v, u2u2);
    let (inv_sqrt, notsquare) = sqrt_ratio_m1(Fe::one(), v_u2u2);
    let mut px = fp::mul(inv_sqrt, u2);
    let mut py = fp::mul(fp::mul(inv_sqrt, px), v);
    px = fp::mul(px, s_);
    px = fp::add(px, px);
    px = fp::abs(px);
    py = fp::mul(u1, py);
    let pz = Fe::one();
    let pt = fp::mul(px, py);
    d.notsquare = notsquare == 0;
    d.t_negative = pt.is_negative() != 0;
    d.y_zero = py.is_zero();
    d.ret = if d.notsquare || d.t_negative || d.y_zero { -1 } else { 0 };
    (Some(P3::from_fe(px, py, pz, pt)), d)
}

/// `ristretto255_p3_tobytes`, exposing the `rotate` and `isnegative(x_z_inv)`
/// branch flags of `configs_2.md` row 2.114.
fn rist_p3_tobytes(x: Fe, y: Fe, z: Fe, t: Fe) -> ([u8; 32], u32, u32) {
    let mut u1 = fp::add(z, y);
    let zmy = fp::sub(z, y);
    u1 = fp::mul(u1, zmy);
    let u2 = fp::mul(x, y);
    let mut u1_u2u2 = fp::sq(u2);
    u1_u2u2 = fp::mul(u1, u1_u2u2);
    let (inv_sqrt, _) = sqrt_ratio_m1(Fe::one(), u1_u2u2);
    let den1 = fp::mul(inv_sqrt, u1);
    let den2 = fp::mul(inv_sqrt, u2);
    let mut z_inv = fp::mul(den1, den2);
    z_inv = fp::mul(z_inv, t);
    let ix = fp::mul(x, fp::SQRTM1);
    let iy = fp::mul(y, fp::SQRTM1);
    let eden = fp::mul(den1, fp::INVSQRTAMD);
    let t_z_inv = fp::mul(t, z_inv);
    let rotate = t_z_inv.is_negative();
    let x_ = fp::cmov(x, iy, rotate);
    let mut y_ = fp::cmov(y, ix, rotate);
    let den_inv = fp::cmov(den2, eden, rotate);
    let x_z_inv = fp::mul(x_, z_inv);
    let xzneg = x_z_inv.is_negative();
    y_ = fp::cmov(y_, fp::neg(y_), xzneg);
    let mut s_ = fp::sub(z, y_);
    s_ = fp::mul(den_inv, s_);
    s_ = fp::abs(s_);
    (s_.to_bytes(), rotate, xzneg)
}

/// `ristretto255_elligator`.  Returns the projective point and `wasnt_square`.
fn rist_elligator(t: Fe) -> (P3, u32) {
    let one = Fe::one();
    let mut r = fp::sq(t);
    r = fp::mul(fp::SQRTM1, r);
    let mut u = fp::add(r, one);
    u = fp::mul(u, fp::ONEMSQD);
    let mut c = fp::neg(one);
    let rpd = fp::add(r, fp::D);
    let mut v = fp::mul(r, fp::D);
    v = fp::sub(c, v);
    v = fp::mul(v, rpd);

    let (s0, was_square) = sqrt_ratio_m1(u, v);
    let wasnt_square = 1 - was_square;
    let mut s = s0;
    let mut s_prime = fp::mul(s, t);
    s_prime = fp::abs(s_prime);
    s_prime = fp::neg(s_prime);
    s = fp::cmov(s, s_prime, wasnt_square);
    c = fp::cmov(c, r, wasnt_square);

    let mut n = fp::sub(r, one);
    n = fp::mul(n, c);
    n = fp::mul(n, fp::SQDMONE);
    n = fp::sub(n, v);

    let mut w0 = fp::add(s, s);
    w0 = fp::mul(w0, v);
    let w1 = fp::mul(n, fp::SQRTADM1);
    let ss = fp::sq(s);
    let w2 = fp::sub(one, ss);
    let w3 = fp::add(one, ss);

    (
        P3::from_fe(
            fp::mul(w0, w3),
            fp::mul(w2, w1),
            fp::mul(w1, w3),
            fp::mul(w0, w2),
        ),
        wasnt_square,
    )
}

// ===========================================================================
// 0. The replica's own arithmetic, checked against the library
// ===========================================================================

/// Before any classification is trusted, pin `fp` down against the three
/// exported `fe25519_*` primitives and against the constant definitions.
#[test]
fn fp_replica_matches_library() {
    // constant sanity, from their mathematical definitions
    assert_eq!(fp::sq(fp::SQRTM1), fp::neg(Fe::one()), "sqrtm1^2 != -1");
    assert_eq!(
        fp::sq(fp::SQRTAM2),
        fp::neg(Fe::from_u64(486664)),
        "sqrtam2^2 != -486664"
    );
    assert_eq!(
        fp::D,
        fp::mul(fp::neg(Fe::from_u64(121665)), fp::invert(Fe::from_u64(121666))),
        "d != -121665/121666"
    );
    assert_eq!(fp::ONEMSQD, fp::sub(Fe::one(), fp::sq(fp::D)), "onemsqd != 1-d^2");
    assert_eq!(
        fp::SQDMONE,
        fp::sq(fp::sub(fp::D, Fe::one())),
        "sqdmone != (d-1)^2"
    );
    assert_eq!(
        fp::sq(fp::SQRTADM1),
        fp::sub(fp::neg(fp::D), Fe::one()),
        "sqrtadm1^2 != a*d-1"
    );
    assert_eq!(
        fp::mul(fp::sq(fp::INVSQRTAMD), fp::sub(fp::neg(Fe::one()), fp::D)),
        Fe::one(),
        "invsqrtamd^2 * (a-d) != 1"
    );

    let (cfb, _) = both::<FeFromBytes>("_sodium_fe25519_frombytes");
    let (ctb, _) = both::<FeToBytes>("_sodium_fe25519_tobytes");
    let (civ, _) = both::<FeInvert>("_sodium_fe25519_invert");

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
    let mut rng = Rng::new(0x2_0f00);
    for _ in 0..400 {
        inputs.push(rng.bytes(32).try_into().unwrap());
    }

    for s in &inputs {
        let mine = Fe::from_bytes(s);
        // frombytes + tobytes must reproduce the same canonical bytes
        let mut limbs = [0i32; 10];
        let mut out = [0u8; 32];
        unsafe {
            cfb(limbs.as_mut_ptr(), s.as_ptr());
            ctb(out.as_mut_ptr(), limbs.as_ptr());
        }
        assert_eq!(out, mine.to_bytes(), "fp::from_bytes/to_bytes != library");
        // the limb decoder must agree with the library's own limbs
        assert_eq!(limbs_to_fe(&limbs), mine, "limbs_to_fe != library limbs");
        // the limb *encoder* must round-trip through the library
        let enc = fe_to_limbs(mine);
        let mut out2 = [0u8; 32];
        unsafe { ctb(out2.as_mut_ptr(), enc.as_ptr()) };
        assert_eq!(out2, mine.to_bytes(), "fe_to_limbs != library");
        // inversion
        let mut iv = [0i32; 10];
        let mut ivb = [0u8; 32];
        unsafe {
            civ(iv.as_mut_ptr(), limbs.as_ptr());
            ctb(ivb.as_mut_ptr(), iv.as_ptr());
        }
        assert_eq!(ivb, fp::invert(mine).to_bytes(), "fp::invert != library");
        // and a couple of algebraic identities that pin down mul/sq/add/sub
        if !mine.is_zero() {
            assert_eq!(fp::mul(mine, fp::invert(mine)), Fe::one());
        }
        assert_eq!(fp::sq(mine), fp::mul(mine, mine));
        assert_eq!(fp::sub(fp::add(mine, fp::D), fp::D), mine);
        // notsquare must agree with "has a square root"
        let (root, ok) = fp::sqrt(mine);
        assert_eq!(
            ok,
            fp::notsquare(mine) == 0,
            "fp::sqrt and fp::notsquare disagree on {}",
            hex(s)
        );
        if ok {
            assert_eq!(fp::sq(root), mine);
        }
    }
}

// ===========================================================================
// 1. ge25519_from_uniform / ge25519_elligator2 / ge25519_mont_to_ed
//    configs_2.md rows 2.94, 2.95, 2.96
// ===========================================================================

#[test]
fn elligator2_both_notsquare_arms() {
    let (cu, ru) = both::<Fn2>("_sodium_ge25519_from_uniform");
    let (ccc, _) = both::<GeUnary>("_sodium_ge25519_clear_cofactor");
    let (ctb, _) = both::<GeToBytes3>("_sodium_ge25519_p3_tobytes");

    // Inputs: the degenerate r = 0, every combination of the two branch bits,
    // and a large randomized sweep.
    let mut inputs: Vec<[u8; 32]> = vec![[0u8; 32], [0xffu8; 32], [0x7fu8; 32]];
    for hi in [0x00u8, 0x20, 0x80, 0xa0, 0xff] {
        let mut v = [0x11u8; 32];
        v[31] = hi;
        inputs.push(v);
        let mut w = [0u8; 32];
        w[31] = hi;
        inputs.push(w);
    }
    let mut rng = Rng::new(0x2_0095_a);
    for _ in 0..300 {
        let mut v: [u8; 32] = rng.bytes(32).try_into().unwrap();
        inputs.push(v);
        v[31] |= 0x80;
        inputs.push(v);
        v[31] &= 0x7f;
        inputs.push(v);
    }

    let mut n_sq = 0usize;
    let mut n_nonsq = 0usize;
    let mut n_sign0 = 0usize;
    let mut n_sign1 = 0usize;
    let mut n_negated = 0usize;
    let mut n_mont_cmov = 0usize;

    for r in &inputs {
        // both libraries, differentially
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            cu(oc.as_mut_ptr(), r.as_ptr());
            ru(or.as_mut_ptr(), r.as_ptr());
        }
        eqb(&format!("ge25519_from_uniform({})", hex(r)), &oc[..32], &or[..32]);
        check_pad("from_uniform(C)", &oc, 32);
        check_pad("from_uniform(Rust)", &or, 32);

        // the replica: predict the whole map, then finish it off with the
        // library's own (exported) cofactor clearing and encoder.
        let tr = from_uniform_trace(r);
        let mut p3 = tr.p3;
        let mut mine = [0u8; 32];
        unsafe {
            ccc(&mut p3);
            ctb(mine.as_mut_ptr(), &p3);
        }
        assert_eq!(
            &oc[..32],
            &mine[..],
            "from_uniform replica disagrees with the C library on r = {}",
            hex(r)
        );

        if tr.notsquare == 0 {
            n_sq += 1;
        } else {
            n_nonsq += 1;
        }
        if tr.x_sign == 0 {
            n_sign0 += 1;
        } else {
            n_sign1 += 1;
        }
        n_negated += tr.x_negated as usize;
        n_mont_cmov += tr.mont_cmov as usize;
    }

    // row 2.95: both arms of `fe25519_notsquare(gx1)` were taken
    assert!(n_sq > 50, "ge25519_elligator2: only {n_sq} square-gx1 inputs");
    assert!(n_nonsq > 50, "ge25519_elligator2: only {n_nonsq} non-square-gx1 inputs");
    // row 2.94: both values of x_sign, and the conditional negation both ways
    assert!(n_sign0 > 50 && n_sign1 > 50, "x_sign coverage: {n_sign0} / {n_sign1}");
    assert!(
        n_negated > 50 && n_negated < inputs.len() - 50,
        "x_sign cmov coverage: {n_negated} of {}",
        inputs.len()
    );
    // row 2.96: the mont_to_ed `iszero(x_plus_one_y_inv)` cmov
    assert!(
        n_mont_cmov >= 1,
        "ge25519_mont_to_ed: the iszero(x_plus_one_y_inv) cmov never fired"
    );
}

/// Row 2.96, pinned down exactly.  `r = 0` is the *only* input (mod the sign
/// bit) that makes `(x+1)*y == 0` inside `ge25519_mont_to_ed`:
///
/// * `x1 = -A/(1+2r^2)`, and `1+2r^2 == 0` has no solution because `-1/2` is a
///   quadratic non-residue mod `2^255-19`, so `x1 == 0` is unreachable;
/// * `y == 0` needs `x^3+Ax^2+x == 0`, i.e. `x == 0` (the other root pair needs
///   `A^2-4` to be a square, which it is not);
/// * `x == 0` after the correction needs `x1 == -A`, i.e. `r == 0`, and `-A` is
///   a non-residue so the `notsquare` arm is the one taken.
///
/// The resulting Edwards point is `(0, 1)` — the identity — so the output of
/// `ge25519_from_uniform` must be the identity encoding.
#[test]
fn mont_to_ed_cmov_path_at_zero() {
    let (cu, ru) = both::<Fn2>("_sodium_ge25519_from_uniform");
    let (ch, rh) = both::<Fn2>("_sodium_ge25519_from_hash");
    let identity = {
        let mut v = [0u8; 32];
        v[0] = 1;
        v
    };

    for r31 in [0x00u8, 0x80] {
        let mut r = [0u8; 32];
        r[31] = r31;
        let tr = from_uniform_trace(&r);
        assert_eq!(tr.notsquare, 1, "gx1 = -A must be a non-residue");
        assert_eq!(tr.mont_cmov, 1, "the mont_to_ed cmov must fire for r = 0");
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            cu(oc.as_mut_ptr(), r.as_ptr());
            ru(or.as_mut_ptr(), r.as_ptr());
        }
        eqb("ge25519_from_uniform(0)", &oc[..32], &or[..32]);
        assert_eq!(&oc[..32], &identity[..], "from_uniform(0) must be the identity");
        check_pad("from_uniform(0) C", &oc, 32);
        check_pad("from_uniform(0) Rust", &or, 32);
    }

    // the same degenerate field element reached through ge25519_from_hash
    for (a, b) in [(0x00u8, 0x00u8), (0x80, 0x00)] {
        let mut h = [0u8; 64];
        h[31] = a;
        h[63] = b;
        let tr = from_hash_trace(&h);
        if tr.mont_cmov == 1 {
            let mut oc = padded(32);
            let mut or = padded(32);
            unsafe {
                ch(oc.as_mut_ptr(), h.as_ptr());
                rh(or.as_mut_ptr(), h.as_ptr());
            }
            eqb("ge25519_from_hash(degenerate)", &oc[..32], &or[..32]);
            assert_eq!(&oc[..32], &identity[..]);
        }
    }
}

// ===========================================================================
// 2. ge25519_from_hash / fe25519_reduce64 — configs_2.md row 2.103
// ===========================================================================

#[test]
fn from_hash_reduce64_and_notsquare_arms() {
    let (ch, rh) = both::<Fn2>("_sodium_ge25519_from_hash");
    let (ccc, _) = both::<GeUnary>("_sodium_ge25519_clear_cofactor");
    let (ctb, _) = both::<GeToBytes3>("_sodium_ge25519_p3_tobytes");

    // every combination of the two high bits that feed the `* 19` / `* 722`
    // corrections, on both an all-zero and an all-one carrier
    let mut inputs: Vec<[u8; 64]> = Vec::new();
    for a in [0x00u8, 0x80] {
        for b in [0x00u8, 0x80] {
            for carrier in [0x00u8, 0x33, 0xff] {
                let mut v = [carrier; 64];
                v[31] = (v[31] & 0x7f) | a;
                v[63] = (v[63] & 0x7f) | b;
                inputs.push(v);
            }
        }
    }
    let mut rng = Rng::new(0x2_0103_a);
    for _ in 0..400 {
        inputs.push(rng.bytes(64).try_into().unwrap());
    }

    let mut bits = [0usize; 4];
    let mut n_sq = 0usize;
    let mut n_nonsq = 0usize;
    let mut n_yneg = 0usize;
    for h in &inputs {
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            ch(oc.as_mut_ptr(), h.as_ptr());
            rh(or.as_mut_ptr(), h.as_ptr());
        }
        eqb(&format!("ge25519_from_hash({})", hex(&h[..8])), &oc[..32], &or[..32]);
        check_pad("from_hash(C)", &oc, 32);
        check_pad("from_hash(Rust)", &or, 32);

        let tr = from_hash_trace(h);
        let mut p3 = tr.p3;
        let mut mine = [0u8; 32];
        unsafe {
            ccc(&mut p3);
            ctb(mine.as_mut_ptr(), &p3);
        }
        assert_eq!(
            &oc[..32],
            &mine[..],
            "from_hash replica disagrees with the C library on h = {}",
            hex(h)
        );
        bits[((h[31] >> 7) * 2 + (h[63] >> 7)) as usize] += 1;
        if tr.notsquare == 0 {
            n_sq += 1;
        } else {
            n_nonsq += 1;
        }
        n_yneg += tr.y_negated as usize;
    }
    for (i, n) in bits.iter().enumerate() {
        assert!(*n > 0, "fe25519_reduce64: high-bit combination {i} never exercised");
    }
    assert!(n_sq > 50 && n_nonsq > 50, "from_hash notsquare coverage: {n_sq} / {n_nonsq}");
    assert!(
        n_yneg > 0 && n_yneg < inputs.len(),
        "from_hash y_sign cmov coverage: {n_yneg} of {}",
        inputs.len()
    );
}

// ===========================================================================
// 3. ristretto255_frombytes — errors_2.md rows 2.12, 2.14, 2.15, 2.16, 2.17
// ===========================================================================

/// The four rejection reasons of `ristretto255_frombytes` are folded into one
/// `-1`.  Classify each input with the replica, check the replica against the
/// library (return code *and* decoded `p3`), and require every reason to have
/// fired at least once.
#[test]
fn ristretto255_frombytes_every_rejection_arm() {
    let (cfb, rfb) = both::<GeFromBytes>("_sodium_ristretto255_frombytes");
    let (valid_c, valid_r) = both::<Pred>("crypto_core_ristretto255_is_valid_point");

    let mut inputs: Vec<[u8; 32]> = Vec::new();
    // canonical & valid: the identity and the base point
    inputs.push([0u8; 32]);
    inputs.push([
        0xe2, 0xf2, 0xae, 0x0a, 0x6a, 0xbc, 0x4e, 0x71, 0xa8, 0x84, 0xa9, 0x61, 0xc5, 0x00, 0x51,
        0x5f, 0x58, 0xe3, 0x0b, 0x6a, 0xa5, 0x82, 0xdd, 0x8d, 0xb6, 0xa6, 0x59, 0x45, 0xe0, 0x8d,
        0x2d, 0x76,
    ]);
    // p - 1: canonical, even, but 1 - s^2 == 0 -> Y == 0 (row 2.16)
    {
        let mut v = [0xffu8; 32];
        v[0] = 0xec;
        v[31] = 0x7f;
        inputs.push(v);
    }
    // non-canonical: odd s[0], bit 255 set, s >= p (row 2.12)
    inputs.push({
        let mut v = [0u8; 32];
        v[0] = 1;
        v
    });
    inputs.push({
        let mut v = [0u8; 32];
        v[31] = 0x80;
        v
    });
    for first in [0xecu8, 0xed, 0xee, 0xf0, 0xfe] {
        let mut v = [0xffu8; 32];
        v[0] = first;
        v[31] = 0x7f;
        inputs.push(v);
    }
    inputs.push([0xffu8; 32]);
    // every small even value: a dense source of both the non-square arm and the
    // T-negative arm
    for x in 0u16..128 {
        let mut v = [0u8; 32];
        v[0] = (x * 2) as u8;
        v[1] = (x >> 7) as u8;
        inputs.push(v);
    }
    let mut rng = Rng::new(0x2_0f14);
    for _ in 0..300 {
        inputs.push(rng.bytes(32).try_into().unwrap());
    }
    // forced through the canonicity gate so the field work always runs
    for _ in 0..900 {
        let mut v: [u8; 32] = rng.bytes(32).try_into().unwrap();
        v[0] &= 0xfe;
        v[31] &= 0x7f;
        inputs.push(v);
    }

    let mut n = [0usize; 5]; // accept, noncanonical, notsquare, t_negative, y_zero
    for s in &inputs {
        let mut pc = P3::new();
        let mut pr = P3::new();
        let (rc, rr) = unsafe { (cfb(&mut pc, s.as_ptr()), rfb(&mut pr, s.as_ptr())) };
        eqi(&format!("ristretto255_frombytes({})", hex(s)), rc, rr);
        eqb(&format!("ristretto255_frombytes p3 ({})", hex(s)), &pc.bytes(), &pr.bytes());
        let (vc, vr) = unsafe { (valid_c(s.as_ptr()), valid_r(s.as_ptr())) };
        eqi("ristretto255_is_valid_point", vc, vr);
        assert_eq!(vc, if rc == 0 { 1 } else { 0 });

        let (mine, d) = rist_frombytes(s);
        assert_eq!(
            d.ret, rc,
            "replica return {} != C {rc} for {}",
            d.ret,
            hex(s)
        );
        if let Some(m) = mine {
            // the library leaves p3 fully written even on the -1 path, so the
            // coordinates must match in *both* cases
            for (i, name) in ["X", "Y", "Z", "T"].iter().enumerate() {
                assert_eq!(
                    m.get(i).to_bytes(),
                    pc.get(i).to_bytes(),
                    "ristretto255_frombytes replica: {name} differs for {}",
                    hex(s)
                );
            }
        }
        if d.ret == 0 {
            n[0] += 1;
        }
        n[1] += d.noncanonical as usize;
        n[2] += d.notsquare as usize;
        n[3] += d.t_negative as usize;
        n[4] += d.y_zero as usize;
    }

    assert!(n[0] > 50, "no accepted encodings ({} of {})", n[0], inputs.len());
    assert!(n[1] > 0, "errors_2 row 2.12/2.13 (non-canonical) never fired");
    assert!(n[2] > 0, "errors_2 row 2.14/2.17 (sqrt_ratio_m1 == 0) never fired");
    assert!(n[3] > 0, "errors_2 row 2.15 (isnegative(T)) never fired");
    assert!(n[4] > 0, "errors_2 row 2.16 (iszero(Y)) never fired");
}

// ===========================================================================
// 4. ristretto255_p3_tobytes — configs_2.md row 2.114
// ===========================================================================

#[test]
fn ristretto255_p3_tobytes_both_rotate_arms() {
    let (cfb, _) = both::<GeFromBytes>("_sodium_ristretto255_frombytes");
    let (ctb, rtb) = both::<GeToBytes3>("_sodium_ristretto255_p3_tobytes");
    let (cfh, _) = both::<Fn2i>("crypto_core_ristretto255_from_hash");
    let (cadd, _) = both::<GeP3Op>("_sodium_ge25519_p3_add");

    // A pool of ristretto255 p3 values: decoded valid encodings, plus the
    // *un-normalised* projective points that `ristretto255_from_hash` builds
    // (Z != 1), which is where `rotate == 1` actually shows up.
    let mut pool: Vec<P3> = Vec::new();
    let mut rng = Rng::new(0x2_0114);
    for _ in 0..250 {
        let h = rng.bytes(64);
        let mut enc = [0u8; 32];
        assert_eq!(unsafe { cfh(enc.as_mut_ptr(), h.as_ptr()) }, 0);
        let mut p = P3::new();
        assert_eq!(unsafe { cfb(&mut p, enc.as_ptr()) }, 0);
        pool.push(p);
        // the pre-encoding projective sum, straight out of the replica
        let ha: [u8; 32] = h[..32].try_into().unwrap();
        let hb: [u8; 32] = h[32..].try_into().unwrap();
        let (p0, _) = rist_elligator(Fe::from_bytes(&ha));
        let (p1, _) = rist_elligator(Fe::from_bytes(&hb));
        let mut sum = P3::new();
        unsafe { cadd(&mut sum, &p0, &p1) };
        pool.push(sum);
    }

    let mut n_rot = [0usize; 2];
    let mut n_xz = [0usize; 2];
    for p in &pool {
        let mut oc = padded(32);
        let mut or = padded(32);
        unsafe {
            ctb(oc.as_mut_ptr(), p);
            rtb(or.as_mut_ptr(), p);
        }
        eqb("ristretto255_p3_tobytes", &oc[..32], &or[..32]);
        check_pad("ristretto255_p3_tobytes(C)", &oc, 32);
        check_pad("ristretto255_p3_tobytes(Rust)", &or, 32);
        let (mine, rotate, xzneg) = rist_p3_tobytes(p.x(), p.y(), p.z(), p.t());
        assert_eq!(
            &oc[..32],
            &mine[..],
            "ristretto255_p3_tobytes replica disagrees with the C library"
        );
        n_rot[rotate as usize] += 1;
        n_xz[xzneg as usize] += 1;
    }
    assert!(
        n_rot[0] > 10 && n_rot[1] > 10,
        "row 2.114: rotate coverage {} / {}",
        n_rot[0],
        n_rot[1]
    );
    assert!(
        n_xz[0] > 10 && n_xz[1] > 10,
        "row 2.114: isnegative(x_z_inv) coverage {} / {}",
        n_xz[0],
        n_xz[1]
    );
}

// ===========================================================================
// 5. ristretto255_elligator — configs_2.md rows 2.112, 2.113
// ===========================================================================

#[test]
fn ristretto255_elligator_both_square_arms() {
    let (cfh, rfh) = both::<Fn2i>("crypto_core_ristretto255_from_hash");
    let (cadd, _) = both::<GeP3Op>("_sodium_ge25519_p3_add");
    let (ctb, _) = both::<GeToBytes3>("_sodium_ristretto255_p3_tobytes");

    let mut inputs: Vec<[u8; 64]> = vec![[0u8; 64], [0xffu8; 64]];
    // t == 0 in one half only, and in both (row 2.112)
    {
        let mut v = [0u8; 64];
        v[32] = 7;
        inputs.push(v);
        let mut w = [0u8; 64];
        w[0] = 7;
        inputs.push(w);
    }
    for i in 0..64 {
        let mut v = [0u8; 64];
        v[i] = 1;
        inputs.push(v);
    }
    let mut rng = Rng::new(0x2_0113);
    for _ in 0..400 {
        inputs.push(rng.bytes(64).try_into().unwrap());
    }

    let mut n_sq = 0usize;
    let mut n_nonsq = 0usize;
    for h in &inputs {
        let mut oc = padded(32);
        let mut or = padded(32);
        let rc = unsafe { cfh(oc.as_mut_ptr(), h.as_ptr()) };
        let rr = unsafe { rfh(or.as_mut_ptr(), h.as_ptr()) };
        eqi("ristretto255_from_hash ret", rc, rr);
        assert_eq!(rc, 0);
        eqb(&format!("ristretto255_from_hash({})", hex(&h[..8])), &oc[..32], &or[..32]);
        check_pad("from_hash(C)", &oc, 32);
        check_pad("from_hash(Rust)", &or, 32);

        let ha: [u8; 32] = h[..32].try_into().unwrap();
        let hb: [u8; 32] = h[32..].try_into().unwrap();
        let (p0, w0) = rist_elligator(Fe::from_bytes(&ha));
        let (p1, w1) = rist_elligator(Fe::from_bytes(&hb));
        let mut sum = P3::new();
        let mut mine = [0u8; 32];
        unsafe {
            cadd(&mut sum, &p0, &p1);
            ctb(mine.as_mut_ptr(), &sum);
        }
        assert_eq!(
            &oc[..32],
            &mine[..],
            "ristretto255_elligator replica disagrees with the C library on h = {}",
            hex(h)
        );
        for w in [w0, w1] {
            if w == 0 {
                n_sq += 1;
            } else {
                n_nonsq += 1;
            }
        }
    }
    assert!(
        n_sq > 50 && n_nonsq > 50,
        "row 2.113: sqrt_ratio_m1 coverage {n_sq} square / {n_nonsq} non-square"
    );
}

// ===========================================================================
// 6. crypto_core_ed25519_scalar_random — errors_2.md row 2.39
// ===========================================================================

/// `crypto_core_ed25519_scalar_random` has no error return; its only "rejection"
/// is the `do { randombytes_buf } while (!canonical || zero)` re-draw loop.
/// Observing that the loop *runs* needs knowledge of the byte stream the library
/// is fed, which the harness fully controls, so replay the same xorshift stream
/// here, predict which candidate is accepted, and require that both the
/// first-candidate-accepted and the re-draw case actually occur.
#[test]
fn scalar_random_redraw_loop() {
    type Fn1 = unsafe extern "C" fn(*mut u8);
    let (c, r) = both::<Fn1>("crypto_core_ed25519_scalar_random");
    let (canon, _) = both::<Pred>("_sodium_sc25519_is_canonical");

    /// The harness' per-library RNG: xorshift64, 8 bytes per step.
    fn stream(seed: u64, words: usize) -> Vec<u8> {
        let mut s = if seed == 0 { 0x2545_F491_4F6C_DD1D } else { seed };
        let mut out = Vec::with_capacity(words * 8);
        for _ in 0..words {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    const MAX_DRAWS: usize = 40;
    let mut draws_hist = [0usize; MAX_DRAWS + 1];
    for i in 0..300u64 {
        let seed = 0xc0ffee_0000_0001u64 ^ (i.wrapping_mul(0x9E37_79B9) | 1);
        // predict: walk the stream in 32-byte candidates until one is accepted
        let bytes = stream(seed, 4 * MAX_DRAWS);
        let mut predicted = [0u8; 32];
        let mut draws = 0usize;
        for k in 0..MAX_DRAWS {
            let mut cand = [0u8; 32];
            cand.copy_from_slice(&bytes[k * 32..k * 32 + 32]);
            cand[31] &= 0x1f;
            draws += 1;
            if unsafe { canon(cand.as_ptr()) } == 1 && cand != [0u8; 32] {
                predicted = cand;
                break;
            }
        }
        assert_ne!(predicted, [0u8; 32], "no candidate accepted within {MAX_DRAWS} draws");

        rng_reseed(seed);
        let mut oc = padded(32);
        unsafe { c(oc.as_mut_ptr()) };
        rng_reseed(seed);
        let mut or = padded(32);
        unsafe { r(or.as_mut_ptr()) };
        eqb("crypto_core_ed25519_scalar_random", &oc[..32], &or[..32]);
        check_pad("scalar_random(C)", &oc, 32);
        check_pad("scalar_random(Rust)", &or, 32);
        assert_eq!(
            &oc[..32],
            &predicted[..],
            "scalar_random accepted a different candidate than the {draws}th"
        );
        draws_hist[draws.min(MAX_DRAWS)] += 1;
    }
    rng_reset();
    assert!(draws_hist[1] > 0, "no seed was accepted on the first draw");
    assert!(
        draws_hist[2..].iter().sum::<usize>() > 0,
        "the scalar_random re-draw loop never iterated: {draws_hist:?}"
    );
}

// ===========================================================================
// 7. core_h2c_string_to_hash — errors_2.md rows 2.42 / 2.43
// ===========================================================================

/// `core_h2c_string_to_hash_sha256` / `_sha512` both start with
/// `assert(h_len <= 0xff)`.  The reference build does not define `NDEBUG`, so the
/// assertion is live; prove that by checking the *outcome* is a fatal signal
/// (`SIGABRT`) rather than merely that both libraries agree, which `eq_abort`
/// alone would also accept if the assert had been compiled out.
#[test]
fn core_h2c_h_len_assert_is_live() {
    type H2CHash =
        unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u8, usize, c_int) -> c_int;
    let (c, r) = both::<H2CHash>("_sodium_core_h2c_string_to_hash");

    // h_len <= 0xff must *not* abort, on either implementation
    for alg in [1i32, 2] {
        for h_len in [1usize, 48, 96, 255] {
            for (side, f) in [("C", &c), ("Rust", &r)] {
                let g = f.clone();
                let st = in_child(move || unsafe {
                    let mut o = vec![0u8; h_len];
                    g(o.as_mut_ptr(), h_len, b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg);
                });
                assert_eq!(
                    status_str(st),
                    "exit:0",
                    "{side}: h_len={h_len} alg={alg} must not abort"
                );
            }
        }
    }

    // h_len > 0xff must abort with a fatal signal on both
    for alg in [1i32, 2] {
        for h_len in [256usize, 257, 1000] {
            let mut outcomes = Vec::new();
            for (side, f) in [("C", &c), ("Rust", &r)] {
                let g = f.clone();
                let st = in_child(move || unsafe {
                    let mut o = vec![0u8; h_len];
                    g(o.as_mut_ptr(), h_len, b"c".as_ptr(), 1, b"m".as_ptr(), 1, alg);
                });
                let s = status_str(st);
                assert!(
                    s.starts_with("sig:"),
                    "{side}: assert(h_len <= 0xff) did not fire for h_len={h_len} alg={alg} \
                     (outcome {s}) — is NDEBUG defined?"
                );
                outcomes.push(s);
            }
            assert_eq!(outcomes[0], outcomes[1], "C and Rust abort differently");
        }
    }
}
