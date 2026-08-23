//! Translation of dtoa.c (David Gay's dtoa/strtod).
//!
//! Active configuration (from jansson_private_config.h + x86_64 Linux):
//!   IEEE_8087 (little-endian), long long available.
//!   USE_BF96 IS defined (neither NO_LONG_LONG nor NO_BF96 is): dtoa_r uses the
//!   exact-integer / 96-bit software-bigfloat fast paths built on the
//!   pten/Lhint/pfive/pfivebits tables and dtoa_divmax, falling back to the
//!   Bigint code only at Fast_failed1.  The `#ifndef USE_BF96` classic path
//!   (including try_quick) is dead in this build and is NOT translated here.
//!   Sudden_Underflow, SET_INEXACT, Honor_FLT_ROUNDS, Check_FLT_ROUNDS,
//!   ROUND_BIASED, No_leftright, Just_16, VAX, IBM, IEEE_MC68k are all undefined.
//!
//! Exported symbols: dtoa_r, dtoa, freedtoa, gethex, strtod__unused,
//! plus the data symbol dtoa_divmax.
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(dead_code)]

use crate::memory::{jsonp_free, jsonp_malloc};
use core::ffi::{c_char, c_int, c_void};
use core::ptr;

#[path = "dtoa_tables.rs"]
mod dtoa_tables;
use dtoa_tables::{LHINT, PTEN};

// ---- Permit experimenting data symbol (must be exported) ----
#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

// ==== IEEE_Arith constants (P=53 double) ====
const EXP_SHIFT: c_int = 20;
const EXP_SHIFT1: c_int = 20;
const EXP_MSK1: u32 = 0x100000;
const EXP_MSK11: u32 = 0x100000;
const EXP_MASK: u32 = 0x7ff00000;
const P: c_int = 53;
const BIAS: c_int = 1023;
const EMIN: c_int = -1022;
const EXP_1: u32 = 0x3ff00000;
const EXP_11: u32 = 0x3ff00000;
const EBITS: c_int = 11;
const FRAC_MASK: u32 = 0xfffff;
const FRAC_MASK1: u32 = 0xfffff;
const TEN_PMAX: c_int = 22;
const BNDRY_MASK: u32 = 0xfffff;
const BNDRY_MASK1: u32 = 0xfffff;
const LSB: u32 = 1;
const SIGN_BIT: u32 = 0x80000000;
const LOG2P: c_int = 1;
const QUICK_MAX: c_int = 14;
const INT_MAX_: c_int = 14;
const DBL_MAX_EXP: c_int = 1024;
const DBL_MAX_10_EXP: c_int = 308;
const DBL_DIG: c_int = 15;
const N_BIGTENS: c_int = 5;
const SCALE_BIT: c_int = 0x10;
const ULBITS: c_int = 32;
const KSHIFT: c_int = 5;
const KMASK: c_int = 31;
const KMAX: c_int = 7;
const FLT_RADIX: f64 = 2.0;

const BIG0: u32 = FRAC_MASK1 | (EXP_MSK1.wrapping_mul((DBL_MAX_EXP + BIAS - 1) as u32));
const BIG1: u32 = 0xffffffff;

const FFFFFFFF: u64 = 0xffffffff;

const ERANGE: c_int = 34;

extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn __errno_location() -> *mut c_int;
}

#[inline]
unsafe fn set_errno(x: c_int) {
    *__errno_location() = x;
}

// ==== The U union: { double d; ULong L[2]; ULLong LL; } ====
// IEEE_8087 (little endian): word0 = L[1] (high 32), word1 = L[0] (low 32).
#[derive(Clone, Copy)]
struct U {
    bits: u64,
}

impl U {
    #[inline]
    fn new() -> U {
        U { bits: 0 }
    }
    #[inline]
    fn from_d(d: f64) -> U {
        U { bits: d.to_bits() }
    }
    #[inline]
    fn d(&self) -> f64 {
        f64::from_bits(self.bits)
    }
    #[inline]
    fn set_d(&mut self, d: f64) {
        self.bits = d.to_bits();
    }
    #[inline]
    fn ll(&self) -> u64 {
        self.bits
    }
    #[inline]
    fn set_ll(&mut self, v: u64) {
        self.bits = v;
    }
    #[inline]
    fn word0(&self) -> u32 {
        (self.bits >> 32) as u32
    }
    #[inline]
    fn word1(&self) -> u32 {
        self.bits as u32
    }
    #[inline]
    fn set_word0(&mut self, w: u32) {
        self.bits = ((w as u64) << 32) | (self.bits & 0xffffffff);
    }
    #[inline]
    fn set_word1(&mut self, w: u32) {
        self.bits = (self.bits & 0xffffffff00000000) | (w as u64);
    }
}

// ==== Bigint ====
// struct Bigint { Bigint *next; int k, maxwds, sign, wds; ULong x[1]; }
#[repr(C)]
struct Bigint {
    next: *mut Bigint,
    k: c_int,
    maxwds: c_int,
    sign: c_int,
    wds: c_int,
    // followed by x[maxwds] ULong (u32); x[0] lives here:
    x: [u32; 1],
}

#[inline]
fn offset_of_x() -> usize {
    core::mem::offset_of!(Bigint, x)
}

#[inline]
unsafe fn bx(b: *mut Bigint) -> *mut u32 {
    (b as *mut u8).add(offset_of_x()) as *mut u32
}

const KMAX_USIZE: usize = KMAX as usize;

// freelist and p5s (single-threaded: TI0)
static mut FREELIST: [*mut Bigint; KMAX_USIZE + 1] = [ptr::null_mut(); KMAX_USIZE + 1];
static mut P5S: *mut Bigint = ptr::null_mut();

unsafe fn balloc(k: c_int) -> *mut Bigint {
    let rv: *mut Bigint;
    if k <= KMAX && !FREELIST[k as usize].is_null() {
        rv = FREELIST[k as usize];
        FREELIST[k as usize] = (*rv).next;
    } else {
        let x = 1usize << k;
        // len in units of double: (sizeof(Bigint) + (x-1)*4 + 7)/8, then *8 bytes.
        // We just allocate offset_of_x + x*4 bytes rounded up to 8, matching capacity.
        let bytes_needed = offset_of_x() + x * core::mem::size_of::<u32>();
        // Round up to multiple of 8 (sizeof double) as C does via the len formula.
        let sizeof_bigint = core::mem::size_of::<Bigint>();
        let len = (sizeof_bigint + (x - 1) * 4 + 8 - 1) / 8;
        let alloc_bytes = core::cmp::max(len * 8, bytes_needed);
        rv = jsonp_malloc(alloc_bytes) as *mut Bigint;
        if rv.is_null() {
            return ptr::null_mut();
        }
        (*rv).k = k;
        (*rv).maxwds = x as c_int;
    }
    (*rv).sign = 0;
    (*rv).wds = 0;
    (*rv).next = ptr::null_mut();
    rv
}

unsafe fn bfree(v: *mut Bigint) {
    if !v.is_null() {
        if (*v).k > KMAX {
            jsonp_free(v as *mut c_void);
        } else {
            (*v).next = FREELIST[(*v).k as usize];
            FREELIST[(*v).k as usize] = v;
        }
    }
}

// Bcopy(x,y): memcpy(&x->sign, &y->sign, y->wds*sizeof(Long)+2*sizeof(int))
unsafe fn bcopy(dst: *mut Bigint, src: *mut Bigint) {
    let n = (*src).wds as usize * core::mem::size_of::<u32>() + 2 * core::mem::size_of::<c_int>();
    memcpy(
        &mut (*dst).sign as *mut c_int as *mut c_void,
        &(*src).sign as *const c_int as *const c_void,
        n,
    );
}

