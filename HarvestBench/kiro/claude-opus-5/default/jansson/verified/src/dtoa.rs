//! Translation of `src/dtoa.c` (David M. Gay's floating point conversions).
//!
//! The active configuration of the C build is reproduced exactly:
//! `IEEE_8087`, `USE_BF96`, `INFNAN_CHECK`, hex FP support, `bigcomp`
//! support, private memory pool, no `MULTIPLE_THREADS`, no `USE_LOCALE`,
//! no `Honor_FLT_ROUNDS` (so `Rounding == Flt_Rounds == 1`).

use crate::cffi;
use crate::dtoa_tables::*;
use crate::memory::{jsonp_free, jsonp_malloc};
use core::ffi::{c_char, c_int, c_void};
use core::ptr::null_mut;

/* ------------------------------------------------------------------ */
/* configuration constants                                            */
/* ------------------------------------------------------------------ */

const Exp_shift: i32 = 20;
const Exp_shift1: i32 = 20;
const Exp_msk1: u32 = 0x100000;
const Exp_mask: u32 = 0x7ff00000;
const P: i32 = 53;
const Nbits: i32 = 53;
const Bias: i32 = 1023;
const Emax: i32 = 1023;
const Emin: i32 = -1022;
const Exp_1: u32 = 0x3ff00000;
const Ebits: i32 = 11;
const Frac_mask: u32 = 0xfffff;
const Frac_mask1: u32 = 0xfffff;
const Ten_pmax: i32 = 22;
const Bndry_mask: u32 = 0xfffff;
const Bndry_mask1: u32 = 0xfffff;
const LSB: u32 = 1;
const Sign_bit: u32 = 0x80000000;
const Log2P: i32 = 1;
const Tiny1: u32 = 1;
const DBL_MAX_EXP: i32 = 1024;
const DBL_DIG: i32 = 15;
const DBL_MAX_10_EXP: i32 = 308;
const FLT_RADIX: f64 = 2.0;

/// `Big0 = Frac_mask1 | Exp_msk1*(DBL_MAX_EXP+Bias-1)`
const Big0: u32 = Frac_mask1 | (Exp_msk1.wrapping_mul((DBL_MAX_EXP + Bias - 1) as u32));
const Big1: u32 = 0xffffffff;

const Kmax: usize = 7;
const ULbits: i32 = 32;
const kshift: i32 = 5;
const kmask: i32 = 31;

const NAN_WORD0: u32 = 0x7ff80000;
const NAN_WORD1: u32 = 0;

const Flt_Rounds: i32 = 1;
const Rounding: i32 = Flt_Rounds;

const STRTOD_DIGLIM: i32 = 40;
const strtod_diglim: i32 = STRTOD_DIGLIM;

const Scale_Bit: i32 = 0x10;

/* rounding values: same as FLT_ROUNDS */
const Round_zero: i32 = 0;
const Round_near: i32 = 1;
const Round_up: i32 = 2;
const Round_down: i32 = 3;

const ERANGE: c_int = cffi::ERANGE;

#[inline]
unsafe fn set_errno(x: c_int) {
    unsafe { cffi::set_errno(x) }
}

/* ------------------------------------------------------------------ */
/* the U union                                                        */
/* ------------------------------------------------------------------ */

/// `typedef union { double d; ULong L[2]; ULLong LL; } U;`
#[derive(Clone, Copy)]
#[repr(C)]
pub struct U {
    pub ll: u64,
}

impl U {
    #[inline]
    const fn zero() -> U {
        U { ll: 0 }
    }
    #[inline]
    fn from_d(d: f64) -> U {
        U { ll: d.to_bits() }
    }
    /// `dval(x)`
    #[inline]
    fn d(&self) -> f64 {
        f64::from_bits(self.ll)
    }
    #[inline]
    fn set_d(&mut self, v: f64) {
        self.ll = v.to_bits();
    }
    /// `word0(x)` (IEEE_8087: the high half)
    #[inline]
    fn w0(&self) -> u32 {
        (self.ll >> 32) as u32
    }
    /// `word1(x)` (IEEE_8087: the low half)
    #[inline]
    fn w1(&self) -> u32 {
        self.ll as u32
    }
    #[inline]
    fn set_w0(&mut self, v: u32) {
        self.ll = (self.ll & 0xffffffff) | ((v as u64) << 32);
    }
    #[inline]
    fn set_w1(&mut self, v: u32) {
        self.ll = (self.ll & 0xffffffff_00000000) | (v as u64);
    }
}

/* ------------------------------------------------------------------ */
/* Bigint                                                            */
/* ------------------------------------------------------------------ */

#[repr(C)]
pub struct Bigint {
    pub next: *mut Bigint,
    pub k: c_int,
    pub maxwds: c_int,
    pub sign: c_int,
    pub wds: c_int,
    pub x: [u32; 1],
}

#[inline]
unsafe fn bx(b: *mut Bigint) -> *mut u32 {
    unsafe { (*b).x.as_mut_ptr() }
}

#[inline]
unsafe fn xat(b: *mut Bigint, i: usize) -> u32 {
    unsafe { *bx(b).add(i) }
}

#[inline]
unsafe fn xset(b: *mut Bigint, i: usize, v: u32) {
    unsafe { *bx(b).add(i) = v }
}

const PRIVATE_MEM: usize = 2304;
const PRIVATE_mem: usize = (PRIVATE_MEM + core::mem::size_of::<f64>() - 1) / core::mem::size_of::<f64>();

static mut private_mem: [f64; PRIVATE_mem] = [0.0; PRIVATE_mem];
static mut pmem_next: *mut f64 = null_mut();

/// `static ThInfo TI0;` - free lists and the cached powers of five.
static mut freelist: [*mut Bigint; Kmax + 1] = [null_mut(); Kmax + 1];
static mut p5s: *mut Bigint = null_mut();

/// `int dtoa_divmax = 2;` - an exported global.
#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

#[inline]
unsafe fn get_divmax() -> c_int {
    unsafe { core::ptr::read(&raw const dtoa_divmax) }
}

/// `pfive[idx]`.
///
/// `dtoa_r()` reaches `pfive[k-1]` with `k == 0` for `mode` 3/5 with certain
/// negative `ndigits`; in the C build that reads the zero padding that precedes
/// the table.  Reproduce that as a zero instead of indexing out of bounds.
#[inline]
fn pfive_at(idx: c_int) -> u64 {
    if idx < 0 { 0 } else { pfive[idx as usize] }
}

unsafe fn Balloc(k: c_int) -> *mut Bigint {
    unsafe {
        let rv: *mut Bigint;

        let fl = &raw mut freelist;
        if k as usize <= Kmax && !(*fl)[k as usize].is_null() {
            rv = (*fl)[k as usize];
            (*fl)[k as usize] = (*rv).next;
        } else {
            let x = 1i32 << k;
            let len: u32 = ((core::mem::size_of::<Bigint>()
                + (x as usize - 1) * core::mem::size_of::<u32>()
                + core::mem::size_of::<f64>()
                - 1)
                / core::mem::size_of::<f64>()) as u32;

            let pm_base = (&raw mut private_mem) as *mut f64;
            if pmem_next.is_null() {
                pmem_next = pm_base;
            }
            if k as usize <= Kmax
                && (pmem_next.offset_from(pm_base) as usize + len as usize) <= PRIVATE_mem
            {
                rv = pmem_next as *mut Bigint;
                pmem_next = pmem_next.add(len as usize);
            } else {
                rv = jsonp_malloc(len as usize * core::mem::size_of::<f64>()) as *mut Bigint;
            }
            (*rv).k = k;
            (*rv).maxwds = x;
        }

        (*rv).sign = 0;
        (*rv).wds = 0;
        rv
    }
}

unsafe fn Bfree(v: *mut Bigint) {
    unsafe {
        if !v.is_null() {
            if (*v).k as usize > Kmax {
                jsonp_free(v as *mut c_void);
            } else {
                let fl = &raw mut freelist;
                (*v).next = (*fl)[(*v).k as usize];
                (*fl)[(*v).k as usize] = v;
            }
        }
    }
}

/// `Bcopy(x, y)`
#[inline]
unsafe fn Bcopy(x: *mut Bigint, y: *mut Bigint) {
    unsafe {
        let n = (*y).wds as usize * core::mem::size_of::<i32>() + 2 * core::mem::size_of::<c_int>();
        core::ptr::copy_nonoverlapping(
            (&raw const (*y).sign) as *const u8,
            (&raw mut (*x).sign) as *mut u8,
            n,
        );
    }
}

/* multiply by m and add a */
unsafe fn multadd(b_in: *mut Bigint, m: c_int, a: c_int) -> *mut Bigint {
    unsafe {
        let mut b = b_in;
        let mut wds = (*b).wds;
        let mut x = bx(b);
        let mut i = 0;
        let mut carry: u64 = a as u64;
        loop {
            let y: u64 = (*x as u64) * (m as u64) + carry;
            carry = y >> 32;
            *x = (y & 0xffffffff) as u32;
            x = x.add(1);
            i += 1;
            if i >= wds {
                break;
            }
        }
        if carry != 0 {
            if wds >= (*b).maxwds {
                let b1 = Balloc((*b).k + 1);
                Bcopy(b1, b);
                Bfree(b);
                b = b1;
            }
            xset(b, wds as usize, carry as u32);
            wds += 1;
            (*b).wds = wds;
        }
        b
    }
}

