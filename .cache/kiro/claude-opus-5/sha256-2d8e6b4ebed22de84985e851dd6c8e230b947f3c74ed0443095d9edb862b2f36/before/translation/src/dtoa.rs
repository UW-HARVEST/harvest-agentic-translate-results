//! Translation of the reachable part of `src/dtoa.c` (David M. Gay's dtoa).
//!
//! Configuration as used by this build of jansson:
//!   `IEEE_8087`, `USE_BF96` (long long available, `NO_BF96` unset),
//!   `Omit_Private_Memory` unset, `MULTIPLE_THREADS` unset, `DEBUG` unset
//!   (so all `assert()`s expand to nothing), `Honor_FLT_ROUNDS` unset,
//!   `SET_INEXACT` unset, `ROUND_BIASED` unset, `Sudden_Underflow` unset and
//!   `Rounding == Flt_Rounds == FLT_ROUNDS == 1` (gcc's float.h).
//!
//! Only `dtoa_r()` is provided; `strtod()` lives in libc and the `dtoa()` /
//! `freedtoa()` pair is not used by jansson.

use crate::dtoa_tables::{Bf96, LHINT, PFIVE, PFIVEBITS, PTEN};
use crate::memory::jsonp_malloc;
use core::ffi::{c_char, c_int};

/* IEEE_Arith constants */
const EXP_SHIFT: i32 = 20;
const EXP_SHIFT1: i32 = 20;
const EXP_MSK1: u32 = 0x100000;
const EXP_MASK: u32 = 0x7ff00000;
const P: i32 = 53;
const BIAS: i32 = 1023;
const FRAC_MASK: u32 = 0xfffff;
const BNDRY_MASK: u32 = 0xfffff;
const SIGN_BIT: u32 = 0x80000000;
const LOG2P: i32 = 1;

/// `int dtoa_divmax = 2;`
#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

/* ----------------------------------------------------------------- shifts */
/* x86 shift semantics (count taken modulo the operand width), which is what
   the C code relies on for its out-of-range shift counts. */

#[inline(always)]
fn shl(x: u64, n: i32) -> u64 {
    x.wrapping_shl(n as u32)
}

#[inline(always)]
fn shr(x: u64, n: i32) -> u64 {
    x.wrapping_shr(n as u32)
}

/* ----------------------------------------------------------------- Bigint */

#[derive(Clone)]
struct Bigint {
    k: i32,
    maxwds: i32,
    sign: i32,
    wds: i32,
    x: Vec<u32>,
}

fn balloc(k: i32) -> Bigint {
    let x = 1i32 << k;
    Bigint {
        k,
        maxwds: x,
        sign: 0,
        wds: 0,
        x: vec![0u32; x as usize],
    }
}

/// `Bcopy(x, y)`: copies `sign`, `wds` and `y->wds` words of `x[]`.
fn bcopy(dst: &mut Bigint, src: &Bigint) {
    dst.sign = src.sign;
    dst.wds = src.wds;
    for i in 0..src.wds as usize {
        dst.x[i] = src.x[i];
    }
}

fn multadd(mut b: Bigint, m: i32, a: i32) -> Bigint {
    let mut wds = b.wds;
    let mut i = 0;
    let mut carry: u64 = a as u32 as u64;
    loop {
        let y = (b.x[i as usize] as u64)
            .wrapping_mul(m as u32 as u64)
            .wrapping_add(carry);
        carry = y >> 32;
        b.x[i as usize] = (y & 0xffffffff) as u32;
        i += 1;
        if i >= wds {
            break;
        }
    }
    if carry != 0 {
        if wds >= b.maxwds {
            let mut b1 = balloc(b.k + 1);
            bcopy(&mut b1, &b);
            b = b1;
        }
        b.x[wds as usize] = carry as u32;
        wds += 1;
        b.wds = wds;
    }
    b
}