unsafe fn multadd(mut b: *mut Bigint, m: c_int, a: c_int) -> *mut Bigint {
    let wds = (*b).wds;
    let mut x = bx(b);
    let mut i = 0;
    let mut carry: u64 = a as u64;
    loop {
        let y: u64 = (*x as u64) * (m as u64) + carry;
        carry = y >> 32;
        *x = (y & FFFFFFFF) as u32;
        x = x.add(1);
        i += 1;
        if i >= wds {
            break;
        }
    }
    let mut wds = wds;
    if carry != 0 {
        if wds >= (*b).maxwds {
            let b1 = balloc((*b).k + 1);
            bcopy(b1, b);
            bfree(b);
            b = b1;
        }
        *bx(b).add(wds as usize) = carry as u32;
        wds += 1;
        (*b).wds = wds;
    }
    b
}

unsafe fn s2b(s: *const c_char, nd0: c_int, nd: c_int, y9: u32, dplen: c_int) -> *mut Bigint {
    let mut b: *mut Bigint;
    let mut i: c_int;
    let mut k: c_int;
    let x: i32;
    let mut y: i32;

    x = (nd + 8) / 9;
    k = 0;
    y = 1;
    while x > y {
        y <<= 1;
        k += 1;
    }
    b = balloc(k);
    *bx(b).add(0) = y9;
    (*b).wds = 1;

    let mut s = s;
    i = 9;
    if 9 < nd0 {
        s = s.add(9);
        loop {
            let c = *s as u8 as c_int - '0' as c_int;
            s = s.add(1);
            b = multadd(b, 10, c);
            i += 1;
            if i >= nd0 {
                break;
            }
        }
        s = s.add(dplen as usize);
    } else {
        s = s.add((dplen + 9) as usize);
    }
    while i < nd {
        let c = *s as u8 as c_int - '0' as c_int;
        s = s.add(1);
        b = multadd(b, 10, c);
        i += 1;
    }
    b
}

fn hi0bits(mut x: u32) -> c_int {
    let mut k = 0;
    if x & 0xffff0000 == 0 {
        k = 16;
        x <<= 16;
    }
    if x & 0xff000000 == 0 {
        k += 8;
        x <<= 8;
    }
    if x & 0xf0000000 == 0 {
        k += 4;
        x <<= 4;
    }
    if x & 0xc0000000 == 0 {
        k += 2;
        x <<= 2;
    }
    if x & 0x80000000 == 0 {
        k += 1;
        if x & 0x40000000 == 0 {
            return 32;
        }
    }
    k
}

fn lo0bits(y: &mut u32) -> c_int {
    let mut k;
    let mut x = *y;

    if x & 7 != 0 {
        if x & 1 != 0 {
            return 0;
        }
        if x & 2 != 0 {
            *y = x >> 1;
            return 1;
        }
        *y = x >> 2;
        return 2;
    }
    k = 0;
    if x & 0xffff == 0 {
        k = 16;
        x >>= 16;
    }
    if x & 0xff == 0 {
        k += 8;
        x >>= 8;
    }
    if x & 0xf == 0 {
        k += 4;
        x >>= 4;
    }
    if x & 0x3 == 0 {
        k += 2;
        x >>= 2;
    }
    if x & 1 == 0 {
        k += 1;
        x >>= 1;
        if x == 0 {
            return 32;
        }
    }
    *y = x;
    k
}

unsafe fn i2b(i: c_int) -> *mut Bigint {
    let b = balloc(1);
    *bx(b).add(0) = i as u32;
    (*b).wds = 1;
    b
}

unsafe fn mult(mut a: *mut Bigint, mut b: *mut Bigint) -> *mut Bigint {
    if (*a).wds < (*b).wds {
        let c = a;
        a = b;
        b = c;
    }
    let k = (*a).k;
    let wa = (*a).wds;
    let wb = (*b).wds;
    let mut wc = wa + wb;
    let mut kk = k;
    if wc > (*a).maxwds {
        kk += 1;
    }
    let c = balloc(kk);
    {
        let mut x = bx(c);
        let xa = bx(c).add(wc as usize);
        while x < xa {
            *x = 0;
            x = x.add(1);
        }
    }
    let xa = bx(a);
    let xae = xa.add(wa as usize);
    let mut xb = bx(b);
    let xbe = xb.add(wb as usize);
    let mut xc0 = bx(c);

    while xb < xbe {
        let y = *xb as u64;
        xb = xb.add(1);
        if y != 0 {
            let mut x = xa;
            let mut xc = xc0;
            let mut carry: u64 = 0;
            loop {
                let z: u64 = (*x as u64) * y + (*xc as u64) + carry;
                x = x.add(1);
                carry = z >> 32;
                *xc = (z & FFFFFFFF) as u32;
                xc = xc.add(1);
                if x >= xae {
                    break;
                }
            }
            *xc = carry as u32;
        }
        xc0 = xc0.add(1);
    }
    // trim
    let xc0b = bx(c);
    let mut xc = bx(c).add(wc as usize);
    while wc > 0 {
        xc = xc.offset(-1);
        if *xc != 0 {
            break;
        }
        wc -= 1;
    }
    (*c).wds = wc;
    c
}

unsafe fn pow5mult(mut b: *mut Bigint, mut k: c_int) -> *mut Bigint {
    static P05: [c_int; 3] = [5, 25, 125];

    let i = k & 3;
    if i != 0 {
        b = multadd(b, P05[(i - 1) as usize], 0);
    }

    k >>= 2;
    if k == 0 {
        return b;
    }
    if P5S.is_null() {
        P5S = i2b(625);
        (*P5S).next = ptr::null_mut();
    }
    let mut p5 = P5S;
    loop {
        if k & 1 != 0 {
            let b1 = mult(b, p5);
            bfree(b);
            b = b1;
        }
        k >>= 1;
        if k == 0 {
            break;
        }
        if (*p5).next.is_null() {
            (*p5).next = mult(p5, p5);
            (*(*p5).next).next = ptr::null_mut();
        }
        p5 = (*p5).next;
    }
    b
}

unsafe fn lshift(b: *mut Bigint, k: c_int) -> *mut Bigint {
    let n = k >> 5;
    let mut k1 = (*b).k;
    let mut n1 = n + (*b).wds + 1;
    let mut i = (*b).maxwds;
    while n1 > i {
        k1 += 1;
        i <<= 1;
    }
    let b1 = balloc(k1);
    let mut x1 = bx(b1);
    for _ in 0..n {
        *x1 = 0;
        x1 = x1.add(1);
    }
    let mut x = bx(b);
    let xe = bx(b).add((*b).wds as usize);
    let kk = k & 0x1f;
    if kk != 0 {
        let k1b = 32 - kk;
        let mut z: u32 = 0;
        loop {
            *x1 = (*x << kk) | z;
            x1 = x1.add(1);
            z = *x >> k1b;
            x = x.add(1);
            if x >= xe {
                break;
            }
        }
        *x1 = z;
        if z != 0 {
            n1 += 1;
        }
    } else {
        loop {
            *x1 = *x;
            x1 = x1.add(1);
            x = x.add(1);
            if x >= xe {
                break;
            }
        }
    }
    (*b1).wds = n1 - 1;
    bfree(b);
    b1
}