unsafe fn s2b(s_in: *const c_char, nd0: c_int, nd: c_int, y9: u32, dplen: c_int) -> *mut Bigint {
    unsafe {
        let mut s = s_in;
        let mut i: c_int;
        let mut k: c_int;
        let x: i32 = (nd + 8) / 9;
        let mut y: i32 = 1;
        k = 0;
        while x > y {
            y <<= 1;
            k += 1;
        }

        let mut b = Balloc(k);
        xset(b, 0, y9);
        (*b).wds = 1;

        i = 9;
        if 9 < nd0 {
            s = s.add(9);
            loop {
                let d = *s as i32 - '0' as i32;
                s = s.add(1);
                b = multadd(b, 10, d);
                i += 1;
                if i >= nd0 {
                    break;
                }
            }
            s = s.add(dplen as usize);
        } else {
            s = s.add(dplen as usize + 9);
        }
        while i < nd {
            let d = *s as i32 - '0' as i32;
            s = s.add(1);
            b = multadd(b, 10, d);
            i += 1;
        }
        b
    }
}

fn hi0bits(x_in: u32) -> c_int {
    let mut x = x_in;
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

unsafe fn lo0bits(y: *mut u32) -> c_int {
    unsafe {
        let mut k: c_int;
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
}

unsafe fn i2b(i: c_int) -> *mut Bigint {
    unsafe {
        let b = Balloc(1);
        xset(b, 0, i as u32);
        (*b).wds = 1;
        b
    }
}

unsafe fn mult(a_in: *mut Bigint, b_in: *mut Bigint) -> *mut Bigint {
    unsafe {
        let mut a = a_in;
        let mut b = b_in;

        if (*a).wds < (*b).wds {
            let c = a;
            a = b;
            b = c;
        }
        let mut k = (*a).k;
        let wa = (*a).wds;
        let wb = (*b).wds;
        let mut wc = wa + wb;
        if wc > (*a).maxwds {
            k += 1;
        }
        let c = Balloc(k);
        {
            let mut x = bx(c);
            let xa = x.add(wc as usize);
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
            let y = *xb;
            xb = xb.add(1);
            if y != 0 {
                let mut x = xa;
                let mut xc = xc0;
                let mut carry: u64 = 0;
                loop {
                    let z: u64 = (*x as u64) * (y as u64) + (*xc as u64) + carry;
                    x = x.add(1);
                    carry = z >> 32;
                    *xc = (z & 0xffffffff) as u32;
                    xc = xc.add(1);
                    if x >= xae {
                        break;
                    }
                }
                *xc = carry as u32;
            }
            xc0 = xc0.add(1);
        }

        let xc0 = bx(c);
        let mut xc = xc0.add(wc as usize);
        while wc > 0 {
            xc = xc.sub(1);
            if *xc != 0 {
                break;
            }
            wc -= 1;
        }
        (*c).wds = wc;
        c
    }
}

unsafe fn pow5mult(b_in: *mut Bigint, k_in: c_int) -> *mut Bigint {
    unsafe {
        static p05: [c_int; 3] = [5, 25, 125];

        let mut b = b_in;
        let mut k = k_in;

        let i = k & 3;
        if i != 0 {
            b = multadd(b, p05[(i - 1) as usize], 0);
        }

        k >>= 2;
        if k == 0 {
            return b;
        }

        let p5s_p = &raw mut p5s;
        let mut p5 = *p5s_p;
        if p5.is_null() {
            p5 = i2b(625);
            *p5s_p = p5;
            (*p5).next = null_mut();
        }
        loop {
            if k & 1 != 0 {
                let b1 = mult(b, p5);
                Bfree(b);
                b = b1;
            }
            k >>= 1;
            if k == 0 {
                break;
            }
            let mut p51 = (*p5).next;
            if p51.is_null() {
                p51 = mult(p5, p5);
                (*p5).next = p51;
                (*p51).next = null_mut();
            }
            p5 = p51;
        }
        b
    }
}

unsafe fn lshift(b: *mut Bigint, k_in: c_int) -> *mut Bigint {
    unsafe {
        let mut k = k_in;
        let n = k >> 5;

        let mut k1 = (*b).k;
        let mut n1 = n + (*b).wds + 1;
        let mut i = (*b).maxwds;
        while n1 > i {
            i <<= 1;
            k1 += 1;
        }
        let b1 = Balloc(k1);
        let mut x1 = bx(b1);
        for _ in 0..n {
            *x1 = 0;
            x1 = x1.add(1);
        }
        let mut x = bx(b);
        let xe = x.add((*b).wds as usize);

        k &= 0x1f;
        if k != 0 {
            let ks = 32 - k;
            let mut z: u32 = 0;
            loop {
                *x1 = (*x << k) | z;
                x1 = x1.add(1);
                z = *x >> ks;
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
        Bfree(b);
        b1
    }
}

unsafe fn cmp(a: *mut Bigint, b: *mut Bigint) -> c_int {
    unsafe {
        let mut i = (*a).wds;
        let j = (*b).wds;

        i -= j;
        if i != 0 {
            return i;
        }
        let xa0 = bx(a);
        let mut xa = xa0.add(j as usize);
        let xb0 = bx(b);
        let mut xb = xb0.add(j as usize);
        loop {
            xa = xa.sub(1);
            xb = xb.sub(1);
            if *xa != *xb {
                return if *xa < *xb { -1 } else { 1 };
            }
            if xa <= xa0 {
                break;
            }
        }
        0
    }
}

unsafe fn diff(a_in: *mut Bigint, b_in: *mut Bigint) -> *mut Bigint {
    unsafe {
        let mut a = a_in;
        let mut b = b_in;

        let mut i = cmp(a, b);
        if i == 0 {
            let c = Balloc(0);
            (*c).wds = 1;
            xset(c, 0, 0);
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
        let c = Balloc((*a).k);
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
            *xc = (y & 0xffffffff) as u32;
            xc = xc.add(1);
            if xb >= xbe {
                break;
            }
        }
        while xa < xae {
            let y: u64 = (*xa as u64).wrapping_sub(borrow);
            xa = xa.add(1);
            borrow = (y >> 32) & 1;
            *xc = (y & 0xffffffff) as u32;
            xc = xc.add(1);
        }

        loop {
            xc = xc.sub(1);
            if *xc != 0 {
                break;
            }
            wa -= 1;
        }
        (*c).wds = wa;
        c
    }
}

unsafe fn ulp(x: *const U) -> f64 {
    unsafe {
        let L: i32 = ((*x).w0() & Exp_mask).wrapping_sub(((P - 1) as u32).wrapping_mul(Exp_msk1))
            as i32;
        let mut u = U::zero();
        u.set_w0(L as u32);
        u.set_w1(0);
        u.d()
    }
}

unsafe fn b2d(a: *mut Bigint, e: *mut c_int) -> f64 {
    unsafe {
        let mut d = U::zero();
        let d0: u32;
        let d1: u32;

        let xa0 = bx(a);
        let mut xa = xa0.add((*a).wds as usize);
        xa = xa.sub(1);
        let mut y = *xa;

        let mut k = hi0bits(y);
        *e = 32 - k;
        if k < Ebits {
            d0 = Exp_1 | (y >> (Ebits - k));
            let w = if xa > xa0 {
                xa = xa.sub(1);
                *xa
            } else {
                0
            };
            d1 = (y << ((32 - Ebits) + k)) | (w >> (Ebits - k));
            d.set_w0(d0);
            d.set_w1(d1);
            return d.d();
        }
        let z = if xa > xa0 {
            xa = xa.sub(1);
            *xa
        } else {
            0
        };
        k -= Ebits;
        if k != 0 {
            d0 = Exp_1 | (y << k) | (z >> (32 - k));
            y = if xa > xa0 {
                xa = xa.sub(1);
                *xa
            } else {
                0
            };
            d1 = (z << k) | (y >> (32 - k));
        } else {
            d0 = Exp_1 | y;
            d1 = z;
        }
        d.set_w0(d0);
        d.set_w1(d1);
        d.d()
    }
}

unsafe fn d2b(d: *mut U, e: *mut c_int, bits: *mut c_int) -> *mut Bigint {
    unsafe {
        let b = Balloc(1);
        let x = bx(b);

        let mut z = (*d).w0() & Frac_mask;
        /* clear sign bit, which we ignore (note: this writes back into *d,
           exactly as the C macro does) */
        (*d).set_w0((*d).w0() & 0x7fffffff);

        let de = ((*d).w0() >> Exp_shift) as c_int;
        if de != 0 {
            z |= Exp_msk1;
        }

        let mut k: c_int;
        let i: c_int;
        let mut y = (*d).w1();
        if y != 0 {
            k = lo0bits(&mut y);
            if k != 0 {
                *x.add(0) = y | (z << (32 - k));
                z >>= k;
            } else {
                *x.add(0) = y;
            }
            *x.add(1) = z;
            (*b).wds = if z != 0 { 2 } else { 1 };
            i = (*b).wds;
        } else {
            k = lo0bits(&mut z);
            *x.add(0) = z;
            (*b).wds = 1;
            i = 1;
            k += 32;
        }

        if de != 0 {
            *e = de - Bias - (P - 1) + k;
            *bits = P - k;
        } else {
            *e = de - Bias - (P - 1) + 1 + k;
            *bits = 32 * i - hi0bits(*x.add(i as usize - 1));
        }
        b
    }
}

unsafe fn ratio(a: *mut Bigint, b: *mut Bigint) -> f64 {
    unsafe {
        let mut da = U::zero();
        let mut db = U::zero();
        let mut ka: c_int = 0;
        let mut kb: c_int = 0;

        da.set_d(b2d(a, &mut ka));
        db.set_d(b2d(b, &mut kb));

        let mut k = ka - kb + 32 * ((*a).wds - (*b).wds);
        if k > 0 {
            da.set_w0(da.w0().wrapping_add((k as u32).wrapping_mul(Exp_msk1)));
        } else {
            k = -k;
            db.set_w0(db.w0().wrapping_add((k as u32).wrapping_mul(Exp_msk1)));
        }

        da.d() / db.d()
    }
}

unsafe fn match_(sp: *mut *const c_char, t_in: &[u8]) -> c_int {
    unsafe {
        let mut s = *sp;
        let mut ti = 0usize;
        loop {
            let d = t_in[ti] as i32;
            ti += 1;
            if d == 0 {
                break;
            }
            s = s.add(1);
            let mut c = *(s as *const u8) as i32;
            if c >= 'A' as i32 && c <= 'Z' as i32 {
                c += 'a' as i32 - 'A' as i32;
            }
            if c != d {
                return 0;
            }
        }
        *sp = s.add(1);
        1
    }
}

unsafe fn hexnan(rvp: *mut U, sp: *mut *const c_char) {
    unsafe {
        let mut x: [u32; 2] = [0, 0];
        let mut havedig = 0;
        let mut xshift = 0;
        let mut udx0 = 1;
        let mut s = *sp as *const u8;

        loop {
            let c = *s.add(1) as u32;
            if !(c != 0 && c <= ' ' as u32) {
                break;
            }
            s = s.add(1);
        }
        if *s.add(1) == b'0' && (*s.add(2) == b'x' || *s.add(2) == b'X') {
            s = s.add(2);
        }
        loop {
            s = s.add(1);
            let mut c = *s as u32;
            if c == 0 {
                break;
            }
            let c1 = hexdig[c as usize] as u32;
            if c1 != 0 {
                c = c1 & 0xf;
            } else if c <= ' ' as u32 {
                if udx0 != 0 && havedig != 0 {
                    udx0 = 0;
                    xshift = 1;
                }
                continue;
            } else {
                loop {
                    if c == ')' as u32 {
                        *sp = (s.add(1)) as *const c_char;
                        break;
                    }
                    s = s.add(1);
                    c = *s as u32;
                    if c == 0 {
                        break;
                    }
                }
                break;
            }

            havedig = 1;
            if xshift != 0 {
                xshift = 0;
                x[0] = x[1];
                x[1] = 0;
            }
            if udx0 != 0 {
                x[0] = (x[0] << 4) | (x[1] >> 28);
            }
            x[1] = (x[1] << 4) | c;
        }
        x[0] &= 0xfffff;
        if x[0] != 0 || x[1] != 0 {
            (*rvp).set_w0(Exp_mask | x[0]);
            (*rvp).set_w1(x[1]);
        }
    }
}

unsafe fn increment(b_in: *mut Bigint) -> *mut Bigint {
    unsafe {
        let mut b = b_in;
        let mut x = bx(b);
        let xe = x.add((*b).wds as usize);
        loop {
            if *x < 0xffffffffu32 {
                *x += 1;
                return b;
            }
            *x = 0;
            x = x.add(1);
            if x >= xe {
                break;
            }
        }
        if (*b).wds >= (*b).maxwds {
            let b1 = Balloc((*b).k + 1);
            Bcopy(b1, b);
            Bfree(b);
            b = b1;
        }
        let w = (*b).wds as usize;
        xset(b, w, 1);
        (*b).wds += 1;
        b
    }
}

unsafe fn rshift(b: *mut Bigint, k_in: c_int) {
    unsafe {
        let mut k = k_in;
        let mut x = bx(b);
        let mut x1 = x;
        let mut n = k >> kshift;
        if n < (*b).wds {
            let xe = x.add((*b).wds as usize);
            x = x.add(n as usize);
            k &= kmask;
            if k != 0 {
                n = 32 - k;
                let mut y = *x >> k;
                x = x.add(1);
                while x < xe {
                    *x1 = (y | (*x << n)) & 0xffffffff;
                    x1 = x1.add(1);
                    y = *x >> k;
                    x = x.add(1);
                }
                *x1 = y;
                if y != 0 {
                    x1 = x1.add(1);
                }
            } else {
                while x < xe {
                    *x1 = *x;
                    x1 = x1.add(1);
                    x = x.add(1);
                }
            }
        }
        (*b).wds = x1.offset_from(bx(b)) as c_int;
        if (*b).wds == 0 {
            xset(b, 0, 0);
        }
    }
}

unsafe fn any_on(b: *mut Bigint, k_in: c_int) -> u32 {
    unsafe {
        let mut k = k_in;
        let mut x = bx(b);
        let nwds = (*b).wds;
        let mut n = k >> kshift;
        if n > nwds {
            n = nwds;
        } else if n < nwds {
            k &= kmask;
            if k != 0 {
                let x2 = *x.add(n as usize);
                let mut x1 = x2;
                x1 >>= k;
                x1 <<= k;
                if x1 != x2 {
                    return 1;
                }
            }
        }
        let x0 = x;
        x = x.add(n as usize);
        while x > x0 {
            x = x.sub(1);
            if *x != 0 {
                return 1;
            }
        }
        0
    }
}

/* ------------------------------------------------------------------ */
/* gethex                                                             */
/* ------------------------------------------------------------------ */

const gethex_emax: i32 = 0x7fe - Bias - P + 1;
const gethex_emin: i32 = Emin - P + 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethex(
    sp: *mut *const c_char,
    rvp: *mut U,
    rounding: c_int,
    sign: c_int,
) {
    unsafe {
        let mut b: *mut Bigint = null_mut();
        let mut e: i32 = 0;
        let mut e1: i32;
        let mut L: u32;
        let mut lostbits: u32;
        let mut x: *mut u32;
        let mut big: i32;
        let mut denorm: i32;
        let mut esign: i32;
        let mut havedig: i32;
        let mut k: i32;
        let mut n: i32;
        let mut nb: i32;
        let mut nbits: i32;
        let mut nz: i32;
        let mut up: i32;
        let mut zret: i32;
        let mut check_denorm: i32 = 0;

        let mut decpt: *const u8 = core::ptr::null();
        let mut s0: *const u8;
        let mut s: *const u8;
        let s1: *const u8;

        havedig = 0;
        s0 = (*sp as *const u8).add(2);
        while *s0.add(havedig as usize) == b'0' {
            havedig += 1;
        }
        s0 = s0.add(havedig as usize);
        s = s0;
        zret = 0;

        'pcheck: {
            if hexdig[*s as usize] != 0 {
                havedig += 1;
            } else {
                zret = 1;

                if *s != b'.' {
                    break 'pcheck;
                }
                s = s.add(1);
                decpt = s;

                if hexdig[*s as usize] == 0 {
                    break 'pcheck;
                }
                while *s == b'0' {
                    s = s.add(1);
                }
                if hexdig[*s as usize] != 0 {
                    zret = 0;
                }
                havedig = 1;
                s0 = s;
            }
            while hexdig[*s as usize] != 0 {
                s = s.add(1);
            }

            if *s == b'.' && decpt.is_null() {
                s = s.add(1);
                decpt = s;

                while hexdig[*s as usize] != 0 {
                    s = s.add(1);
                }
            }
            if !decpt.is_null() {
                e = -(((s.offset_from(decpt) as i32) << 2));
            }
        }
        /* pcheck: */
        s1 = s;
        big = 0;
        esign = 0;
        if *s == b'p' || *s == b'P' {
            'pexp: {
                s = s.add(1);
                if *s == b'-' {
                    esign = 1;
                    s = s.add(1);
                } else if *s == b'+' {
                    s = s.add(1);
                }
                n = hexdig[*s as usize] as i32;
                if n == 0 || n > 0x19 {
                    s = s1;
                    break 'pexp;
                }
                e1 = n - 0x10;
                loop {
                    s = s.add(1);
                    n = hexdig[*s as usize] as i32;
                    if n == 0 || n > 0x19 {
                        break;
                    }
                    if e1 & 0xf8000000u32 as i32 != 0 {
                        big = 1;
                    }
                    e1 = 10i32.wrapping_mul(e1).wrapping_add(n - 0x10);
                }
                if esign != 0 {
                    e1 = -e1;
                }
                e = e.wrapping_add(e1);
            }
        }
        *sp = s as *const c_char;
        if havedig == 0 {
            *sp = (s0 as *const c_char).sub(1);
        }

        /* The tail is a small state machine reproducing the goto structure. */
        #[derive(PartialEq, Clone, Copy)]
        enum T {
            Body,
            RetTinyf,
            RetTiny,
            RetBig,
            Ovfl,
            Ovfl1,
            Retz,
            Retz1,
            Normal,
            Store,
            Done,
        }
        let mut goto_normal;
        let mut st = if zret != 0 { T::Retz1 } else { T::Body };

        lostbits = 0;
        denorm = 0;
        nbits = Nbits;
        x = null_mut();

        loop {
            match st {
                T::Body => {
                    if big != 0 {
                        if esign != 0 {
                            let mut tiny = false;
                            match rounding {
                                x if x == Round_up => {
                                    if sign == 0 {
                                        tiny = true;
                                    }
                                }
                                x if x == Round_down => {
                                    if sign != 0 {
                                        tiny = true;
                                    }
                                }
                                _ => {}
                            }
                            if tiny {
                                st = T::RetTiny;
                                continue;
                            }
                            st = T::Retz;
                            continue;
                        }
                        let mut ovfl1 = false;
                        match rounding {
                            x if x == Round_near => ovfl1 = true,
                            x if x == Round_up => {
                                if sign == 0 {
                                    ovfl1 = true;
                                }
                            }
                            x if x == Round_down => {
                                if sign != 0 {
                                    ovfl1 = true;
                                }
                            }
                            _ => {}
                        }
                        if ovfl1 {
                            st = T::Ovfl1;
                            continue;
                        }
                        st = T::RetBig;
                        continue;
                    }

                    n = (s1.offset_from(s0) as i32) - 1;
                    k = 0;
                    while n > (1 << (kshift - 2)) - 1 {
                        n >>= 1;
                        k += 1;
                    }
                    b = Balloc(k);
                    x = bx(b);
                    havedig = 0;
                    n = 0;
                    nz = 0;
                    let _ = nz;
                    L = 0;

                    let mut sp1 = s1;
                    while sp1 > s0 {
                        sp1 = sp1.sub(1);
                        if *sp1 == b'.' {
                            continue;
                        }
                        let d = hexdig[*sp1 as usize];
                        if d != 0 {
                            havedig = 1;
                        } else if havedig == 0 {
                            e += 4;
                            continue;
                        }
                        if n == ULbits {
                            *x = L;
                            x = x.add(1);
                            L = 0;
                            n = 0;
                        }
                        L |= ((d as u32) & 0x0f) << n;
                        n += 4;
                    }
                    *x = L;
                    x = x.add(1);
                    n = x.offset_from(bx(b)) as i32;
                    (*b).wds = n;
                    nb = ULbits * n - hi0bits(L);
                    nbits = Nbits;
                    lostbits = 0;
                    x = bx(b);
                    if nb > nbits {
                        n = nb - nbits;
                        if any_on(b, n) != 0 {
                            lostbits = 1;
                            k = n - 1;
                            if *x.add((k >> kshift) as usize) & (1u32 << (k & kmask)) != 0 {
                                lostbits = 2;
                                if k > 0 && any_on(b, k) != 0 {
                                    lostbits = 3;
                                }
                            }
                        }
                        rshift(b, n);
                        e += n;
                    } else if nb < nbits {
                        n = nbits - nb;
                        b = lshift(b, n);
                        e -= n;
                        x = bx(b);
                    }
                    if e > gethex_emax {
                        st = T::Ovfl;
                        continue;
                    }
                    denorm = 0;
                    goto_normal = false;
                    if e < gethex_emin {
                        denorm = 1;
                        n = gethex_emin - e;
                        if n >= nbits {
                            let mut tinyf = false;
                            match rounding {
                                r if r == Round_near => {
                                    if n == nbits
                                        && (n < 2 || lostbits != 0 || any_on(b, n - 1) != 0)
                                    {
                                        tinyf = true;
                                    }
                                }
                                r if r == Round_up => {
                                    if sign == 0 {
                                        tinyf = true;
                                    }
                                }
                                r if r == Round_down => {
                                    if sign != 0 {
                                        tinyf = true;
                                    }
                                }
                                _ => {}
                            }
                            if tinyf {
                                st = T::RetTinyf;
                                continue;
                            }
                            Bfree(b);
                            st = T::Retz;
                            continue;
                        }
                        k = n - 1;
                        if k == 0 {
                            let mut do_emin_check = false;
                            let mut do_incr_denorm = false;
                            match rounding {
                                r if r == Round_near => {
                                    if (xat(b, 0) & 3) == 3
                                        || (lostbits != 0 && (xat(b, 0) & 1) != 0)
                                    {
                                        multadd(b, 1, 1);
                                        do_emin_check = true;
                                    }
                                }
                                r if r == Round_up => {
                                    if sign == 0 && (lostbits != 0 || (xat(b, 0) & 1) != 0) {
                                        do_incr_denorm = true;
                                    }
                                }
                                r if r == Round_down => {
                                    if sign != 0 && (lostbits != 0 || (xat(b, 0) & 1) != 0) {
                                        do_incr_denorm = true;
                                    }
                                }
                                _ => {}
                            }
                            if do_incr_denorm {
                                multadd(b, 1, 2);
                                check_denorm = 1;
                                lostbits = 0;
                                do_emin_check = true;
                            }
                            if do_emin_check {
                                if xat(b, 1) == (1u32 << (Exp_shift + 1)) {
                                    rshift(b, 1);
                                    e = gethex_emin;
                                    goto_normal = true;
                                }
                            }
                        }

                        if !goto_normal {
                            let mut skip_lostbits = false;
                            if lostbits != 0 {
                                lostbits = 1;
                            } else if k > 0 {
                                lostbits = any_on(b, k);
                            } else if check_denorm != 0 {
                                skip_lostbits = true;
                            }

                            if !skip_lostbits {
                                if *x.add((k >> kshift) as usize) & (1u32 << (k & kmask)) != 0 {
                                    lostbits |= 2;
                                }
                            }
                            /* no_lostbits: */
                            nbits -= n;
                            rshift(b, n);
                            e = gethex_emin;
                        }
                    }
                    if goto_normal {
                        st = T::Normal;
                        continue;
                    }
                    if lostbits != 0 {
                        up = 0;
                        match rounding {
                            r if r == Round_zero => {}
                            r if r == Round_near => {
                                if lostbits & 2 != 0 && ((lostbits & 1) | (*x.add(0) & 1)) != 0 {
                                    up = 1;
                                }
                            }
                            r if r == Round_up => up = 1 - sign,
                            r if r == Round_down => up = sign,
                            _ => {}
                        }
                        if up != 0 {
                            k = (*b).wds;
                            b = increment(b);
                            x = bx(b);
                            if denorm == 0 {
                                let n2 = nbits & kmask;
                                if (*b).wds > k
                                    || (n2 != 0 && hi0bits(*x.add(k as usize - 1)) < 32 - n2)
                                {
                                    rshift(b, 1);
                                    e += 1;
                                    if e > Emax {
                                        st = T::Ovfl;
                                        continue;
                                    }
                                }
                            }
                        }
                    }

                    if denorm != 0 {
                        (*rvp).set_w0(if (*b).wds > 1 {
                            xat(b, 1) & !0x100000
                        } else {
                            0
                        });
                        st = T::Store;
                        continue;
                    }
                    st = T::Normal;
                    continue;
                }
                T::Normal => {
                    (*rvp).set_w0(
                        (xat(b, 1) & !0x100000) | (((e + 0x3ff + 52) as u32) << 20),
                    );
                    st = T::Store;
                }
                T::Store => {
                    (*rvp).set_w1(xat(b, 0));
                    Bfree(b);
                    st = T::Done;
                }
                T::RetTinyf => {
                    Bfree(b);
                    st = T::RetTiny;
                }
                T::RetTiny => {
                    set_errno(ERANGE);
                    (*rvp).set_w0(0);
                    (*rvp).set_w1(1);
                    st = T::Done;
                }
                T::RetBig => {
                    (*rvp).set_w0(Big0);
                    (*rvp).set_w1(Big1);
                    st = T::Done;
                }
                T::Ovfl => {
                    Bfree(b);
                    st = T::Ovfl1;
                }
                T::Ovfl1 => {
                    set_errno(ERANGE);
                    (*rvp).set_w0(Exp_mask);
                    (*rvp).set_w1(0);
                    st = T::Done;
                }
                T::Retz => {
                    set_errno(ERANGE);
                    st = T::Retz1;
                }
                T::Retz1 => {
                    (*rvp).set_d(0.0);
                    st = T::Done;
                }
                T::Done => return,
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* dshift / quorem / sulp / bigcomp                                   */
/* ------------------------------------------------------------------ */

unsafe fn dshift(b: *mut Bigint, p2: c_int) -> c_int {
    unsafe {
        let mut rv = hi0bits(xat(b, (*b).wds as usize - 1)) - 4;
        if p2 > 0 {
            rv -= p2;
        }
        rv & kmask
    }
}

unsafe fn quorem(b: *mut Bigint, S: *mut Bigint) -> c_int {
    unsafe {
        let mut n = (*S).wds;

        if (*b).wds < n {
            return 0;
        }
        let mut sx = bx(S);
        n -= 1;
        let sxe = sx.add(n as usize);
        let mut bx_ = bx(b);
        let mut bxe = bx_.add(n as usize);
        let mut q = *bxe / (*sxe + 1); /* ensure q <= true quotient */

        if q != 0 {
            let mut borrow: u64 = 0;
            let mut carry: u64 = 0;
            loop {
                let ys: u64 = (*sx as u64) * (q as u64) + carry;
                sx = sx.add(1);
                carry = ys >> 32;
                let y: u64 = (*bx_ as u64)
                    .wrapping_sub(ys & 0xffffffff)
                    .wrapping_sub(borrow);
                borrow = (y >> 32) & 1;
                *bx_ = (y & 0xffffffff) as u32;
                bx_ = bx_.add(1);
                if sx > sxe {
                    break;
                }
            }
            if *bxe == 0 {
                let bx0 = bx(b);
                loop {
                    bxe = bxe.sub(1);
                    if !(bxe > bx0 && *bxe == 0) {
                        break;
                    }
                    n -= 1;
                }
                (*b).wds = n;
            }
        }
        if cmp(b, S) >= 0 {
            q += 1;
            let mut borrow: u64 = 0;
            let mut carry: u64 = 0;
            let mut bx_ = bx(b);
            let mut sx = bx(S);
            loop {
                let ys: u64 = (*sx as u64) + carry;
                sx = sx.add(1);
                carry = ys >> 32;
                let y: u64 = (*bx_ as u64)
                    .wrapping_sub(ys & 0xffffffff)
                    .wrapping_sub(borrow);
                borrow = (y >> 32) & 1;
                *bx_ = (y & 0xffffffff) as u32;
                bx_ = bx_.add(1);
                if sx > sxe {
                    break;
                }
            }
            let bx0 = bx(b);
            let mut bxe = bx0.add(n as usize);
            if *bxe == 0 {
                loop {
                    bxe = bxe.sub(1);
                    if !(bxe > bx0 && *bxe == 0) {
                        break;
                    }
                    n -= 1;
                }
                (*b).wds = n;
            }
        }
        q as c_int
    }
}

unsafe fn sulp(x: *mut U, bc: *const BCinfo) -> f64 {
    unsafe {
        let rv = ulp(x);
        if (*bc).scale == 0 {
            return rv;
        }
        let i = 2 * P + 1 - ((((*x).w0() & Exp_mask) >> Exp_shift) as i32);
        if i <= 0 {
            return rv; /* Is there an example where i <= 0 ? */
        }
        let mut u = U::zero();
        u.set_w0(Exp_1.wrapping_add((i as u32) << Exp_shift));
        u.set_w1(0);
        rv * u.d()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BCinfo {
    pub dp0: c_int,
    pub dp1: c_int,
    pub dplen: c_int,
    pub dsign: c_int,
    pub e0: c_int,
    pub inexact: c_int,
    pub nd: c_int,
    pub nd0: c_int,
    pub rounding: c_int,
    pub scale: c_int,
    pub uflchk: c_int,
}

impl BCinfo {
    const fn zero() -> BCinfo {
        BCinfo {
            dp0: 0,
            dp1: 0,
            dplen: 0,
            dsign: 0,
            e0: 0,
            inexact: 0,
            nd: 0,
            nd0: 0,
            rounding: 0,
            scale: 0,
            uflchk: 0,
        }
    }
}

unsafe fn bigcomp(rv: *mut U, s0: *const c_char, bc: *mut BCinfo) {
    unsafe {
        let mut b: *mut Bigint;
        let mut dd: c_int = 0;
        let mut dig: c_int;
        let mut i: c_int;
        let mut j: c_int;
        let mut p2: c_int = 0;
        let mut bbits: c_int = 0;
        let mut speccase: c_int = 0;

        let mut dsign = (*bc).dsign;
        let nd = (*bc).nd;
        let nd0 = (*bc).nd0;
        let p5 = nd + (*bc).e0 - 1;

        let mut have_i = false;

        if (*rv).d() == 0.0 {
            /* special case: value near underflow-to-zero */
            b = i2b(1);
            p2 = Emin - P + 1;
            bbits = 1;
            (*rv).set_w0(((P + 2) as u32) << Exp_shift);
            i = 0;
            {
                speccase = 1;
                p2 -= 1;
                dsign = 0;
                have_i = true;
            }
            let _ = i;
        } else {
            b = d2b(rv, &mut p2, &mut bbits);
        }

        let mut i_val: c_int = 0;
        if !have_i {
            p2 -= (*bc).scale;
            i_val = P - bbits;
            j = P - Emin - 1 + p2;
            if i_val > j {
                i_val = j;
            }
            {
                b = lshift(b, i_val + 1);
                i_val += 1;
                xset(b, 0, xat(b, 0) | 1);
            }
        }
        /* have_i: */
        p2 -= p5 + i_val;
        let mut d = i2b(1);

        if p5 > 0 {
            d = pow5mult(d, p5);
        } else if p5 < 0 {
            b = pow5mult(b, -p5);
        }
        let mut b2: c_int;
        let mut d2: c_int;
        if p2 > 0 {
            b2 = p2;
            d2 = 0;
        } else {
            b2 = 0;
            d2 = -p2;
        }
        let ii = dshift(d, d2);
        b2 += ii;
        if b2 > 0 {
            b = lshift(b, b2);
        }
        d2 += ii;
        if d2 > 0 {
            d = lshift(d, d2);
        }

        dig = quorem(b, d);
        if dig == 0 {
            b = multadd(b, 10, 0); /* very unlikely */
            dig = quorem(b, d);
        }

        'ret: {
            i = 0;
            while i < nd0 {
                dd = *s0.add(i as usize) as c_int - '0' as c_int - dig;
                i += 1;
                if dd != 0 {
                    break 'ret;
                }
                if xat(b, 0) == 0 && (*b).wds == 1 {
                    if i < nd {
                        dd = 1;
                    }
                    break 'ret;
                }
                b = multadd(b, 10, 0);
                dig = quorem(b, d);
            }
            j = (*bc).dp1;
            loop {
                let prev_i = i;
                i += 1;
                if !(prev_i < nd) {
                    break;
                }
                dd = *s0.add(j as usize) as c_int - '0' as c_int - dig;
                j += 1;
                if dd != 0 {
                    break 'ret;
                }
                if xat(b, 0) == 0 && (*b).wds == 1 {
                    if i < nd {
                        dd = 1;
                    }
                    break 'ret;
                }
                b = multadd(b, 10, 0);
                dig = quorem(b, d);
            }
            if dig > 0 || xat(b, 0) != 0 || (*b).wds > 1 {
                dd = -1;
            }
        }
        /* ret: */
        Bfree(b);
        Bfree(d);

        let mut retlow1 = false;
        let mut rethi1 = false;

        if speccase != 0 {
            if dd <= 0 {
                (*rv).set_d(0.0);
            }
        } else if dd < 0 {
            if dsign == 0 {
                retlow1 = true;
            }
        } else if dd > 0 {
            if dsign != 0 {
                rethi1 = true;
            }
        } else {
            let mut odd = false;
            j = (((((*rv).w0() & Exp_mask) >> Exp_shift) as i32) - (*bc).scale) as c_int;
            if j <= 0 {
                i = 1 - j;
                if i <= 31 {
                    if (*rv).w1() & (1u32 << i) != 0 {
                        odd = true;
                    }
                } else if (*rv).w0() & (1u32 << (i - 32)) != 0 {
                    odd = true;
                }
            } else if (*rv).w1() & 1 != 0 {
                odd = true;
            }
            if odd {
                if dsign != 0 {
                    rethi1 = true;
                } else {
                    retlow1 = true;
                }
            }
        }

        if rethi1 {
            let v = (*rv).d() + sulp(rv, bc);
            (*rv).set_d(v);
        } else if retlow1 {
            let v = (*rv).d() - sulp(rv, bc);
            (*rv).set_d(v);
        }
    }
}

/* ------------------------------------------------------------------ */
/* dtoa result allocation                                             */
/* ------------------------------------------------------------------ */

static mut dtoa_result: *mut c_char = null_mut();

unsafe fn rv_alloc(i: c_int) -> *mut c_char {
    unsafe {
        let mut j: usize = core::mem::size_of::<u32>();
        let mut k: c_int = 0;
        while core::mem::size_of::<Bigint>() - core::mem::size_of::<u32>()
            - core::mem::size_of::<c_int>()
            + j
            <= i as usize
        {
            j <<= 1;
            k += 1;
        }
        let r = Balloc(k) as *mut c_int;
        *r = k;
        let res = r.add(1) as *mut c_char;
        core::ptr::write(&raw mut dtoa_result, res);
        res
    }
}

unsafe fn nrv_alloc(
    s_in: &[u8],
    s0_in: *mut c_char,
    s0len: usize,
    rve: *mut *mut c_char,
    n: c_int,
) -> *mut c_char {
    unsafe {
        let mut s0 = s0_in;
        let rv: *mut c_char;
        let mut t: *mut c_char;

        if s0.is_null() {
            s0 = rv_alloc(n);
        } else if s0len <= n as usize {
            rv = null_mut();
            t = rv.wrapping_add(n as usize);
            if !rve.is_null() {
                *rve = t;
            }
            return rv;
        }
        rv = s0;
        t = rv;
        let mut si = 0usize;
        loop {
            let c = s_in[si] as c_char;
            si += 1;
            *t = c;
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freedtoa(s: *mut c_char) {
    unsafe {
        let b = (s as *mut c_int).sub(1) as *mut Bigint;
        let k = *(b as *mut c_int);
        (*b).k = k;
        (*b).maxwds = 1 << k;
        Bfree(b);
        if s == core::ptr::read(&raw const dtoa_result) {
            core::ptr::write(&raw mut dtoa_result, null_mut());
        }
    }
}

/* ------------------------------------------------------------------ */
/* dtoa_r                                                             */
/* ------------------------------------------------------------------ */

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
    unsafe {
        let mut mode = mode_in;
        let mut ndigits = ndigits_in;
        let mut buf = buf_in;
        let mut blen = blen_in;

        let mut bbits: c_int = 0;
        let mut b2: c_int;
        let mut b5: c_int;
        let mut be: c_int;
        let mut dig: c_int;
        let mut i: c_int;
        let mut ilim: c_int;
        let mut ilim1: c_int;
        let mut j: c_int;
        let mut j1: c_int = 0;
        let mut k: c_int;
        let mut leftright: c_int;
        let mut m2: c_int;
        let mut m5: c_int;
        let mut s2: c_int;
        let mut s5: c_int;
        let mut spec_case: c_int;
        let denorm: c_int;

        let mut b: *mut Bigint = null_mut();
        let mut b1: *mut Bigint;
        let mut delta: *mut Bigint;
        let mut mlo: *mut Bigint = null_mut();
        let mut mhi: *mut Bigint = null_mut();
        let mut S: *mut Bigint = null_mut();
        let mut u: U;
        let mut s: *mut c_char;

        let mut p10: usize; /* index into pten */
        let dbhi: u64;
        let mut dbits: u64;
        let dblo: u64;
        let mut den: u64;
        let mut hb: u64;
        let mut rb: u64;
        let mut rblo: u64;
        let mut res: u64;
        let mut res0: u64;
        let mut res3: u64;
        let mut reslo: u64;
        let mut sres: u64;
        let mut sulp_v: u64;
        let mut tv0: u64;
        let mut tv1: u64;
        let mut tv2: u64;
        let mut tv3: u64;
        let mut ulp_v: u64;
        let mut ulplo: u64;
        let mut ulpmask: u64 = 0;
        let mut ures: u64;
        let mut ureslo: u64;
        let mut zb: u64;
        let mut eulp: c_int;
        let mut k1: c_int;
        let mut n2: c_int;
        let mut ulpadj: c_int;
        let mut ulpshift: c_int;

        u = U::from_d(dd);
        if u.w0() & Sign_bit != 0 {
            *sign = 1;
            u.set_w0(u.w0() & !Sign_bit); /* clear sign bit */
        } else {
            *sign = 0;
        }

        if (u.w0() & Exp_mask) == Exp_mask {
            /* Infinity or NaN */
            *decpt = 9999;
            if u.w1() == 0 && (u.w0() & 0xfffff) == 0 {
                return nrv_alloc(b"Infinity\0", buf, blen, rve, 8);
            }
            return nrv_alloc(b"NaN\0", buf, blen, rve, 3);
        }

        if u.d() == 0.0 {
            *decpt = 1;
            return nrv_alloc(b"0\0", buf, blen, rve, 1);
        }

        dbits = (u.ll & 0xfffffffffffff) << 11; /* fraction bits */
        be = (u.ll >> 52) as c_int;
        if be != 0 {
            /* biased exponent; nonzero ==> normal */
            dbits |= 0x8000000000000000;
            denorm = 0;
            ulpadj = 0;
        } else {
            denorm = 1;
            ulpadj = be + 1;
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
            ulpadj -= be;
        }
        j = Lhint[(be + 51) as usize] as c_int;
        p10 = j as usize;
        dbhi = dbits >> 32;
        dblo = dbits & 0xffffffff;
        i = be - 0x3fe;
        if i < pten[p10].e
            || (i == pten[p10].e
                && (dbhi < pten[p10].b0 as u64
                    || (dbhi == pten[p10].b0 as u64 && dblo < pten[p10].b1 as u64)))
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
        ilim1 = -1; /* Values for cases 0 and 1 */

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
                i = ndigits + k + 1;
                ilim = i;
                ilim1 = i - 1;
                if i <= 0 {
                    i = 1;
                }
            }
            _ => {}
        }

        if buf.is_null() {
            buf = rv_alloc(i);
            let kk = *(buf as *const c_int).sub(1);
            blen = core::mem::size_of::<Bigint>()
                + ((1usize << kk) - 1) * core::mem::size_of::<u32>()
                - core::mem::size_of::<c_int>();
        } else if blen <= i as usize {
            buf = null_mut();
            if !rve.is_null() {
                *rve = buf.wrapping_add(i as usize);
            }
            return buf;
        }
        s = buf;

        spec_case = 0;
        if mode < 2 || leftright != 0 {
            if u.w1() == 0 && (u.w0() & Bndry_mask) == 0 && (u.w0() & (Exp_mask & !Exp_msk1)) != 0
            {
                /* The special case */
                spec_case = 1;
            }
        }

        b = null_mut();

        /* --------------------------------------------------------------
           The remainder of the function is a state machine faithfully
           reproducing the goto structure of the original.
           -------------------------------------------------------------- */
        #[derive(Clone, Copy, PartialEq)]
        enum L {
            Start,
            UseExact,
            NoDiv,
            Toobig,
            FastFailed,
            FastFailed1,
            Roundup,
            NoDigits,
            OneDigit,
            Roundoff,
            Ret,
            Retc,
            Ret1,
            BigLoopDone,
        }

        let mut label = L::Start;

        let _ = ilim1;

        i = 1;
        j = 52 + 0x3ff - be;
        ulpshift = 0;
        ulplo = 0;
        res = 0;
        res0 = 0;
        res3 = 0;
        reslo = 0;
        ulp_v = 0;
        rb = 0;
        rblo = 0;
        eulp = 0;
        k1 = 0;
        n2 = 0;
        den = 0;
        p10 = 0;
        b2 = 0;
        b5 = 0;
        m2 = 0;
        m5 = 0;
        s2 = 0;
        s5 = 0;
        dig = 0;

        if ilim < 0 && (mode == 3 || mode == 5) {
            S = null_mut();
            mhi = null_mut();
            label = L::NoDigits;
        } else if k < 0 {
            if k < -25 {
                label = L::Toobig;
            } else {
                res = dbits >> 11;
                k1 = -(k + 1);
                n2 = pfivebits[k1 as usize] + 53;
                j1 = j;
                let mut toobig = false;
                if n2 > 61 {
                    ulpshift = n2 - 61;
                    ulpmask = (1u64 << ulpshift) - 1;
                    if res & ulpmask != 0 {
                        toobig = true;
                    } else {
                        j -= ulpshift;
                        res >>= ulpshift;
                    }
                }
                if toobig {
                    label = L::Toobig;
                } else {
                    ulp_v = pfive[k1 as usize];
                    res = res.wrapping_mul(ulp_v);
                    if ulpshift != 0 {
                        ulplo = ulp_v;
                        ulp_v >>= ulpshift;
                    }
                    j += k;
                    if ilim == 0 {
                        S = null_mut();
                        mhi = null_mut();
                        if res > (5u64 << j) {
                            label = L::OneDigit;
                        } else {
                            label = L::NoDigits;
                        }
                    } else {
                        label = L::NoDiv;
                    }
                }
            }
        } else if ilim == 0 && j + k >= 0 {
            S = null_mut();
            mhi = null_mut();
            if (dbits >> 11) > (pfive_at(k - 1) << j) {
                label = L::OneDigit;
            } else {
                label = L::NoDigits;
            }
        } else if k <= get_divmax() && j + k >= 0 {
            label = L::UseExact;
        } else {
            label = L::Toobig;
        }

        'sm: loop {
            match label {
                L::Start => unreachable!(),

                L::UseExact => {
                    res = dbits >> 11; /* residual */
                    ulp_v = 1;
                    if k <= 0 {
                        label = L::NoDiv;
                        continue 'sm;
                    }
                    j1 = j + k + 1;
                    den = pfive[(k - i) as usize] << (j1 - i);
                    loop {
                        dig = (res / den) as c_int;
                        *s = (b'0' as c_int + dig) as c_char;
                        s = s.add(1);
                        res -= (dig as u64) * den;
                        if res == 0 {
                            label = L::Retc;
                            continue 'sm;
                        }
                        if ilim < 0 {
                            ures = den - res;
                            if 2 * res <= ulp_v
                                && (if spec_case != 0 {
                                    4 * res <= ulp_v
                                } else {
                                    2 * res < ulp_v || (dig & 1) != 0
                                })
                            {
                                /* ulp_reached */
                                if ures < res || (ures == res && (dig & 1) != 0) {
                                    label = L::Roundup;
                                } else {
                                    label = L::Retc;
                                }
                                continue 'sm;
                            }
                            if 2 * ures < ulp_v {
                                label = L::Roundup;
                                continue 'sm;
                            }
                        } else if i == ilim {
                            if Rounding == 0 {
                                label = L::Retc;
                                continue 'sm;
                            }
                            if Rounding == 2 {
                                label = L::Roundup;
                                continue 'sm;
                            }
                            ures = 2 * res;
                            if ures > den
                                || (ures == den && (dig & 1) != 0)
                                || (spec_case != 0 && res <= ulp_v && 2 * res >= ulp_v)
                            {
                                label = L::Roundup;
                                continue 'sm;
                            }
                            label = L::Retc;
                            continue 'sm;
                        }
                        i += 1;
                        if j1 < i {
                            res *= 10;
                            ulp_v *= 10;
                        } else {
                            if i > k {
                                break;
                            }
                            den = pfive[(k - i) as usize] << (j1 - i);
                        }
                    }
                    label = L::NoDiv;
                }

                L::NoDiv => {
                    loop {
                        den = res >> j;
                        dig = den as c_int;
                        *s = (b'0' as c_int + dig) as c_char;
                        s = s.add(1);
                        res -= den << j;
                        if res == 0 {
                            label = L::Retc;
                            continue 'sm;
                        }
                        if ilim < 0 {
                            ures = (1u64 << j) - res;
                            if 2 * res <= ulp_v
                                && (if spec_case != 0 {
                                    4 * res <= ulp_v
                                } else {
                                    2 * res < ulp_v || (dig & 1) != 0
                                })
                            {
                                /* ulp_reached: */
                                if ures < res || (ures == res && (dig & 1) != 0) {
                                    label = L::Roundup;
                                } else {
                                    label = L::Retc;
                                }
                                continue 'sm;
                            }
                            if 2 * ures < ulp_v {
                                label = L::Roundup;
                                continue 'sm;
                            }
                        }
                        j -= 1;
                        if i == ilim {
                            hb = 1u64 << j;
                            if res & hb != 0 && ((dig & 1) != 0 || res & (hb - 1) != 0) {
                                label = L::Roundup;
                                continue 'sm;
                            }
                            if spec_case != 0 && res <= ulp_v && 2 * res >= ulp_v {
                                label = L::Roundup;
                                continue 'sm;
                            }
                            label = L::Retc;
                            continue 'sm;
                        }
                        i += 1;
                        res *= 5;
                        if ulpshift != 0 {
                            ulplo = 5 * (ulplo & ulpmask);
                            ulp_v = 5 * ulp_v + (ulplo >> ulpshift);
                        } else {
                            ulp_v *= 5;
                        }
                    }
                }

                L::Toobig => {
                    if ilim > 28 {
                        label = L::FastFailed1;
                        continue 'sm;
                    }
                    p10 = (342 - k) as usize;
                    tv0 = (pten[p10].b2 as u64) * dblo;
                    tv1 = (pten[p10].b1 as u64) * dblo + (tv0 >> 32);
                    tv2 = (pten[p10].b2 as u64) * dbhi + (tv1 & 0xffffffff);
                    tv3 = (pten[p10].b0 as u64) * dblo + (tv1 >> 32) + (tv2 >> 32);
                    res3 = (pten[p10].b1 as u64) * dbhi + (tv3 & 0xffffffff);
                    res = (pten[p10].b0 as u64) * dbhi + (tv3 >> 32) + (res3 >> 32);
                    be += pten[p10].e - 0x3fe;
                    j1 = be - 54 + ulpadj;
                    eulp = j1;
                    if res & 0x8000000000000000 == 0 {
                        be -= 1;
                        res3 <<= 1;
                        res = (res << 1) | ((res3 & 0x100000000) >> 32);
                    }
                    res0 = res; /* save for Fast_failed */

                    if ilim > 19 {
                        label = L::FastFailed;
                        continue 'sm;
                    }
                    res = res.wrapping_shr((4 - be) as u32);
                    ulp_v = pten[p10].b0 as u64; /* ulp */
                    ulp_v = (ulp_v << 29) | ((pten[p10].b1 as u64) >> 3);

                    if ilim == 0 {
                        if res & 0x7fffffffffffffe == 0 || (!res) & 0x7fffffffffffffe == 0 {
                            label = L::FastFailed1;
                            continue 'sm;
                        }
                        S = null_mut();
                        mhi = null_mut();
                        if res >= 0x5000000000000000 {
                            label = L::OneDigit;
                        } else {
                            label = L::NoDigits;
                        }
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
                                sulp_v = ulp_v.wrapping_shl((eulp - 1) as u32);
                                if res <= ures {
                                    if res.wrapping_add(rb) > ures.wrapping_sub(rb) {
                                        label = L::FastFailed;
                                        continue 'sm;
                                    }
                                    if res < sulp_v {
                                        label = L::Retc;
                                        continue 'sm;
                                    }
                                } else {
                                    if res.wrapping_sub(rb) <= ures.wrapping_add(rb) {
                                        label = L::FastFailed;
                                        continue 'sm;
                                    }
                                    if ures < sulp_v {
                                        label = L::Roundup;
                                        continue 'sm;
                                    }
                                }
                            } else {
                                zb = (1u64.wrapping_shl((eulp + 63) as u32)).wrapping_neg();
                                if zb & res == 0 {
                                    sres = res.wrapping_shl((1 - eulp) as u32);
                                    if sres < ulp_v && (spec_case == 0 || 2u64.wrapping_mul(sres) < ulp_v) {
                                        if res.wrapping_add(rb).wrapping_shl((1 - eulp) as u32) >= ulp_v {
                                            label = L::FastFailed;
                                            continue 'sm;
                                        }
                                        if ures < res {
                                            if ures.wrapping_add(rb) >= res.wrapping_sub(rb) {
                                                label = L::FastFailed;
                                                continue 'sm;
                                            }
                                            label = L::Roundup;
                                            continue 'sm;
                                        }
                                        if ures.wrapping_sub(rb) < res.wrapping_add(rb) {
                                            label = L::FastFailed;
                                            continue 'sm;
                                        }
                                        label = L::Retc;
                                        continue 'sm;
                                    }
                                }
                                if zb & ures == 0 && ures.wrapping_shl((-eulp) as u32) < ulp_v {
                                    if ures.wrapping_shl((1 - eulp) as u32) < ulp_v {
                                        label = L::Roundup;
                                        continue 'sm;
                                    }
                                    label = L::FastFailed;
                                    continue 'sm;
                                }
                            }
                        } else if i == ilim {
                            ures = 0x1000000000000000u64.wrapping_sub(res);
                            if ures < res {
                                if ures <= rb || res.wrapping_sub(rb) <= ures.wrapping_add(rb) {
                                    if j + k >= 0 && k >= 0 && k <= 27 {
                                        /* use_exact1 */
                                        s = buf;
                                        i = 1;
                                        label = L::UseExact;
                                        continue 'sm;
                                    }
                                    label = L::FastFailed;
                                    continue 'sm;
                                }
                                label = L::Roundup;
                                continue 'sm;
                            }
                            if res <= rb || ures.wrapping_sub(rb) <= res.wrapping_add(rb) {
                                if j + k >= 0 && k >= 0 && k <= 27 {
                                    /* use_exact1 */
                                    s = buf;
                                    i = 1;
                                    label = L::UseExact;
                                    continue 'sm;
                                }
                                label = L::FastFailed;
                                continue 'sm;
                            }
                            label = L::Retc;
                            continue 'sm;
                        }
                        rb = rb.wrapping_mul(10);
                        if rb >= 0x1000000000000000 {
                            label = L::FastFailed;
                            continue 'sm;
                        }
                        res = res.wrapping_mul(10);
                        ulp_v = ulp_v.wrapping_mul(5);
                        if ulp_v & 0x8000000000000000 != 0 {
                            eulp += 4;
                            ulp_v >>= 3;
                        } else {
                            eulp += 3;
                            ulp_v >>= 2;
                        }
                        i += 1;
                    }
                }

                L::FastFailed => {
                    s = buf;
                    i = 4 - be;
                    res = res0.wrapping_shr(i as u32);
                    reslo = 0xffffffff & res3;
                    if i != 0 {
                        reslo = (res0.wrapping_shl((64 - i) as u32) >> 32) | reslo.wrapping_shr(i as u32);
                    }
                    rb = 0;
                    rblo = 4; /* roundoff bound */
                    ulp_v = pten[p10].b0 as u64; /* ulp */
                    ulp_v = (ulp_v << 29) | ((pten[p10].b1 as u64) >> 3);
                    eulp = j1;
                    i = 1;
                    loop {
                        dig = (res >> 60) as c_int;
                        *s = (b'0' as c_int + dig) as c_char;
                        s = s.add(1);
                        res &= 0xfffffffffffffff;

                        let mut goto_more96 = false;
                        if ilim < 0 {
                            ures = 0x1000000000000000u64.wrapping_sub(res);
                            ureslo = 0;
                            if reslo != 0 {
                                ureslo = 0x100000000u64.wrapping_sub(reslo);
                                ures = ures.wrapping_sub(1);
                            }
                            if eulp > 0 {
                                sulp_v = ulp_v.wrapping_shl((eulp - 1) as u32).wrapping_sub(rb);
                                if res <= ures {
                                    if res < sulp_v {
                                        if res.wrapping_add(rb) < ures.wrapping_sub(rb) {
                                            label = L::Retc;
                                            continue 'sm;
                                        }
                                    }
                                } else if ures < sulp_v {
                                    if res.wrapping_sub(rb) > ures.wrapping_add(rb) {
                                        label = L::Roundup;
                                        continue 'sm;
                                    }
                                }
                                label = L::FastFailed1;
                                continue 'sm;
                            } else {
                                zb = (1u64.wrapping_shl((eulp + 60) as u32)).wrapping_neg();
                                if zb & res.wrapping_add(rb) == 0 {
                                    sres = res.wrapping_sub(rb).wrapping_shl((1 - eulp) as u32);
                                    if sres < ulp_v && (spec_case == 0 || 2u64.wrapping_mul(sres) < ulp_v) {
                                        sres = res.wrapping_shl((1 - eulp) as u32);
                                        j = eulp + 31;
                                        if j > 0 {
                                            sres = sres.wrapping_add(rblo.wrapping_add(reslo) >> j);
                                        } else {
                                            sres = sres.wrapping_add(rblo.wrapping_add(reslo).wrapping_shl((-j) as u32));
                                        }
                                        if sres.wrapping_add(rb.wrapping_shl((1 - eulp) as u32)) >= ulp_v {
                                            label = L::FastFailed1;
                                            continue 'sm;
                                        }
                                        if sres >= ulp_v {
                                            goto_more96 = true;
                                        } else if ures < res || (ures == res && ureslo < reslo) {
                                            if ures.wrapping_add(rb) >= res.wrapping_sub(rb) {
                                                label = L::FastFailed1;
                                                continue 'sm;
                                            }
                                            label = L::Roundup;
                                            continue 'sm;
                                        } else if ures.wrapping_sub(rb) <= res.wrapping_add(rb) {
                                            label = L::FastFailed1;
                                            continue 'sm;
                                        } else {
                                            label = L::Retc;
                                            continue 'sm;
                                        }
                                    }
                                }
                                if !goto_more96 {
                                    if zb & ures == 0 && ures.wrapping_sub(rb).wrapping_shl((1 - eulp) as u32) < ulp_v {
                                        if ures.wrapping_add(rb).wrapping_shl((1 - eulp) as u32) < ulp_v {
                                            label = L::Roundup;
                                            continue 'sm;
                                        }
                                        label = L::FastFailed1;
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
                            sres = sres.wrapping_add(2u64.wrapping_mul(rb));
                            if ures <= res {
                                if ures <= sres || res.wrapping_sub(ures) <= sres {
                                    label = L::FastFailed1;
                                    continue 'sm;
                                }
                                label = L::Roundup;
                                continue 'sm;
                            }
                            if res <= sres || ures.wrapping_sub(res) <= sres {
                                label = L::FastFailed1;
                                continue 'sm;
                            }
                            label = L::Retc;
                            continue 'sm;
                        }
                        /* more96: */
                        let _ = goto_more96;
                        rblo = rblo.wrapping_mul(10);
                        rb = 10u64.wrapping_mul(rb).wrapping_add(rblo >> 32);
                        rblo &= 0xffffffff;
                        if rb >= 0x1000000000000000 {
                            label = L::FastFailed1;
                            continue 'sm;
                        }
                        reslo = reslo.wrapping_mul(10);
                        res = 10u64.wrapping_mul(res).wrapping_add(reslo >> 32);
                        reslo &= 0xffffffff;
                        ulp_v = ulp_v.wrapping_mul(5);
                        if ulp_v & 0x8000000000000000 != 0 {
                            eulp += 4;
                            ulp_v >>= 3;
                        } else {
                            eulp += 3;
                            ulp_v >>= 2;
                        }
                        i += 1;
                    }
                }

                L::FastFailed1 => {
                    S = null_mut();
                    mhi = null_mut();
                    mlo = null_mut();

                    b = d2b(&mut u, &mut be, &mut bbits);

                    s = buf;
                    i = ((u.w0() >> Exp_shift1) & (Exp_mask >> Exp_shift1)) as c_int;
                    i -= Bias;
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
                    mhi = null_mut();
                    mlo = null_mut();
                    if leftright != 0 {
                        i = if denorm != 0 {
                            be + (Bias + (P - 1) - 1 + 1)
                        } else {
                            1 + P - bbits
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
                                Bfree(b);
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
                    S = i2b(1);
                    if s5 > 0 {
                        S = pow5mult(S, s5);
                    }

                    if spec_case != 0 {
                        b2 += Log2P;
                        s2 += Log2P;
                    }

                    i = dshift(S, s2);
                    b2 += i;
                    m2 += i;
                    s2 += i;
                    if b2 > 0 {
                        b = lshift(b, b2);
                    }
                    if s2 > 0 {
                        S = lshift(S, s2);
                    }

                    if ilim <= 0 && (mode == 3 || mode == 5) {
                        S = multadd(S, 5, 0);
                        if ilim < 0 || cmp(b, S) <= 0 {
                            label = L::NoDigits;
                            continue 'sm;
                        }
                        label = L::OneDigit;
                        continue 'sm;
                    }
                    if leftright != 0 {
                        if m2 > 0 {
                            mhi = lshift(mhi, m2);
                        }

                        mlo = mhi;
                        if spec_case != 0 {
                            mhi = Balloc((*mlo).k);
                            Bcopy(mhi, mlo);
                            mhi = lshift(mhi, Log2P);
                        }

                        i = 1;
                        loop {
                            dig = quorem(b, S) + b'0' as c_int;

                            j = cmp(b, mlo);
                            delta = diff(S, mhi);
                            j1 = if (*delta).sign != 0 { 1 } else { cmp(b, delta) };
                            Bfree(delta);

                            if j1 == 0 && mode != 1 && (u.w1() & 1) == 0 {
                                if dig == b'9' as c_int {
                                    /* round_9_up */
                                    *s = b'9' as c_char;
                                    s = s.add(1);
                                    label = L::Roundoff;
                                    continue 'sm;
                                }
                                if j > 0 {
                                    dig += 1;
                                }
                                *s = dig as c_char;
                                s = s.add(1);
                                label = L::Ret;
                                continue 'sm;
                            }

                            if j < 0 || (j == 0 && mode != 1 && (u.w1() & 1) == 0) {
                                if !(xat(b, 0) == 0 && (*b).wds <= 1) {
                                    if j1 > 0 {
                                        b = lshift(b, 1);
                                        j1 = cmp(b, S);
                                        if (j1 > 0 || (j1 == 0 && (dig & 1) != 0)) && {
                                            let old = dig;
                                            dig += 1;
                                            old == b'9' as c_int
                                        } {
                                            /* round_9_up */
                                            *s = b'9' as c_char;
                                            s = s.add(1);
                                            label = L::Roundoff;
                                            continue 'sm;
                                        }
                                    }
                                }
                                /* accept_dig: */
                                *s = dig as c_char;
                                s = s.add(1);
                                label = L::Ret;
                                continue 'sm;
                            }
                            if j1 > 0 {
                                if dig == b'9' as c_int {
                                    /* possible if i == 1 : round_9_up */
                                    *s = b'9' as c_char;
                                    s = s.add(1);
                                    label = L::Roundoff;
                                    continue 'sm;
                                }
                                *s = (dig + 1) as c_char;
                                s = s.add(1);
                                label = L::Ret;
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
                            dig = quorem(b, S) + b'0' as c_int;
                            *s = dig as c_char;
                            s = s.add(1);
                            if xat(b, 0) == 0 && (*b).wds <= 1 {
                                label = L::Ret;
                                continue 'sm;
                            }
                            if i >= ilim {
                                break;
                            }
                            b = multadd(b, 10, 0);
                            i += 1;
                        }
                    }
                    label = L::BigLoopDone;
                }

                L::BigLoopDone => {
                    /* Round off last digit */
                    b = lshift(b, 1);
                    j = cmp(b, S);
                    if j > 0 || (j == 0 && (dig & 1) != 0) {
                        label = L::Roundoff;
                        continue 'sm;
                    }
                    label = L::Ret;
                }

                L::Roundoff => {
                    loop {
                        s = s.sub(1);
                        if *s != b'9' as c_char {
                            break;
                        }
                        if s == buf {
                            k += 1;
                            *s = b'1' as c_char;
                            s = s.add(1);
                            label = L::Ret;
                            continue 'sm;
                        }
                    }
                    *s += 1;
                    s = s.add(1);
                    label = L::Ret;
                }

                L::Roundup => {
                    loop {
                        s = s.sub(1);
                        if *s != b'9' as c_char {
                            break;
                        }
                        if s == buf {
                            k += 1;
                            *s = b'1' as c_char;
                            s = s.add(1);
                            label = L::Ret1;
                            continue 'sm;
                        }
                    }
                    *s += 1;
                    s = s.add(1);
                    label = L::Ret1;
                }

                L::NoDigits => {
                    k = -1 - ndigits;
                    label = L::Ret;
                }

                L::OneDigit => {
                    *s = b'1' as c_char;
                    s = s.add(1);
                    k += 1;
                    label = L::Ret;
                }

                L::Ret => {
                    Bfree(S);
                    if !mhi.is_null() {
                        if !mlo.is_null() && mlo != mhi {
                            Bfree(mlo);
                        }
                        Bfree(mhi);
                    }
                    label = L::Retc;
                }

                L::Retc => {
                    while s > buf && *s.sub(1) == b'0' as c_char {
                        s = s.sub(1);
                    }
                    label = L::Ret1;
                }

                L::Ret1 => {
                    if !b.is_null() {
                        Bfree(b);
                    }
                    *s = 0;
                    *decpt = k + 1;
                    if !rve.is_null() {
                        *rve = s;
                    }
                    return buf;
                }
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
    unsafe {
        let r = core::ptr::read(&raw const dtoa_result);
        if !r.is_null() {
            freedtoa(r);
        }

        dtoa_r(dd, mode, ndigits, decpt, sign, rve, null_mut(), 0)
    }
}

/* ------------------------------------------------------------------ */
/* strtod__unused                                                     */
/* ------------------------------------------------------------------ */

include!("dtoa_strtod.rs");