fn hi0bits(mut x: u32) -> i32 {
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

fn lo0bits(y: &mut u32) -> i32 {
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

fn i2b(i: i32) -> Bigint {
    let mut b = balloc(1);
    b.x[0] = i as u32;
    b.wds = 1;
    b
}

fn mult(a: &Bigint, b: &Bigint) -> Bigint {
    let (a, b) = if a.wds < b.wds { (b, a) } else { (a, b) };

    let mut k = a.k;
    let wa = a.wds as usize;
    let wb = b.wds as usize;
    let mut wc = wa + wb;
    if wc as i32 > a.maxwds {
        k += 1;
    }
    let mut c = balloc(k);

    for j in 0..wb {
        let y = b.x[j] as u64;
        if y != 0 {
            let mut carry: u64 = 0;
            for i in 0..wa {
                let z = (a.x[i] as u64)
                    .wrapping_mul(y)
                    .wrapping_add(c.x[i + j] as u64)
                    .wrapping_add(carry);
                carry = z >> 32;
                c.x[i + j] = (z & 0xffffffff) as u32;
            }
            c.x[wa + j] = carry as u32;
        }
    }
    while wc > 0 && c.x[wc - 1] == 0 {
        wc -= 1;
    }
    c.wds = wc as i32;
    c
}

fn pow5mult(mut b: Bigint, mut k: i32) -> Bigint {
    const P05: [i32; 3] = [5, 25, 125];

    let i = k & 3;
    if i != 0 {
        b = multadd(b, P05[(i - 1) as usize], 0);
    }

    k >>= 2;
    if k == 0 {
        return b;
    }

    let mut p5 = i2b(625);
    loop {
        if k & 1 != 0 {
            b = mult(&b, &p5);
        }
        k >>= 1;
        if k == 0 {
            break;
        }
        p5 = mult(&p5, &p5);
    }
    b
}

fn lshift(b: Bigint, k: i32) -> Bigint {
    let n = (k >> 5) as usize;
    let mut k1 = b.k;
    let mut n1 = n as i32 + b.wds + 1;
    let mut i = b.maxwds;
    while n1 > i {
        i <<= 1;
        k1 += 1;
    }
    let mut b1 = balloc(k1);
    let mut idx = n;
    let kk = k & 0x1f;
    if kk != 0 {
        let kr = 32 - kk;
        let mut z: u32 = 0;
        for i in 0..b.wds as usize {
            b1.x[idx] = (b.x[i] << kk) | z;
            idx += 1;
            z = b.x[i] >> kr;
        }
        b1.x[idx] = z;
        if z != 0 {
            n1 += 1;
        }
    } else {
        for i in 0..b.wds as usize {
            b1.x[idx] = b.x[i];
            idx += 1;
        }
    }
    b1.wds = n1 - 1;
    b1
}

fn cmp(a: &Bigint, b: &Bigint) -> i32 {
    let i = a.wds;
    let j = b.wds;
    let d = i - j;
    if d != 0 {
        return d;
    }
    let mut idx = j as usize;
    loop {
        idx -= 1;
        if a.x[idx] != b.x[idx] {
            return if a.x[idx] < b.x[idx] { -1 } else { 1 };
        }
        if idx == 0 {
            break;
        }
    }
    0
}

fn diff(a: &Bigint, b: &Bigint) -> Bigint {
    let mut i = cmp(a, b);
    if i == 0 {
        let mut c = balloc(0);
        c.wds = 1;
        c.x[0] = 0;
        return c;
    }
    let (a, b) = if i < 0 {
        i = 1;
        (b, a)
    } else {
        i = 0;
        (a, b)
    };
    let mut c = balloc(a.k);
    c.sign = i;
    let mut wa = a.wds as usize;
    let wb = b.wds as usize;
    let mut borrow: u64 = 0;

    for idx in 0..wb {
        let y = (a.x[idx] as u64)
            .wrapping_sub(b.x[idx] as u64)
            .wrapping_sub(borrow);
        borrow = (y >> 32) & 1;
        c.x[idx] = (y & 0xffffffff) as u32;
    }
    for idx in wb..a.wds as usize {
        let y = (a.x[idx] as u64).wrapping_sub(borrow);
        borrow = (y >> 32) & 1;
        c.x[idx] = (y & 0xffffffff) as u32;
    }
    let mut idx = wa;
    loop {
        idx -= 1;
        if c.x[idx] != 0 {
            break;
        }
        wa -= 1;
    }
    c.wds = wa as i32;
    c
}

/// `d2b()`; returns `(b, e, bits)`.
fn d2b(ull: u64) -> (Bigint, i32, i32) {
    let mut b = balloc(1);
    let mut d0 = (ull >> 32) as u32;
    let d1 = (ull & 0xffffffff) as u32;
    let e: i32;
    let bits: i32;

    let mut z = d0 & FRAC_MASK;
    d0 &= 0x7fffffff; /* clear sign bit, which we ignore */
    let de = (d0 >> EXP_SHIFT) as i32;
    if de != 0 {
        z |= EXP_MSK1;
    }

    let mut k;
    let i;
    let mut y = d1;
    if y != 0 {
        k = lo0bits(&mut y);
        if k != 0 {
            b.x[0] = y | (z << (32 - k));
            z >>= k;
        } else {
            b.x[0] = y;
        }
        b.x[1] = z;
        b.wds = if z != 0 { 2 } else { 1 };
        i = b.wds;
    } else {
        k = lo0bits(&mut z);
        b.x[0] = z;
        b.wds = 1;
        i = 1;
        k += 32;
    }

    if de != 0 {
        e = de - BIAS - (P - 1) + k;
        bits = P - k;
    } else {
        e = de - BIAS - (P - 1) + 1 + k;
        bits = 32 * i - hi0bits(b.x[(i - 1) as usize]);
    }

    (b, e, bits)
}

fn dshift(b: &Bigint, p2: i32) -> i32 {
    let mut rv = hi0bits(b.x[(b.wds - 1) as usize]) - 4;
    if p2 > 0 {
        rv -= p2;
    }
    rv & 31
}

fn quorem(b: &mut Bigint, s: &Bigint) -> i32 {
    let n_orig = s.wds;
    if b.wds < n_orig {
        return 0;
    }
    let n_top = (n_orig - 1) as usize; /* index of the top word (sxe/bxe) */
    let mut n = n_orig - 1;

    let mut q: u32 = b.x[n_top] / (s.x[n_top] + 1); /* ensure q <= true quotient */

    if q != 0 {
        let mut borrow: u64 = 0;
        let mut carry: u64 = 0;
        for idx in 0..=n_top {
            let ys = (s.x[idx] as u64)
                .wrapping_mul(q as u64)
                .wrapping_add(carry);
            carry = ys >> 32;
            let y = (b.x[idx] as u64)
                .wrapping_sub(ys & 0xffffffff)
                .wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            b.x[idx] = (y & 0xffffffff) as u32;
        }
        if b.x[n_top] == 0 {
            let mut idx = n_top;
            loop {
                if idx == 0 {
                    break;
                }
                idx -= 1;
                if !(idx > 0 && b.x[idx] == 0) {
                    break;
                }
                n -= 1;
            }
            b.wds = n;
        }
    }
    if cmp(b, s) >= 0 {
        q += 1;
        let mut borrow: u64 = 0;
        let mut carry: u64 = 0;
        for idx in 0..=n_top {
            let ys = (s.x[idx] as u64).wrapping_add(carry);
            carry = ys >> 32;
            let y = (b.x[idx] as u64)
                .wrapping_sub(ys & 0xffffffff)
                .wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            b.x[idx] = (y & 0xffffffff) as u32;
        }
        let n_top2 = n as usize;
        if b.x[n_top2] == 0 {
            let mut idx = n_top2;
            loop {
                if idx == 0 {
                    break;
                }
                idx -= 1;
                if !(idx > 0 && b.x[idx] == 0) {
                    break;
                }
                n -= 1;
            }
            b.wds = n;
        }
    }
    q as i32
}

/* ---------------------------------------------------------------- dtoa_r */

/// `nrv_alloc(s, s0, s0len, rve, n)` for the case where `s0 != NULL`.
unsafe fn nrv_alloc(
    src: &[u8],
    s0: *mut c_char,
    s0len: usize,
    rve: *mut *mut c_char,
    n: i32,
) -> *mut c_char {
    let rv: *mut c_char;
    let t: *mut c_char;

    if s0.is_null() {
        /* Not reachable from jansson: dtoa_r() is always called with a buffer. */
        let p = jsonp_malloc(n as usize + 1) as *mut c_char;
        if p.is_null() {
            return core::ptr::null_mut();
        }
        rv = p;
        let mut q = p;
        for &c in src {
            *q = c as c_char;
            q = q.add(1);
        }
        *q = 0;
        if !rve.is_null() {
            *rve = q;
        }
        return rv;
    } else if s0len <= n as usize {
        rv = core::ptr::null_mut();
        t = (n as usize) as *mut c_char;
        if !rve.is_null() {
            *rve = t;
        }
        return rv;
    }

    rv = s0;
    let mut q = s0;
    for &c in src {
        *q = c as c_char;
        q = q.add(1);
    }
    *q = 0;
    if !rve.is_null() {
        *rve = q;
    }
    rv
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum St {
    UseExact,
    NoDiv,
    Toobig,
    FastFailed,
    FastFailed1,
    BigTail,
    Roundup,
    Roundoff,
    NoDigits,
    OneDigit,
    Ret,
    Retc,
    Ret1,
}

/* Several locals mirror C declarations that are only read on code paths which
   this configuration never takes (e.g. `ilim1`, used by the non-USE_BF96 fast
   float path); they are kept so the translation stays line-for-line. */
#[allow(unused_assignments, unused_variables)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa_r(
    dd: f64,
    mode_in: c_int,
    ndigits_in: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
    buf_in: *mut c_char,
    blen_in: usize,
) -> *mut c_char {
    let mut mode = mode_in;
    let mut ndigits = ndigits_in;
    let mut buf = buf_in;
    let mut blen = blen_in;

    let mut ull: u64 = dd.to_bits();
    if (ull >> 32) as u32 & SIGN_BIT != 0 {
        /* set sign for everything, including 0's and NaNs */
        *sign = 1;
        ull &= !((SIGN_BIT as u64) << 32); /* clear sign bit */
    } else {
        *sign = 0;
    }

    if (ull >> 32) as u32 & EXP_MASK == EXP_MASK {
        /* Infinity or NaN */
        *decpt = 9999;
        if (ull & 0xffffffff) == 0 && ((ull >> 32) as u32 & 0xfffff) == 0 {
            return nrv_alloc(b"Infinity", buf, blen, rve, 8);
        }
        return nrv_alloc(b"NaN", buf, blen, rve, 3);
    }
    if f64::from_bits(ull) == 0.0 {
        *decpt = 1;
        return nrv_alloc(b"0", buf, blen, rve, 1);
    }

    let mut dbits: u64 = (ull & 0xfffffffffffff) << 11; /* fraction bits */
    let mut be: i32 = (ull >> 52) as i32; /* biased exponent */
    let denorm: i32;
    let ulpadj: i32;
    if be != 0 {
        dbits |= 0x8000000000000000;
        denorm = 0;
        ulpadj = 0;
    } else {
        denorm = 1;
        let mut adj = be + 1;
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
        adj -= be;
        ulpadj = adj;
    }
    let mut j: i32 = LHINT[(be + 51) as usize] as i32;
    let mut p10: &Bf96 = &PTEN[j as usize];
    let dbhi: u64 = dbits >> 32;
    let dblo: u64 = dbits & 0xffffffff;
    let mut i: i32 = be - 0x3fe;
    if i < p10.e
        || (i == p10.e && (dbhi < p10.b0 as u64 || (dbhi == p10.b0 as u64 && dblo < p10.b1 as u64)))
    {
        j -= 1;
    }
    let mut k: i32 = j - 342;

    /* now 10^k <= dd < 10^(k+1) */

    if mode < 0 || mode > 9 {
        mode = 0;
    }

    if mode > 5 {
        mode -= 4;
    }
    let mut leftright = 1;
    let mut ilim: i32 = -1;
    let mut ilim1: i32 = -1;
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
        _ => {
            /* 3 | 5 */
            if mode == 3 {
                leftright = 0;
            }
            i = ndigits + k + 1;
            ilim = i;
            ilim1 = i - 1;
            if i <= 0 {
                i = 1;
            }
        }
    }
    if buf.is_null() {
        /* rv_alloc(i); jansson always supplies a buffer, so this uses a plain
           allocation instead of dtoa.c's Bigint backed one. */
        let mut jj: usize = core::mem::size_of::<u32>();
        let mut kk = 0;
        while 32 - 4 - 4 + jj <= i as usize {
            jj <<= 1;
            kk += 1;
        }
        blen = 32 + ((1usize << kk) - 1) * 4 - 4;
        buf = jsonp_malloc(blen) as *mut c_char;
        if buf.is_null() {
            return core::ptr::null_mut();
        }
    } else if blen <= i as usize {
        buf = core::ptr::null_mut();
        if !rve.is_null() {
            *rve = (i as usize) as *mut c_char;
        }
        return buf;
    }
    let mut s: usize = 0; /* s - buf */

    /* Check for special case that d is a normalized power of 2. */

    let mut spec_case = false;
    if mode < 2 || leftright != 0 {
        let w0 = (ull >> 32) as u32;
        let w1 = (ull & 0xffffffff) as u32;
        if w1 == 0 && (w0 & BNDRY_MASK) == 0 && (w0 & (EXP_MASK & !EXP_MSK1)) != 0 {
            /* The special case */
            spec_case = true;
        }
    }

    /* Bigint state */
    let mut b: Option<Bigint> = None;
    let mut bs: Option<Bigint> = None; /* S */
    let mut mhi: Option<Bigint> = None;
    let mut mlo: Option<Bigint> = None; /* None => aliases mhi */
    let mut bbits: i32 = 0;
    let mut b2: i32 = 0;
    let mut b5: i32 = 0;
    let mut s2: i32 = 0;
    let mut s5: i32 = 0;

    let mut dig: i32 = 0;
    let mut res: u64 = 0;
    let mut res0: u64 = 0;
    let mut res3: u64 = 0;
    let mut reslo: u64 = 0;
    let mut ures: u64 = 0;
    let mut ureslo: u64 = 0;
    let mut ulp: u64 = 0;
    let mut ulplo: u64 = 0;
    let mut ulpmask: u64 = 0;
    let mut ulpshift: i32 = 0;
    let mut den: u64;
    let mut rb: u64 = 0;
    let mut rblo: u64 = 0;
    let mut eulp: i32 = 0;
    let mut j1: i32 = 0;

    let state: St;

    'linear: {
        if ilim < 0 && (mode == 3 || mode == 5) {
            state = St::NoDigits;
            break 'linear;
        }
        i = 1;
        j = 52 + 0x3ff - be;
        ulpshift = 0;
        ulplo = 0;
        /* Can we do an exact computation with 64-bit integer arithmetic? */
        if k < 0 {
            if k < -25 {
                state = St::Toobig;
                break 'linear;
            }
            res = dbits >> 11;
            let k1 = -(k + 1);
            let n2 = PFIVEBITS[k1 as usize] + 53;
            j1 = j;
            if n2 > 61 {
                ulpshift = n2 - 61;
                ulpmask = shl(1, ulpshift) - 1;
                if res & ulpmask != 0 {
                    state = St::Toobig;
                    break 'linear;
                }
                j -= ulpshift;
                res = shr(res, ulpshift);
            }
            /* Yes. */
            ulp = PFIVE[k1 as usize];
            res = res.wrapping_mul(ulp);
            if ulpshift != 0 {
                ulplo = ulp;
                ulp = shr(ulp, ulpshift);
            }
            j += k;
            if ilim == 0 {
                if res > shl(5, j) {
                    state = St::OneDigit;
                } else {
                    state = St::NoDigits;
                }
                break 'linear;
            }
            state = St::NoDiv;
            break 'linear;
        }
        if ilim == 0 && j + k >= 0 {
            if (dbits >> 11) > shl(PFIVE[(k - 1) as usize], j) {
                state = St::OneDigit;
            } else {
                state = St::NoDigits;
            }
            break 'linear;
        }
        if k <= dtoa_divmax && j + k >= 0 {
            state = St::UseExact;
            break 'linear;
        }
        state = St::Toobig;
    }

    let mut state = state;

    'dispatch: loop {
        match state {
            /* ------------------------------------------------------------ */
            St::UseExact => {
                /* Another "yes" case -- we will use exact integer arithmetic. */
                res = dbits >> 11; /* residual */
                ulp = 1;
                if k <= 0 {
                    state = St::NoDiv;
                    continue 'dispatch;
                }
                j1 = j + k + 1;
                den = shl(PFIVE[(k - i) as usize], j1 - i);
                loop {
                    dig = (res / den) as i32;
                    *buf.add(s) = (b'0' as i32 + dig) as c_char;
                    s += 1;
                    res = res.wrapping_sub((dig as u64).wrapping_mul(den));
                    if res == 0 {
                        state = St::Retc;
                        continue 'dispatch;
                    }
                    if ilim < 0 {
                        ures = den - res;
                        if 2 * res <= ulp
                            && (if spec_case {
                                4 * res <= ulp
                            } else {
                                2 * res < ulp || dig & 1 != 0
                            })
                        {
                            /* goto ulp_reached */
                            if ures < res || (ures == res && dig & 1 != 0) {
                                state = St::Roundup;
                            } else {
                                state = St::Retc;
                            }
                            continue 'dispatch;
                        }
                        if 2 * ures < ulp {
                            state = St::Roundup;
                            continue 'dispatch;
                        }
                    } else if i == ilim {
                        ures = 2 * res;
                        if ures > den
                            || (ures == den && dig & 1 != 0)
                            || (spec_case && res <= ulp && 2 * res >= ulp)
                        {
                            state = St::Roundup;
                        } else {
                            state = St::Retc;
                        }
                        continue 'dispatch;
                    }
                    i += 1;
                    if j1 < i {
                        res = res.wrapping_mul(10);
                        ulp = ulp.wrapping_mul(10);
                    } else {
                        if i > k {
                            break;
                        }
                        den = shl(PFIVE[(k - i) as usize], j1 - i);
                    }
                }
                state = St::NoDiv;
            }

            /* ------------------------------------------------------------ */
            St::NoDiv => {
                loop {
                    den = shr(res, j);
                    dig = den as u32 as i32;
                    *buf.add(s) = (b'0' as i32 + dig) as c_char;
                    s += 1;
                    res = res.wrapping_sub(shl(den, j));
                    if res == 0 {
                        state = St::Retc;
                        continue 'dispatch;
                    }
                    if ilim < 0 {
                        ures = shl(1, j) - res;
                        if 2 * res <= ulp
                            && (if spec_case {
                                4 * res <= ulp
                            } else {
                                2 * res < ulp || dig & 1 != 0
                            })
                        {
                            /* ulp_reached: */
                            if ures < res || (ures == res && dig & 1 != 0) {
                                state = St::Roundup;
                            } else {
                                state = St::Retc;
                            }
                            continue 'dispatch;
                        }
                        if 2 * ures < ulp {
                            state = St::Roundup;
                            continue 'dispatch;
                        }
                    }
                    j -= 1;
                    if i == ilim {
                        let hb = shl(1, j);
                        if res & hb != 0 && (dig & 1 != 0 || res & (hb - 1) != 0) {
                            state = St::Roundup;
                            continue 'dispatch;
                        }
                        if spec_case && res <= ulp && 2 * res >= ulp {
                            state = St::Roundup;
                        } else {
                            state = St::Retc;
                        }
                        continue 'dispatch;
                    }
                    i += 1;
                    res = res.wrapping_mul(5);
                    if ulpshift != 0 {
                        ulplo = 5u64.wrapping_mul(ulplo & ulpmask);
                        ulp = 5u64.wrapping_mul(ulp).wrapping_add(shr(ulplo, ulpshift));
                    } else {
                        ulp = ulp.wrapping_mul(5);
                    }
                }
            }

            /* ------------------------------------------------------------ */
            St::Toobig => {
                if ilim > 28 {
                    state = St::FastFailed1;
                    continue 'dispatch;
                }
                /* Scale by 10^-k */
                p10 = &PTEN[(342 - k) as usize];
                let tv0 = (p10.b2 as u64).wrapping_mul(dblo);
                let tv1 = (p10.b1 as u64).wrapping_mul(dblo).wrapping_add(tv0 >> 32);
                let tv2 = (p10.b2 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv1 & 0xffffffff);
                let tv3 = (p10.b0 as u64)
                    .wrapping_mul(dblo)
                    .wrapping_add(tv1 >> 32)
                    .wrapping_add(tv2 >> 32);
                res3 = (p10.b1 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv3 & 0xffffffff);
                res = (p10.b0 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv3 >> 32)
                    .wrapping_add(res3 >> 32);
                be += p10.e - 0x3fe;
                j1 = be - 54 + ulpadj;
                eulp = j1;
                if res & 0x8000000000000000 == 0 {
                    be -= 1;
                    res3 <<= 1;
                    res = (res << 1) | ((res3 & 0x100000000) >> 32);
                }
                res0 = res; /* save for Fast_failed */

                if ilim > 19 {
                    state = St::FastFailed;
                    continue 'dispatch;
                }
                res = shr(res, 4 - be);
                ulp = p10.b0 as u64; /* ulp */
                ulp = (ulp << 29) | ((p10.b1 as u64) >> 3);
                /* scaled ulp = ulp * 2^(eulp - 60) */
                /* We maintain 61 bits of the scaled ulp. */
                if ilim == 0 {
                    if res & 0x7fffffffffffffe == 0 || (!res) & 0x7fffffffffffffe == 0 {
                        state = St::FastFailed1;
                        continue 'dispatch;
                    }
                    if res >= 0x5000000000000000 {
                        state = St::OneDigit;
                    } else {
                        state = St::NoDigits;
                    }
                    continue 'dispatch;
                }
                rb = 1; /* upper bound on rounding error */
                loop {
                    dig = (res >> 60) as i32;
                    *buf.add(s) = (b'0' as i32 + dig) as c_char;
                    s += 1;
                    res &= 0xfffffffffffffff;
                    if ilim < 0 {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if eulp > 0 {
                            let sulp = shl(ulp, eulp - 1);
                            if res <= ures {
                                if res + rb > ures - rb {
                                    state = St::FastFailed;
                                    continue 'dispatch;
                                }
                                if res < sulp {
                                    state = St::Retc;
                                    continue 'dispatch;
                                }
                            } else {
                                if res - rb <= ures + rb {
                                    state = St::FastFailed;
                                    continue 'dispatch;
                                }
                                if ures < sulp {
                                    state = St::Roundup;
                                    continue 'dispatch;
                                }
                            }
                        } else {
                            let zb = shl(1, eulp + 63).wrapping_neg();
                            if zb & res == 0 {
                                let sres = shl(res, 1 - eulp);
                                if sres < ulp && (!spec_case || 2 * sres < ulp) {
                                    if shl(res + rb, 1 - eulp) >= ulp {
                                        state = St::FastFailed;
                                        continue 'dispatch;
                                    }
                                    if ures < res {
                                        if ures + rb >= res - rb {
                                            state = St::FastFailed;
                                            continue 'dispatch;
                                        }
                                        state = St::Roundup;
                                        continue 'dispatch;
                                    }
                                    if ures - rb < res + rb {
                                        state = St::FastFailed;
                                        continue 'dispatch;
                                    }
                                    state = St::Retc;
                                    continue 'dispatch;
                                }
                            }
                            if zb & ures == 0 && shl(ures, -eulp) < ulp {
                                if shl(ures, 1 - eulp) < ulp {
                                    state = St::Roundup;
                                } else {
                                    state = St::FastFailed;
                                }
                                continue 'dispatch;
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if ures < res {
                            if ures <= rb || res - rb <= ures + rb {
                                if j + k >= 0 && k >= 0 && k <= 27 {
                                    /* use_exact1 */
                                    s = 0;
                                    i = 1;
                                    state = St::UseExact;
                                } else {
                                    state = St::FastFailed;
                                }
                                continue 'dispatch;
                            }
                            state = St::Roundup;
                            continue 'dispatch;
                        }
                        if res <= rb || ures - rb <= res + rb {
                            if j + k >= 0 && k >= 0 && k <= 27 {
                                /* use_exact1: */
                                s = 0;
                                i = 1;
                                state = St::UseExact;
                            } else {
                                state = St::FastFailed;
                            }
                            continue 'dispatch;
                        }
                        state = St::Retc;
                        continue 'dispatch;
                    }
                    rb = rb.wrapping_mul(10);
                    if rb >= 0x1000000000000000 {
                        state = St::FastFailed;
                        continue 'dispatch;
                    }
                    res = res.wrapping_mul(10);
                    ulp = ulp.wrapping_mul(5);
                    if ulp & 0x8000000000000000 != 0 {
                        eulp += 4;
                        ulp >>= 3;
                    } else {
                        eulp += 3;
                        ulp >>= 2;
                    }
                    i += 1;
                }
            }

            /* ------------------------------------------------------------ */
            St::FastFailed => {
                s = 0;
                i = 4 - be;
                res = shr(res0, i);
                reslo = 0xffffffff & res3;
                if i != 0 {
                    reslo = (shl(res0, 64 - i) >> 32) | shr(reslo, i);
                }
                rb = 0;
                rblo = 4; /* roundoff bound */
                ulp = p10.b0 as u64; /* ulp */
                ulp = (ulp << 29) | ((p10.b1 as u64) >> 3);
                eulp = j1;
                i = 1;
                loop {
                    dig = (res >> 60) as i32;
                    *buf.add(s) = (b'0' as i32 + dig) as c_char;
                    s += 1;
                    res &= 0xfffffffffffffff;

                    'more96: {
                        if ilim < 0 {
                            ures = 0x1000000000000000u64.wrapping_sub(res);
                            ureslo = 0;
                            if reslo != 0 {
                                ureslo = 0x100000000u64.wrapping_sub(reslo);
                                ures = ures.wrapping_sub(1);
                            }
                            if eulp > 0 {
                                let sulp = shl(ulp, eulp - 1).wrapping_sub(rb);
                                if res <= ures {
                                    if res < sulp && res + rb < ures - rb {
                                        state = St::Retc;
                                        continue 'dispatch;
                                    }
                                } else if ures < sulp && res - rb > ures + rb {
                                    state = St::Roundup;
                                    continue 'dispatch;
                                }
                                state = St::FastFailed1;
                                continue 'dispatch;
                            } else {
                                let zb = shl(1, eulp + 60).wrapping_neg();
                                if zb & (res + rb) == 0 {
                                    let mut sres = shl(res - rb, 1 - eulp);
                                    if sres < ulp && (!spec_case || 2 * sres < ulp) {
                                        sres = shl(res, 1 - eulp);
                                        j = eulp + 31;
                                        if j > 0 {
                                            sres = sres.wrapping_add(shr(rblo + reslo, j));
                                        } else {
                                            sres = sres.wrapping_add(shl(rblo + reslo, -j));
                                        }
                                        if sres.wrapping_add(shl(rb, 1 - eulp)) >= ulp {
                                            state = St::FastFailed1;
                                            continue 'dispatch;
                                        }
                                        if sres >= ulp {
                                            break 'more96;
                                        }
                                        if ures < res || (ures == res && ureslo < reslo) {
                                            if ures + rb >= res - rb {
                                                state = St::FastFailed1;
                                                continue 'dispatch;
                                            }
                                            state = St::Roundup;
                                            continue 'dispatch;
                                        }
                                        if ures - rb <= res + rb {
                                            state = St::FastFailed1;
                                            continue 'dispatch;
                                        }
                                        state = St::Retc;
                                        continue 'dispatch;
                                    }
                                }
                                if zb & ures == 0 && shl(ures - rb, 1 - eulp) < ulp {
                                    if shl(ures + rb, 1 - eulp) < ulp {
                                        state = St::Roundup;
                                    } else {
                                        state = St::FastFailed1;
                                    }
                                    continue 'dispatch;
                                }
                            }
                        } else if i == ilim {
                            ures = 0x1000000000000000u64.wrapping_sub(res);
                            let mut sres: u64 = 0;
                            ureslo = 0;
                            if reslo != 0 {
                                ureslo = 0x100000000u64.wrapping_sub(reslo);
                                ures = ures.wrapping_sub(1);
                                sres = (reslo + rblo) >> 31;
                            }
                            sres = sres.wrapping_add(2 * rb);
                            if ures <= res {
                                if ures <= sres || res - ures <= sres {
                                    state = St::FastFailed1;
                                } else {
                                    state = St::Roundup;
                                }
                                continue 'dispatch;
                            }
                            if res <= sres || ures - res <= sres {
                                state = St::FastFailed1;
                            } else {
                                state = St::Retc;
                            }
                            continue 'dispatch;
                        }
                    }
                    /* more96: */
                    rblo = rblo.wrapping_mul(10);
                    rb = 10u64.wrapping_mul(rb).wrapping_add(rblo >> 32);
                    rblo &= 0xffffffff;
                    if rb >= 0x1000000000000000 {
                        state = St::FastFailed1;
                        continue 'dispatch;
                    }
                    reslo = reslo.wrapping_mul(10);
                    res = 10u64.wrapping_mul(res).wrapping_add(reslo >> 32);
                    reslo &= 0xffffffff;
                    ulp = ulp.wrapping_mul(5);
                    if ulp & 0x8000000000000000 != 0 {
                        eulp += 4;
                        ulp >>= 3;
                    } else {
                        eulp += 3;
                        ulp >>= 2;
                    }
                    i += 1;
                }
            }

            /* ------------------------------------------------------------ */
            St::FastFailed1 => {
                bs = None;
                mhi = None;
                mlo = None;
                let (bb, bee, bbb) = d2b(ull);
                b = Some(bb);
                be = bee;
                bbits = bbb;
                s = 0;
                i = ((ull >> 32) as u32 >> EXP_SHIFT1 & (EXP_MASK >> EXP_SHIFT1)) as i32;
                i -= BIAS;
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
                state = St::BigTail;
            }

            /* ------------------------------------------------------------ */
            St::BigTail => {
                let mut m2 = b2;
                let m5 = b5;
                mhi = None;
                mlo = None;
                if leftright != 0 {
                    i = if denorm != 0 {
                        be + (BIAS + (P - 1) - 1 + 1)
                    } else {
                        1 + P - bbits
                    };
                    b2 += i;
                    s2 += i;
                    mhi = Some(i2b(1));
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
                            mhi = Some(pow5mult(mhi.take().unwrap(), m5));
                            let nb = mult(mhi.as_ref().unwrap(), b.as_ref().unwrap());
                            b = Some(nb);
                        }
                        j = b5 - m5;
                        if j != 0 {
                            b = Some(pow5mult(b.take().unwrap(), j));
                        }
                    } else {
                        b = Some(pow5mult(b.take().unwrap(), b5));
                    }
                }
                bs = Some(i2b(1));
                if s5 > 0 {
                    bs = Some(pow5mult(bs.take().unwrap(), s5));
                }

                if spec_case {
                    b2 += LOG2P;
                    s2 += LOG2P;
                }

                /* Arrange for convenient computation of quotients:
                 * shift left if necessary so divisor has 4 leading 0 bits. */
                i = dshift(bs.as_ref().unwrap(), s2);
                b2 += i;
                m2 += i;
                s2 += i;
                if b2 > 0 {
                    b = Some(lshift(b.take().unwrap(), b2));
                }
                if s2 > 0 {
                    bs = Some(lshift(bs.take().unwrap(), s2));
                }

                if ilim <= 0 && (mode == 3 || mode == 5) {
                    if ilim < 0 {
                        state = St::NoDigits;
                        continue 'dispatch;
                    }
                    bs = Some(multadd(bs.take().unwrap(), 5, 0));
                    if cmp(b.as_ref().unwrap(), bs.as_ref().unwrap()) <= 0 {
                        state = St::NoDigits;
                    } else {
                        state = St::OneDigit;
                    }
                    continue 'dispatch;
                }
                if leftright != 0 {
                    if m2 > 0 {
                        mhi = Some(lshift(mhi.take().unwrap(), m2));
                    }

                    /* Compute mlo -- check for special case
                     * that d is a normalized power of 2. */
                    if spec_case {
                        let lo = mhi.take().unwrap();
                        let mut hi = balloc(lo.k);
                        bcopy(&mut hi, &lo);
                        mhi = Some(lshift(hi, LOG2P));
                        mlo = Some(lo);
                    }

                    let w1_odd = ((ull & 0xffffffff) as u32) & 1 != 0;

                    i = 1;
                    loop {
                        dig = quorem(b.as_mut().unwrap(), bs.as_ref().unwrap()) + b'0' as i32;
                        /* Do we yet have the shortest decimal string
                         * that will round to d? */
                        let jj = {
                            let mlo_ref = mlo.as_ref().unwrap_or(mhi.as_ref().unwrap());
                            cmp(b.as_ref().unwrap(), mlo_ref)
                        };
                        j = jj;
                        let delta = diff(bs.as_ref().unwrap(), mhi.as_ref().unwrap());
                        j1 = if delta.sign != 0 {
                            1
                        } else {
                            cmp(b.as_ref().unwrap(), &delta)
                        };

                        if j1 == 0 && mode != 1 && !w1_odd {
                            if dig == b'9' as i32 {
                                /* round_9_up */
                                *buf.add(s) = b'9' as c_char;
                                s += 1;
                                state = St::Roundoff;
                                continue 'dispatch;
                            }
                            if j > 0 {
                                dig += 1;
                            }
                            *buf.add(s) = dig as c_char;
                            s += 1;
                            state = St::Ret;
                            continue 'dispatch;
                        }
                        if j < 0 || (j == 0 && mode != 1 && !w1_odd) {
                            let bref = b.as_ref().unwrap();
                            if !(bref.x[0] == 0 && bref.wds <= 1) {
                                if j1 > 0 {
                                    b = Some(lshift(b.take().unwrap(), 1));
                                    j1 = cmp(b.as_ref().unwrap(), bs.as_ref().unwrap());
                                    if (j1 > 0 || (j1 == 0 && dig & 1 != 0)) && {
                                        let old = dig;
                                        dig += 1;
                                        old == b'9' as i32
                                    } {
                                        /* round_9_up */
                                        *buf.add(s) = b'9' as c_char;
                                        s += 1;
                                        state = St::Roundoff;
                                        continue 'dispatch;
                                    }
                                }
                            }
                            /* accept_dig: */
                            *buf.add(s) = dig as c_char;
                            s += 1;
                            state = St::Ret;
                            continue 'dispatch;
                        }
                        if j1 > 0 {
                            if dig == b'9' as i32 {
                                /* possible if i == 1 */
                                /* round_9_up: */
                                *buf.add(s) = b'9' as c_char;
                                s += 1;
                                state = St::Roundoff;
                                continue 'dispatch;
                            }
                            *buf.add(s) = (dig + 1) as c_char;
                            s += 1;
                            state = St::Ret;
                            continue 'dispatch;
                        }
                        *buf.add(s) = dig as c_char;
                        s += 1;
                        if i == ilim {
                            break;
                        }
                        b = Some(multadd(b.take().unwrap(), 10, 0));
                        if mlo.is_none() {
                            mhi = Some(multadd(mhi.take().unwrap(), 10, 0));
                        } else {
                            mlo = Some(multadd(mlo.take().unwrap(), 10, 0));
                            mhi = Some(multadd(mhi.take().unwrap(), 10, 0));
                        }
                        i += 1;
                    }
                } else {
                    i = 1;
                    loop {
                        dig = quorem(b.as_mut().unwrap(), bs.as_ref().unwrap()) + b'0' as i32;
                        *buf.add(s) = dig as c_char;
                        s += 1;
                        {
                            let bref = b.as_ref().unwrap();
                            if bref.x[0] == 0 && bref.wds <= 1 {
                                state = St::Ret;
                                continue 'dispatch;
                            }
                        }
                        if i >= ilim {
                            break;
                        }
                        b = Some(multadd(b.take().unwrap(), 10, 0));
                        i += 1;
                    }
                }

                /* Round off last digit */
                b = Some(lshift(b.take().unwrap(), 1));
                j = cmp(b.as_ref().unwrap(), bs.as_ref().unwrap());
                if j > 0 || (j == 0 && dig & 1 != 0) {
                    state = St::Roundoff;
                } else {
                    state = St::Ret;
                }
            }

            /* ------------------------------------------------------------ */
            St::Roundoff => {
                let mut done = false;
                loop {
                    s -= 1;
                    if *buf.add(s) != b'9' as c_char {
                        break;
                    }
                    if s == 0 {
                        k += 1;
                        *buf.add(s) = b'1' as c_char;
                        s += 1;
                        done = true;
                        break;
                    }
                }
                if !done {
                    *buf.add(s) = (*buf.add(s)).wrapping_add(1);
                    s += 1;
                }
                state = St::Ret;
            }

            /* ------------------------------------------------------------ */
            St::Roundup => {
                loop {
                    s -= 1;
                    if *buf.add(s) != b'9' as c_char {
                        break;
                    }
                    if s == 0 {
                        k += 1;
                        *buf.add(s) = b'1' as c_char;
                        s += 1;
                        state = St::Ret1;
                        continue 'dispatch;
                    }
                }
                *buf.add(s) = (*buf.add(s)).wrapping_add(1);
                s += 1;
                state = St::Ret1;
            }

            /* ------------------------------------------------------------ */
            St::NoDigits => {
                k = -1 - ndigits;
                state = St::Ret;
            }

            St::OneDigit => {
                *buf.add(s) = b'1' as c_char;
                s += 1;
                k += 1;
                state = St::Ret;
            }

            /* ------------------------------------------------------------ */
            St::Ret => {
                /* Bfree(S); Bfree(mlo); Bfree(mhi) -- handled by Rust drops */
                bs = None;
                mhi = None;
                mlo = None;
                state = St::Retc;
            }

            St::Retc => {
                while s > 0 && *buf.add(s - 1) == b'0' as c_char {
                    s -= 1;
                }
                state = St::Ret1;
            }

            St::Ret1 => {
                drop(b);
                *buf.add(s) = 0;
                *decpt = k + 1;
                if !rve.is_null() {
                    *rve = buf.add(s);
                }
                return buf;
            }
        }
    }
}