unsafe fn cmp(a: *mut Bigint, b: *mut Bigint) -> c_int {
    let mut i = (*a).wds;
    let j = (*b).wds;
    i -= j;
    if i != 0 {
        return i;
    }
    let xa0 = bx(a);
    let mut xa = bx(a).add(j as usize);
    let xb0 = bx(b);
    let mut xb = bx(b).add(j as usize);
    loop {
        xa = xa.offset(-1);
        xb = xb.offset(-1);
        if *xa != *xb {
            return if *xa < *xb { -1 } else { 1 };
        }
        if xa <= xa0 {
            break;
        }
    }
    let _ = xb0;
    0
}

unsafe fn diff(mut a: *mut Bigint, mut b: *mut Bigint) -> *mut Bigint {
    let mut i = cmp(a, b);
    if i == 0 {
        let c = balloc(0);
        (*c).wds = 1;
        *bx(c).add(0) = 0;
        return c;
    }
    if i < 0 {
        let c = a;
        a = b;
        b = c;
        i = 1;
    } else {
        i = 0;
    }
    let c = balloc((*a).k);
    (*c).sign = i;
    let mut wa = (*a).wds;
    let mut xa = bx(a);
    let xae = xa.add(wa as usize);
    let wb = (*b).wds;
    let mut xb = bx(b);
    let xbe = xb.add(wb as usize);
    let mut xc = bx(c);
    let mut borrow: u64 = 0;
    loop {
        let y: u64 = (*xa as u64).wrapping_sub(*xb as u64).wrapping_sub(borrow);
        xa = xa.add(1);
        xb = xb.add(1);
        borrow = (y >> 32) & 1;
        *xc = (y & FFFFFFFF) as u32;
        xc = xc.add(1);
        if xb >= xbe {
            break;
        }
    }
    while xa < xae {
        let y: u64 = (*xa as u64).wrapping_sub(borrow);
        xa = xa.add(1);
        borrow = (y >> 32) & 1;
        *xc = (y & FFFFFFFF) as u32;
        xc = xc.add(1);
    }
    loop {
        xc = xc.offset(-1);
        if *xc != 0 {
            break;
        }
        wa -= 1;
    }
    (*c).wds = wa;
    c
}

unsafe fn ulp(x: &U) -> f64 {
    // Avoid_Underflow defined => the simple branch always executes.
    let l: i32 = ((x.word0() & EXP_MASK) as i32) - (P - 1) * (EXP_MSK1 as i32);
    let mut u = U::new();
    u.set_word0(l as u32);
    u.set_word1(0);
    u.d()
}

unsafe fn b2d(a: *mut Bigint, e: &mut c_int) -> f64 {
    let xa0 = bx(a);
    let mut xa = bx(a).add((*a).wds as usize);
    xa = xa.offset(-1);
    let mut y = *xa;
    let mut d = U::new();
    let k = hi0bits(y);
    *e = 32 - k;
    // Pack_32
    macro_rules! d0 { () => {} }
    if k < EBITS {
        let d0v = EXP_1 | (y >> (EBITS - k));
        let w = if xa > xa0 {
            xa = xa.offset(-1);
            *xa
        } else {
            0
        };
        let d1v = (y << ((32 - EBITS) + k)) | (w >> (EBITS - k));
        d.set_word0(d0v);
        d.set_word1(d1v);
        return d.d();
    }
    let mut z = if xa > xa0 {
        xa = xa.offset(-1);
        *xa
    } else {
        0
    };
    let kk = k - EBITS;
    if kk != 0 {
        let d0v = EXP_1 | (y << kk) | (z >> (32 - kk));
        y = if xa > xa0 {
            xa = xa.offset(-1);
            *xa
        } else {
            0
        };
        let d1v = (z << kk) | (y >> (32 - kk));
        d.set_word0(d0v);
        d.set_word1(d1v);
    } else {
        d.set_word0(EXP_1 | y);
        d.set_word1(z);
    }
    let _ = &mut z;
    d.d()
}

unsafe fn d2b(d: &U, e: &mut c_int, bits: &mut c_int) -> *mut Bigint {
    let de: c_int;
    let mut k: c_int;
    let mut y: u32;
    let mut z: u32;
    let mut i: c_int;

    let b = balloc(1);
    let x = bx(b);

    let d0 = d.word0();
    let d1 = d.word1();

    z = d0 & FRAC_MASK;
    let d0m = d0 & 0x7fffffff; // clear sign bit

    de = (d0m >> EXP_SHIFT) as c_int;
    if de != 0 {
        z |= EXP_MSK1;
    }

    y = d1;
    if y != 0 {
        k = lo0bits(&mut y);
        if k != 0 {
            *x.add(0) = y | (z << (32 - k));
            z >>= k;
        } else {
            *x.add(0) = y;
        }
        i = if z != 0 {
            *x.add(1) = z;
            2
        } else {
            1
        };
        (*b).wds = i;
    } else {
        k = lo0bits(&mut z);
        *x.add(0) = z;
        i = 1;
        (*b).wds = 1;
        k += 32;
    }
    if de != 0 {
        // normal
        *e = de - BIAS - (P - 1) + k;
        *bits = P - k;
    } else {
        *e = de - BIAS - (P - 1) + 1 + k;
        *bits = 32 * i - hi0bits(*x.add((i - 1) as usize));
    }
    b
}

unsafe fn ratio(a: *mut Bigint, b: *mut Bigint) -> f64 {
    let mut da = U::new();
    let mut db = U::new();
    let mut ka: c_int = 0;
    let mut kb: c_int = 0;
    da.set_d(b2d(a, &mut ka));
    db.set_d(b2d(b, &mut kb));
    let k = ka - kb + 32 * ((*a).wds - (*b).wds);
    if k > 0 {
        da.set_word0(da.word0().wrapping_add((k as u32).wrapping_mul(EXP_MSK1)));
    } else {
        let k = -k;
        db.set_word0(db.word0().wrapping_add((k as u32).wrapping_mul(EXP_MSK1)));
    }
    da.d() / db.d()
}

static TENS: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

static BIGTENS: [f64; 5] = [1e16, 1e32, 1e64, 1e128, 1e256];
static TINYTENS: [f64; 5] = [
    1e-16,
    1e-32,
    1e-64,
    1e-128,
    9007199254740992.0 * 9007199254740992.0e-256,
];

static PFIVE: [u64; 27] = [
    5,
    25,
    125,
    625,
    3125,
    15625,
    78125,
    390625,
    1953125,
    9765625,
    48828125,
    244140625,
    1220703125,
    6103515625,
    30517578125,
    152587890625,
    762939453125,
    3814697265625,
    19073486328125,
    95367431640625,
    476837158203125,
    2384185791015625,
    11920928955078125,
    59604644775390625,
    298023223876953125,
    1490116119384765625,
    7450580596923828125,
];

static PFIVEBITS: [c_int; 25] = [
    3, 5, 7, 10, 12, 14, 17, 19, 21, 24, 26, 28, 31, 33, 35, 38, 40, 42, 45, 47, 49, 52, 54, 56, 59,
];

// hexdig table for gethex/hexnan
static HEXDIG: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut i = b'0';
    while i <= b'9' {
        t[i as usize] = 0x10 + (i - b'0');
        i += 1;
    }
    let mut i = b'a';
    while i <= b'f' {
        t[i as usize] = 0x10 + 10 + (i - b'a');
        i += 1;
    }
    let mut i = b'A';
    while i <= b'F' {
        t[i as usize] = 0x10 + 10 + (i - b'A');
        i += 1;
    }
    t
};

fn dshift(b: *mut Bigint, p2: c_int) -> c_int {
    unsafe {
        let mut rv = hi0bits(*bx(b).add(((*b).wds - 1) as usize)) - 4;
        if p2 > 0 {
            rv -= p2;
        }
        rv & KMASK
    }
}

unsafe fn quorem(b: *mut Bigint, s: *mut Bigint) -> c_int {
    let mut n = (*s).wds;
    if (*b).wds < n {
        return 0;
    }
    let sx = bx(s);
    n -= 1;
    let sxe = sx.add(n as usize);
    let bx_ = bx(b);
    let bxe = bx_.add(n as usize);
    let mut q = *bxe / (*sxe + 1);
    if q != 0 {
        let mut borrow: u64 = 0;
        let mut carry: u64 = 0;
        let mut sxp = sx;
        let mut bxp = bx_;
        loop {
            let ys = (*sxp as u64) * (q as u64) + carry;
            sxp = sxp.add(1);
            carry = ys >> 32;
            let y = (*bxp as u64).wrapping_sub(ys & FFFFFFFF).wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            *bxp = (y & FFFFFFFF) as u32;
            bxp = bxp.add(1);
            if sxp > sxe {
                break;
            }
        }
        if *bxe == 0 {
            let bxx = bx(b);
            let mut e = bxe;
            loop {
                e = e.offset(-1);
                if !(e > bxx && *e == 0) {
                    break;
                }
                n -= 1;
            }
            (*b).wds = n;
        }
    }
    if cmp(b, s) >= 0 {
        q += 1;
        let mut borrow: u64 = 0;
        let mut carry: u64 = 0;
        let mut bxp = bx(b);
        let mut sxp = bx(s);
        loop {
            let ys = (*sxp as u64) + carry;
            sxp = sxp.add(1);
            carry = ys >> 32;
            let y = (*bxp as u64).wrapping_sub(ys & FFFFFFFF).wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            *bxp = (y & FFFFFFFF) as u32;
            bxp = bxp.add(1);
            if sxp > sxe {
                break;
            }
        }
        let bxx = bx(b);
        let mut e = bxx.add(n as usize);
        if *e == 0 {
            loop {
                e = e.offset(-1);
                if !(e > bxx && *e == 0) {
                    break;
                }
                n -= 1;
            }
            (*b).wds = n;
        }
    }
    q as c_int
}

// ==== dtoa_result / rv_alloc / nrv_alloc / freedtoa ====
static mut DTOA_RESULT: *mut c_char = ptr::null_mut();

unsafe fn rv_alloc(i: c_int) -> *mut c_char {
    let mut j = core::mem::size_of::<u32>();
    let mut k = 0;
    // for(k=0; sizeof(Bigint)-sizeof(ULong)-sizeof(int)+j <= i; j<<=1) k++;
    let base = core::mem::size_of::<Bigint>() - core::mem::size_of::<u32>() - core::mem::size_of::<c_int>();
    while base + j <= i as usize {
        k += 1;
        j <<= 1;
    }
    let r = balloc(k) as *mut c_int;
    *r = k;
    DTOA_RESULT = r.add(1) as *mut c_char;
    DTOA_RESULT
}

unsafe fn nrv_alloc(s: *const c_char, s0: *mut c_char, s0len: usize, rve: *mut *mut c_char, n: c_int) -> *mut c_char {
    let rv: *mut c_char;
    let mut t: *mut c_char;
    if s0.is_null() {
        let a = rv_alloc(n);
        rv = a;
        t = a;
    } else if s0len <= n as usize {
        let r: *mut c_char = ptr::null_mut();
        t = r.wrapping_add(n as usize);
        if !rve.is_null() {
            *rve = t;
        }
        return r;
    } else {
        rv = s0;
        t = s0;
    }
    let mut sp = s;
    loop {
        let c = *sp;
        *t = c;
        sp = sp.add(1);
        if c == 0 {
            break;
        }
        t = t.add(1);
    }
    if !rve.is_null() {
        *rve = t;
    }
    rv
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freedtoa(s: *mut c_char) {
    let b = (s as *mut c_int).offset(-1) as *mut Bigint;
    (*b).k = *((b as *mut c_int));
    (*b).maxwds = 1 << (*b).k;
    bfree(b);
    if s == DTOA_RESULT {
        DTOA_RESULT = ptr::null_mut();
    }
}

// ==== dtoa_r (USE_BF96 path: exact-integer / 96-bit bigfloat, Bigint fallback) ====

// Shift helpers matching C's behaviour on x86-64 (shift counts are taken mod 64).
#[inline]
fn sl(x: u64, n: c_int) -> u64 {
    x.wrapping_shl(n as u32)
}
#[inline]
fn sr(x: u64, n: c_int) -> u64 {
    x.wrapping_shr(n as u32)
}

// pfive[i]; C indexes pfive[k-1] with k == 0 in one (practically unreachable)
// spot, which reads out of bounds.  We return 0 there instead of reading OOB.
#[inline]
fn pfive_at(i: c_int) -> u64 {
    if i >= 0 && (i as usize) < PFIVE.len() {
        PFIVE[i as usize]
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa_r(
    dd: f64,
    mut mode: c_int,
    mut ndigits: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
    buf_in: *mut c_char,
    blen_in: usize,
) -> *mut c_char {
    // Labels of the C source, emulated as an explicit state machine.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum St {
        Sect1,
        UseExact,
        UseExact1,
        NoDiv,
        UlpReached,
        Roundup,
        Toobig,
        FastFailed,
        FastFailed1,
        NoDigits,
        OneDigit,
        Round9Up,
        AcceptDig,
        Roundoff,
        Ret,
        Retc,
        Ret1,
    }

    let mut bbits: c_int = 0;
    let mut b2: c_int = 0;
    let mut b5: c_int = 0;
    let mut be: c_int;
    let mut dig: c_int = 0;
    let mut i: c_int;
    let mut ilim: c_int;
    let mut ilim1: c_int;
    let mut j: c_int;
    let mut j1: c_int = 0;
    let mut k: c_int;
    let mut leftright: c_int;
    let mut m2: c_int = 0;
    let mut m5: c_int = 0;
    let mut s2: c_int = 0;
    let mut s5: c_int = 0;
    let mut spec_case: c_int;
    let denorm: c_int;
    let mut b: *mut Bigint;
    let mut b1: *mut Bigint = ptr::null_mut();
    let mut delta: *mut Bigint;
    let mut mlo: *mut Bigint = ptr::null_mut();
    let mut mhi: *mut Bigint = ptr::null_mut();
    let mut sbig: *mut Bigint = ptr::null_mut(); /* "S" in the C source */
    let mut u = U::new();
    let mut s: *mut c_char;
    let mut p10: usize = 0; /* index into PTEN, i.e. C's "BF96 *p10" */
    let dbhi: u64;
    let mut dbits: u64;
    let dblo: u64;
    let mut den: u64 = 0;
    let mut hb: u64;
    let mut rb: u64 = 0;
    let mut rblo: u64 = 0;
    let mut res: u64 = 0;
    let mut res0: u64 = 0;
    let mut res3: u64 = 0;
    let mut reslo: u64 = 0;
    let mut sres: u64 = 0;
    let mut sulp: u64;
    let mut tv0: u64 = 0;
    let mut tv1: u64 = 0;
    let mut tv2: u64 = 0;
    let mut tv3: u64 = 0;
    let mut ulpv: u64 = 0; /* "ulp" in the C source */
    let mut ulplo: u64 = 0;
    let mut ulpmask: u64 = 0;
    let mut ures: u64 = 0;
    let mut ureslo: u64 = 0;
    let mut zb: u64;
    let mut eulp: c_int = 0;
    let mut k1: c_int;
    let mut n2: c_int;
    let ulpadj: c_int;
    let mut ulpshift: c_int = 0;

    let mut buf = buf_in;
    let mut blen = blen_in;

    u.set_d(dd);
    if u.word0() & 0x80000000 != 0 {
        *sign = 1;
        u.set_word0(u.word0() & !0x80000000);
    } else {
        *sign = 0;
    }
    if (u.word0() & 0x7ff00000) == 0x7ff00000 {
        *decpt = 9999;
        if u.word1() == 0 && (u.word0() & 0xfffff) == 0 {
            return nrv_alloc(b"Infinity\0".as_ptr() as *const c_char, buf, blen, rve, 8);
        }
        return nrv_alloc(b"NaN\0".as_ptr() as *const c_char, buf, blen, rve, 3);
    }
    if u.d() == 0.0 {
        *decpt = 1;
        return nrv_alloc(b"0\0".as_ptr() as *const c_char, buf, blen, rve, 1);
    }

    dbits = (u.ll() & 0xfffffffffffff) << 11; /* fraction bits */
    be = (u.ll() >> 52) as c_int; /* biased exponent; nonzero ==> normal */
    if be != 0 {
        dbits |= 0x8000000000000000;
        ulpadj = 0;
        denorm = 0;
    } else {
        denorm = 1;
        let mut ua = be + 1;
        dbits <<= 1;
        if dbits & 0xffffffff00000000 == 0 {
            dbits <<= 32;
            be -= 32;
        }
        if dbits & 0xffff000000000000 == 0 {
            dbits <<= 16;
            be -= 16;
        }
        if dbits & 0xff00000000000000 == 0 {
            dbits <<= 8;
            be -= 8;
        }
        if dbits & 0xf000000000000000 == 0 {
            dbits <<= 4;
            be -= 4;
        }
        if dbits & 0xc000000000000000 == 0 {
            dbits <<= 2;
            be -= 2;
        }
        if dbits & 0x8000000000000000 == 0 {
            dbits <<= 1;
            be -= 1;
        }
        ua -= be;
        ulpadj = ua;
    }
    j = LHINT[(be + 51) as usize] as c_int;
    p10 = j as usize;
    dbhi = dbits >> 32;
    dblo = dbits & 0xffffffff;
    i = be - 0x3fe;
    if i < PTEN[p10].e
        || (i == PTEN[p10].e
            && (dbhi < PTEN[p10].b0 as u64
                || (dbhi == PTEN[p10].b0 as u64 && dblo < PTEN[p10].b1 as u64)))
    {
        j -= 1;
    }
    k = j - 342;

    /* now 10^k <= dd < 10^(k+1) */

    if mode < 0 || mode > 9 {
        mode = 0;
    }
    if mode > 5 {
        mode -= 4;
    }
    leftright = 1;
    ilim = -1;
    ilim1 = -1;
    match mode {
        0 | 1 => {
            i = 18;
            ndigits = 0;
        }
        2 | 4 => {
            if mode == 2 {
                leftright = 0;
            }
            if ndigits <= 0 {
                ndigits = 1;
            }
            ilim = ndigits;
            ilim1 = ndigits;
            i = ndigits;
        }
        3 | 5 => {
            if mode == 3 {
                leftright = 0;
            }
            // `wrapping_*`, not `+`/`-`: the C is `i = ndigits + k + 1; ilim = i;
            // ilim1 = i - 1;` on plain `int`s, and `ndigits` is a caller-supplied
            // argument of dtoa_r()/dtoa() (both exported).  With ndigits near
            // INT_MAX the sum wraps in the C (gcc/x86-64) to a negative value,
            // which then trips the `if (i <= 0) i = 1;` clamp below; `i - 1`
            // likewise wraps when i == INT_MIN.  Rust's `+`/`-` would instead
            // panic under overflow-checks and abort across the FFI boundary.
            i = ndigits.wrapping_add(k).wrapping_add(1);
            ilim = i;
            ilim1 = i.wrapping_sub(1);
            if i <= 0 {
                i = 1;
            }
        }
        _ => {}
    }
    if buf.is_null() {
        buf = rv_alloc(i);
        let kk = *((buf as *mut c_int).offset(-1));
        blen = core::mem::size_of::<Bigint>() + (((1i64 << kk) - 1) as usize) * 4
            - core::mem::size_of::<c_int>();
    } else if blen <= i as usize {
        buf = ptr::null_mut();
        if !rve.is_null() {
            *rve = buf.wrapping_add(i as usize);
        }
        return buf;
    }
    s = buf;

    /* Check for special case that d is a normalized power of 2. */

    spec_case = 0;
    if mode < 2 || leftright != 0 {
        if u.word1() == 0
            && (u.word0() & 0xfffff) == 0
            && (u.word0() & (0x7ff00000 & !0x100000)) != 0
        {
            spec_case = 1;
        }
    }

    b = ptr::null_mut();
    let mut st;
    if ilim < 0 && (mode == 3 || mode == 5) {
        sbig = ptr::null_mut();
        mhi = ptr::null_mut();
        st = St::NoDigits;
    } else {
        st = St::Sect1;
    }

    'sm: loop {
        match st {
            St::Sect1 => {
                i = 1;
                j = 52 + 0x3ff - be;
                ulpshift = 0;
                ulplo = 0;
                /* Can we do an exact computation with 64-bit integer arithmetic? */
                if k < 0 {
                    if k < -25 {
                        st = St::Toobig;
                        continue 'sm;
                    }
                    res = dbits >> 11;
                    k1 = -(k + 1);
                    n2 = PFIVEBITS[k1 as usize] + 53;
                    j1 = j;
                    if n2 > 61 {
                        ulpshift = n2 - 61;
                        ulpmask = sl(1, ulpshift).wrapping_sub(1);
                        if res & ulpmask != 0 {
                            st = St::Toobig;
                            continue 'sm;
                        }
                        j -= ulpshift;
                        res = sr(res, ulpshift);
                    }
                    /* Yes. */
                    ulpv = PFIVE[k1 as usize];
                    res = res.wrapping_mul(ulpv);
                    if ulpshift != 0 {
                        ulplo = ulpv;
                        ulpv = sr(ulpv, ulpshift);
                    }
                    j += k;
                    if ilim == 0 {
                        sbig = ptr::null_mut();
                        mhi = ptr::null_mut();
                        st = if res > sl(5, j) {
                            St::OneDigit
                        } else {
                            St::NoDigits
                        };
                        continue 'sm;
                    }
                    st = St::NoDiv;
                    continue 'sm;
                }
                if ilim == 0 && j + k >= 0 {
                    sbig = ptr::null_mut();
                    mhi = ptr::null_mut();
                    st = if (dbits >> 11) > sl(pfive_at(k - 1), j) {
                        St::OneDigit
                    } else {
                        St::NoDigits
                    };
                    continue 'sm;
                }
                if k <= dtoa_divmax && j + k >= 0 {
                    /* Another "yes" case -- we will use exact integer arithmetic. */
                    st = St::UseExact;
                    continue 'sm;
                }
                st = St::Toobig;
                continue 'sm;
            }

            St::UseExact1 => {
                s = buf;
                i = 1;
                st = St::UseExact;
                continue 'sm;
            }

            St::UseExact => {
                res = dbits >> 11; /* residual */
                ulpv = 1;
                if k <= 0 {
                    st = St::NoDiv;
                    continue 'sm;
                }
                j1 = j + k + 1;
                den = sl(pfive_at(k - i), j1 - i);
                loop {
                    dig = (res / den) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res = res.wrapping_sub((dig as u64).wrapping_mul(den));
                    if res == 0 {
                        st = St::Retc;
                        continue 'sm;
                    }
                    if ilim < 0 {
                        ures = den.wrapping_sub(res);
                        if res.wrapping_mul(2) <= ulpv
                            && (if spec_case != 0 {
                                res.wrapping_mul(4) <= ulpv
                            } else {
                                res.wrapping_mul(2) < ulpv || dig & 1 != 0
                            })
                        {
                            st = St::UlpReached;
                            continue 'sm;
                        }
                        if ures.wrapping_mul(2) < ulpv {
                            st = St::Roundup;
                            continue 'sm;
                        }
                    } else if i == ilim {
                        /* switch(Rounding) with Rounding == 1: no case matches */
                        ures = res.wrapping_mul(2);
                        if ures > den
                            || (ures == den && dig & 1 != 0)
                            || (spec_case != 0 && res <= ulpv && res.wrapping_mul(2) >= ulpv)
                        {
                            st = St::Roundup;
                            continue 'sm;
                        }
                        st = St::Retc;
                        continue 'sm;
                    }
                    i += 1;
                    if j1 < i {
                        res = res.wrapping_mul(10);
                        ulpv = ulpv.wrapping_mul(10);
                    } else {
                        if i > k {
                            st = St::NoDiv;
                            continue 'sm;
                        }
                        den = sl(pfive_at(k - i), j1 - i);
                    }
                }
            }

            St::NoDiv => loop {
                den = sr(res, j);
                dig = den as c_int;
                *s = (b'0' as c_int + dig) as c_char;
                s = s.add(1);
                res = res.wrapping_sub(sl(den, j));
                if res == 0 {
                    st = St::Retc;
                    continue 'sm;
                }
                if ilim < 0 {
                    ures = sl(1, j).wrapping_sub(res);
                    if res.wrapping_mul(2) <= ulpv
                        && (if spec_case != 0 {
                            res.wrapping_mul(4) <= ulpv
                        } else {
                            res.wrapping_mul(2) < ulpv || dig & 1 != 0
                        })
                    {
                        st = St::UlpReached;
                        continue 'sm;
                    }
                    if ures.wrapping_mul(2) < ulpv {
                        st = St::Roundup;
                        continue 'sm;
                    }
                }
                j -= 1;
                if i == ilim {
                    hb = sl(1, j);
                    if res & hb != 0 && (dig & 1 != 0 || res & hb.wrapping_sub(1) != 0) {
                        st = St::Roundup;
                        continue 'sm;
                    }
                    if spec_case != 0 && res <= ulpv && res.wrapping_mul(2) >= ulpv {
                        st = St::Roundup;
                        continue 'sm;
                    }
                    st = St::Retc;
                    continue 'sm;
                }
                i += 1;
                res = res.wrapping_mul(5);
                if ulpshift != 0 {
                    ulplo = (ulplo & ulpmask).wrapping_mul(5);
                    ulpv = ulpv.wrapping_mul(5).wrapping_add(sr(ulplo, ulpshift));
                } else {
                    ulpv = ulpv.wrapping_mul(5);
                }
            },

            St::UlpReached => {
                st = if ures < res || (ures == res && dig & 1 != 0) {
                    St::Roundup
                } else {
                    St::Retc
                };
                continue 'sm;
            }

            St::Roundup => {
                let mut jumped = false;
                loop {
                    s = s.offset(-1);
                    if *s == b'9' as c_char {
                        if s == buf {
                            k += 1;
                            *s = b'1' as c_char;
                            s = s.add(1);
                            jumped = true;
                            break;
                        }
                        continue;
                    }
                    break;
                }
                if !jumped {
                    *s += 1;
                    s = s.add(1);
                }
                st = St::Ret1;
                continue 'sm;
            }

            St::Toobig => {
                if ilim > 28 {
                    st = St::FastFailed1;
                    continue 'sm;
                }
                /* Scale by 10^-k */
                p10 = (342 - k) as usize;
                let pb0 = PTEN[p10].b0 as u64;
                let pb1 = PTEN[p10].b1 as u64;
                let pb2 = PTEN[p10].b2 as u64;
                tv0 = pb2.wrapping_mul(dblo);
                tv1 = pb1.wrapping_mul(dblo).wrapping_add(tv0 >> 32);
                tv2 = pb2.wrapping_mul(dbhi).wrapping_add(tv1 & 0xffffffff);
                tv3 = pb0
                    .wrapping_mul(dblo)
                    .wrapping_add(tv1 >> 32)
                    .wrapping_add(tv2 >> 32);
                res3 = pb1.wrapping_mul(dbhi).wrapping_add(tv3 & 0xffffffff);
                res = pb0
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv3 >> 32)
                    .wrapping_add(res3 >> 32);
                be += PTEN[p10].e - 0x3fe;
                j1 = be - 54 + ulpadj;
                eulp = j1;
                if res & 0x8000000000000000 == 0 {
                    be -= 1;
                    res3 <<= 1;
                    res = (res << 1) | ((res3 & 0x100000000) >> 32);
                }
                res0 = res; /* save for Fast_failed */
                if ilim > 19 {
                    st = St::FastFailed;
                    continue 'sm;
                }
                res = sr(res, 4 - be);
                ulpv = pb0; /* ulp */
                ulpv = (ulpv << 29) | (pb1 >> 3);
                /* scaled ulp = ulp * 2^(eulp - 60); 61 bits maintained */
                if ilim == 0 {
                    if res & 0x7fffffffffffffe == 0 || (!res) & 0x7fffffffffffffe == 0 {
                        st = St::FastFailed1;
                        continue 'sm;
                    }
                    sbig = ptr::null_mut();
                    mhi = ptr::null_mut();
                    st = if res >= 0x5000000000000000 {
                        St::OneDigit
                    } else {
                        St::NoDigits
                    };
                    continue 'sm;
                }
                rb = 1; /* upper bound on rounding error */
                loop {
                    dig = (res >> 60) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res &= 0xfffffffffffffff;
                    if ilim < 0 {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if eulp > 0 {
                            sulp = sl(ulpv, eulp - 1);
                            if res <= ures {
                                if res.wrapping_add(rb) > ures.wrapping_sub(rb) {
                                    st = St::FastFailed;
                                    continue 'sm;
                                }
                                if res < sulp {
                                    st = St::Retc;
                                    continue 'sm;
                                }
                            } else {
                                if res.wrapping_sub(rb) <= ures.wrapping_add(rb) {
                                    st = St::FastFailed;
                                    continue 'sm;
                                }
                                if ures < sulp {
                                    st = St::Roundup;
                                    continue 'sm;
                                }
                            }
                        } else {
                            zb = sl(1, eulp + 63).wrapping_neg();
                            if zb & res == 0 {
                                sres = sl(res, 1 - eulp);
                                if sres < ulpv && (spec_case == 0 || sres.wrapping_mul(2) < ulpv) {
                                    if sl(res.wrapping_add(rb), 1 - eulp) >= ulpv {
                                        st = St::FastFailed;
                                        continue 'sm;
                                    }
                                    if ures < res {
                                        if ures.wrapping_add(rb) >= res.wrapping_sub(rb) {
                                            st = St::FastFailed;
                                            continue 'sm;
                                        }
                                        st = St::Roundup;
                                        continue 'sm;
                                    }
                                    if ures.wrapping_sub(rb) < res.wrapping_add(rb) {
                                        st = St::FastFailed;
                                        continue 'sm;
                                    }
                                    st = St::Retc;
                                    continue 'sm;
                                }
                            }
                            if zb & ures == 0 && sl(ures, -eulp) < ulpv {
                                if sl(ures, 1 - eulp) < ulpv {
                                    st = St::Roundup;
                                    continue 'sm;
                                }
                                st = St::FastFailed;
                                continue 'sm;
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if ures < res {
                            if ures <= rb || res.wrapping_sub(rb) <= ures.wrapping_add(rb) {
                                if j + k >= 0 && k >= 0 && k <= 27 {
                                    st = St::UseExact1;
                                    continue 'sm;
                                }
                                st = St::FastFailed;
                                continue 'sm;
                            }
                            st = St::Roundup;
                            continue 'sm;
                        }
                        if res <= rb || ures.wrapping_sub(rb) <= res.wrapping_add(rb) {
                            if j + k >= 0 && k >= 0 && k <= 27 {
                                st = St::UseExact1;
                                continue 'sm;
                            }
                            st = St::FastFailed;
                            continue 'sm;
                        }
                        st = St::Retc;
                        continue 'sm;
                    }
                    rb = rb.wrapping_mul(10);
                    if rb >= 0x1000000000000000 {
                        st = St::FastFailed;
                        continue 'sm;
                    }
                    res = res.wrapping_mul(10);
                    ulpv = ulpv.wrapping_mul(5);
                    if ulpv & 0x8000000000000000 != 0 {
                        eulp += 4;
                        ulpv >>= 3;
                    } else {
                        eulp += 3;
                        ulpv >>= 2;
                    }
                    i += 1;
                }
            }

            St::FastFailed => {
                s = buf;
                i = 4 - be;
                res = sr(res0, i);
                reslo = 0xffffffff & res3;
                if i != 0 {
                    reslo = (sl(res0, 64 - i) >> 32) | sr(reslo, i);
                }
                rb = 0;
                rblo = 4; /* roundoff bound */
                ulpv = PTEN[p10].b0 as u64; /* ulp */
                ulpv = (ulpv << 29) | ((PTEN[p10].b1 as u64) >> 3);
                eulp = j1;
                i = 1;
                loop {
                    dig = (res >> 60) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res &= 0xfffffffffffffff;
                    if ilim < 0 {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        ureslo = 0;
                        if reslo != 0 {
                            ureslo = 0x100000000u64.wrapping_sub(reslo);
                            ures = ures.wrapping_sub(1);
                        }
                        if eulp > 0 {
                            sulp = sl(ulpv, eulp - 1).wrapping_sub(rb);
                            if res <= ures {
                                if res < sulp {
                                    if res.wrapping_add(rb) < ures.wrapping_sub(rb) {
                                        st = St::Retc;
                                        continue 'sm;
                                    }
                                }
                            } else if ures < sulp {
                                if res.wrapping_sub(rb) > ures.wrapping_add(rb) {
                                    st = St::Roundup;
                                    continue 'sm;
                                }
                            }
                            st = St::FastFailed1;
                            continue 'sm;
                        } else {
                            zb = sl(1, eulp + 60).wrapping_neg();
                            'blk: {
                                if zb & res.wrapping_add(rb) == 0 {
                                    sres = sl(res.wrapping_sub(rb), 1 - eulp);
                                    if sres < ulpv
                                        && (spec_case == 0 || sres.wrapping_mul(2) < ulpv)
                                    {
                                        sres = sl(res, 1 - eulp);
                                        j = eulp + 31;
                                        if j > 0 {
                                            sres = sres
                                                .wrapping_add(sr(rblo.wrapping_add(reslo), j));
                                        } else {
                                            sres = sres
                                                .wrapping_add(sl(rblo.wrapping_add(reslo), -j));
                                        }
                                        if sres.wrapping_add(sl(rb, 1 - eulp)) >= ulpv {
                                            st = St::FastFailed1;
                                            continue 'sm;
                                        }
                                        if sres >= ulpv {
                                            break 'blk; /* goto more96 */
                                        }
                                        if ures < res || (ures == res && ureslo < reslo) {
                                            if ures.wrapping_add(rb) >= res.wrapping_sub(rb) {
                                                st = St::FastFailed1;
                                                continue 'sm;
                                            }
                                            st = St::Roundup;
                                            continue 'sm;
                                        }
                                        if ures.wrapping_sub(rb) <= res.wrapping_add(rb) {
                                            st = St::FastFailed1;
                                            continue 'sm;
                                        }
                                        st = St::Retc;
                                        continue 'sm;
                                    }
                                }
                                if zb & ures == 0
                                    && sl(ures.wrapping_sub(rb), 1 - eulp) < ulpv
                                {
                                    if sl(ures.wrapping_add(rb), 1 - eulp) < ulpv {
                                        st = St::Roundup;
                                        continue 'sm;
                                    }
                                    st = St::FastFailed1;
                                    continue 'sm;
                                }
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        sres = 0;
                        ureslo = 0;
                        if reslo != 0 {
                            ureslo = 0x100000000u64.wrapping_sub(reslo);
                            ures = ures.wrapping_sub(1);
                            sres = reslo.wrapping_add(rblo) >> 31;
                        }
                        sres = sres.wrapping_add(rb.wrapping_mul(2));
                        if ures <= res {
                            if ures <= sres || res.wrapping_sub(ures) <= sres {
                                st = St::FastFailed1;
                                continue 'sm;
                            }
                            st = St::Roundup;
                            continue 'sm;
                        }
                        if res <= sres || ures.wrapping_sub(res) <= sres {
                            st = St::FastFailed1;
                            continue 'sm;
                        }
                        st = St::Retc;
                        continue 'sm;
                    }
                    /* more96: */
                    rblo = rblo.wrapping_mul(10);
                    rb = rb.wrapping_mul(10).wrapping_add(rblo >> 32);
                    rblo &= 0xffffffff;
                    if rb >= 0x1000000000000000 {
                        st = St::FastFailed1;
                        continue 'sm;
                    }
                    reslo = reslo.wrapping_mul(10);
                    res = res.wrapping_mul(10).wrapping_add(reslo >> 32);
                    reslo &= 0xffffffff;
                    ulpv = ulpv.wrapping_mul(5);
                    if ulpv & 0x8000000000000000 != 0 {
                        eulp += 4;
                        ulpv >>= 3;
                    } else {
                        eulp += 3;
                        ulpv >>= 2;
                    }
                    i += 1;
                }
            }

            St::FastFailed1 => {
                sbig = ptr::null_mut();
                mhi = ptr::null_mut();
                mlo = ptr::null_mut();
                b = d2b(&u, &mut be, &mut bbits);
                s = buf;
                i = (u.word0() >> 20 & (0x7ff00000u32 >> 20)) as c_int;
                i -= 1023;
                if ulpadj != 0 {
                    i -= ulpadj - 1;
                }
                j = bbits - i - 1;
                if j >= 0 {
                    b2 = 0;
                    s2 = j;
                } else {
                    b2 = -j;
                    s2 = 0;
                }
                if k >= 0 {
                    b5 = 0;
                    s5 = k;
                    s2 += k;
                } else {
                    b2 -= k;
                    b5 = -k;
                    s5 = 0;
                }

                m2 = b2;
                m5 = b5;
                mhi = ptr::null_mut();
                mlo = ptr::null_mut();
                if leftright != 0 {
                    i = if denorm != 0 {
                        be + (1023 + (53 - 1) - 1 + 1)
                    } else {
                        1 + 53 - bbits
                    };
                    b2 += i;
                    s2 += i;
                    mhi = i2b(1);
                }
                if m2 > 0 && s2 > 0 {
                    i = if m2 < s2 { m2 } else { s2 };
                    b2 -= i;
                    m2 -= i;
                    s2 -= i;
                }
                if b5 > 0 {
                    if leftright != 0 {
                        if m5 > 0 {
                            mhi = pow5mult(mhi, m5);
                            b1 = mult(mhi, b);
                            bfree(b);
                            b = b1;
                        }
                        j = b5 - m5;
                        if j != 0 {
                            b = pow5mult(b, j);
                        }
                    } else {
                        b = pow5mult(b, b5);
                    }
                }
                sbig = i2b(1);
                if s5 > 0 {
                    sbig = pow5mult(sbig, s5);
                }
                if spec_case != 0 {
                    b2 += 1;
                    s2 += 1;
                }
                i = dshift(sbig, s2);
                b2 += i;
                m2 += i;
                s2 += i;
                if b2 > 0 {
                    b = lshift(b, b2);
                }
                if s2 > 0 {
                    sbig = lshift(sbig, s2);
                }
                if ilim <= 0 && (mode == 3 || mode == 5) {
                    if ilim < 0 {
                        st = St::NoDigits;
                        continue 'sm;
                    }
                    sbig = multadd(sbig, 5, 0);
                    if cmp(b, sbig) <= 0 {
                        st = St::NoDigits;
                        continue 'sm;
                    }
                    st = St::OneDigit;
                    continue 'sm;
                }
                if leftright != 0 {
                    if m2 > 0 {
                        mhi = lshift(mhi, m2);
                    }
                    mlo = mhi;
                    if spec_case != 0 {
                        mhi = balloc((*mlo).k);
                        bcopy(mhi, mlo);
                        mhi = lshift(mhi, 1);
                    }
                    i = 1;
                    loop {
                        dig = quorem(b, sbig) + b'0' as c_int;
                        j = cmp(b, mlo);
                        delta = diff(sbig, mhi);
                        j1 = if (*delta).sign != 0 { 1 } else { cmp(b, delta) };
                        bfree(delta);
                        if j1 == 0 && mode != 1 && (u.word1() & 1) == 0 {
                            if dig == b'9' as c_int {
                                st = St::Round9Up;
                                continue 'sm;
                            }
                            if j > 0 {
                                dig += 1;
                            }
                            *s = dig as c_char;
                            s = s.add(1);
                            st = St::Ret;
                            continue 'sm;
                        }
                        if j < 0 || (j == 0 && mode != 1 && (u.word1() & 1) == 0) {
                            if *bx(b).add(0) == 0 && (*b).wds <= 1 {
                                st = St::AcceptDig;
                                continue 'sm;
                            }
                            if j1 > 0 {
                                b = lshift(b, 1);
                                j1 = cmp(b, sbig);
                                if (j1 > 0 || (j1 == 0 && dig & 1 != 0)) && {
                                    let old = dig;
                                    dig += 1;
                                    old == b'9' as c_int
                                } {
                                    st = St::Round9Up;
                                    continue 'sm;
                                }
                            }
                            st = St::AcceptDig;
                            continue 'sm;
                        }
                        if j1 > 0 {
                            if dig == b'9' as c_int {
                                st = St::Round9Up;
                                continue 'sm;
                            }
                            *s = (dig + 1) as c_char;
                            s = s.add(1);
                            st = St::Ret;
                            continue 'sm;
                        }
                        *s = dig as c_char;
                        s = s.add(1);
                        if i == ilim {
                            break;
                        }
                        b = multadd(b, 10, 0);
                        if mlo == mhi {
                            mhi = multadd(mhi, 10, 0);
                            mlo = mhi;
                        } else {
                            mlo = multadd(mlo, 10, 0);
                            mhi = multadd(mhi, 10, 0);
                        }
                        i += 1;
                    }
                } else {
                    i = 1;
                    loop {
                        dig = quorem(b, sbig) + b'0' as c_int;
                        *s = dig as c_char;
                        s = s.add(1);
                        if *bx(b).add(0) == 0 && (*b).wds <= 1 {
                            st = St::Ret;
                            continue 'sm;
                        }
                        if i >= ilim {
                            break;
                        }
                        b = multadd(b, 10, 0);
                        i += 1;
                    }
                }

                /* Round off last digit */
                b = lshift(b, 1);
                j = cmp(b, sbig);
                st = if j > 0 || (j == 0 && dig & 1 != 0) {
                    St::Roundoff
                } else {
                    St::Ret
                };
                continue 'sm;
            }

            St::Round9Up => {
                *s = b'9' as c_char;
                s = s.add(1);
                st = St::Roundoff;
                continue 'sm;
            }

            St::AcceptDig => {
                *s = dig as c_char;
                s = s.add(1);
                st = St::Ret;
                continue 'sm;
            }

            St::NoDigits => {
                k = -1 - ndigits;
                st = St::Ret;
                continue 'sm;
            }

            St::OneDigit => {
                *s = b'1' as c_char;
                s = s.add(1);
                k += 1;
                st = St::Ret;
                continue 'sm;
            }

            St::Roundoff => {
                let mut jumped = false;
                loop {
                    s = s.offset(-1);
                    if *s == b'9' as c_char {
                        if s == buf {
                            k += 1;
                            *s = b'1' as c_char;
                            s = s.add(1);
                            jumped = true;
                            break;
                        }
                        continue;
                    }
                    break;
                }
                if !jumped {
                    *s += 1;
                    s = s.add(1);
                }
                st = St::Ret;
                continue 'sm;
            }

            St::Ret => {
                bfree(sbig);
                if !mhi.is_null() {
                    if !mlo.is_null() && mlo != mhi {
                        bfree(mlo);
                    }
                    bfree(mhi);
                }
                st = St::Retc;
                continue 'sm;
            }

            St::Retc => {
                while s > buf && *s.offset(-1) == b'0' as c_char {
                    s = s.offset(-1);
                }
                st = St::Ret1;
                continue 'sm;
            }

            St::Ret1 => {
                if !b.is_null() {
                    bfree(b);
                }
                *s = 0;
                // `wrapping_add`, not `+`: the C is `*decpt = k + 1;` on `int`.
                // The `no_digits` path above sets `k = -1 - ndigits`, so a caller
                // passing ndigits == INT_MIN to dtoa_r()/dtoa() (both exported)
                // reaches here with k == INT_MAX; the C wraps to INT_MIN and
                // reports *decpt == -2147483648.  Rust's `+` panicked instead.
                *decpt = k.wrapping_add(1);
                if !rve.is_null() {
                    *rve = s;
                }
                return buf;
            }
        }
    }
}


#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa(
    dd: f64,
    mode: c_int,
    ndigits: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
) -> *mut c_char {
    if !DTOA_RESULT.is_null() {
        freedtoa(DTOA_RESULT);
    }
    dtoa_r(dd, mode, ndigits, decpt, sign, rve, ptr::null_mut(), 0)
}

// gethex and strtod__unused are provided in dtoa_strtod.rs (same crate module tree).
include!("dtoa_strtod.rs");
