// Translation of c_src/src/dtoa.c -- David M. Gay's dtoa/strtod.
// Core Bigint infrastructure + freedtoa/dtoa entry points.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::memory::{jsonp_free, jsonp_malloc};
use std::ffi::{c_char, c_int};
use std::ptr;

pub type ULong = u32;

pub const Kmax: c_int = 7;
pub const PRIVATE_MEM: usize = 2304;
pub const PRIVATE_mem: usize = (PRIVATE_MEM + 8 - 1) / 8;

/* Layout must match the C struct exactly: freedtoa()/rv_alloc() depend on it. */
#[repr(C)]
pub struct Bigint {
    pub next: *mut Bigint,
    pub k: c_int,
    pub maxwds: c_int,
    pub sign: c_int,
    pub wds: c_int,
    pub x: [ULong; 1],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union U {
    pub d: f64,
    pub L: [ULong; 2],
    pub LL: u64,
}

impl U {
    #[inline]
    pub fn new() -> U {
        U { LL: 0 }
    }
    #[inline]
    pub fn from_d(d: f64) -> U {
        U { d }
    }
    /* word0 == L[1], word1 == L[0] for IEEE_8087 */
    #[inline]
    pub fn w0(&self) -> ULong {
        unsafe { self.L[1] }
    }
    #[inline]
    pub fn w1(&self) -> ULong {
        unsafe { self.L[0] }
    }
    #[inline]
    pub fn set_w0(&mut self, v: ULong) {
        unsafe {
            self.L[1] = v;
        }
    }
    #[inline]
    pub fn set_w1(&mut self, v: ULong) {
        unsafe {
            self.L[0] = v;
        }
    }
    #[inline]
    pub fn dval(&self) -> f64 {
        unsafe { self.d }
    }
    #[inline]
    pub fn set_dval(&mut self, v: f64) {
        self.d = v;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
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

pub const Exp_shift: c_int = 20;
pub const Exp_shift1: c_int = 20;
pub const Exp_msk1: ULong = 0x100000;
pub const Exp_msk11: ULong = 0x100000;
pub const Exp_mask: ULong = 0x7ff00000;
pub const P: c_int = 53;
pub const Nbits: c_int = 53;
pub const Bias: c_int = 1023;
pub const Emax: c_int = 1023;
pub const Emin: c_int = -1022;
pub const Exp_1: ULong = 0x3ff00000;
pub const Exp_11: ULong = 0x3ff00000;
pub const Ebits: c_int = 11;
pub const Frac_mask: ULong = 0xfffff;
pub const Frac_mask1: ULong = 0xfffff;
pub const Ten_pmax: c_int = 22;
pub const Bletch: c_int = 0x10;
pub const Bndry_mask: ULong = 0xfffff;
pub const Bndry_mask1: ULong = 0xfffff;
pub const LSB: ULong = 1;
pub const Sign_bit: ULong = 0x80000000;
pub const Log2P: c_int = 1;
pub const Tiny0: ULong = 0;
pub const Tiny1: ULong = 1;
pub const Quick_max: c_int = 14;
pub const Int_max: c_int = 14;
pub const Big0: ULong = Frac_mask1 | Exp_msk1 * ((1024 + 1023 - 1) as ULong);
pub const Big1: ULong = 0xffffffff;
pub const DBL_DIG: c_int = 15;
pub const DBL_MAX_10_EXP: c_int = 308;
pub const DBL_MAX_EXP: c_int = 1024;
pub const Flt_Rounds: c_int = 1;
pub const Rounding: c_int = Flt_Rounds;
pub const STRTOD_DIGLIM: c_int = 40;
pub const strtod_diglim: c_int = STRTOD_DIGLIM;

/* dtoa_divmax is a public (exported) variable in the C library. */
#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

struct ThInfo {
    freelist: [*mut Bigint; (Kmax + 1) as usize],
    p5s: *mut Bigint,
}

static mut TI0: ThInfo = ThInfo {
    freelist: [ptr::null_mut(); (Kmax + 1) as usize],
    p5s: ptr::null_mut(),
};

static mut PRIVATE_MEM_ARR: [f64; PRIVATE_mem] = [0.0; PRIVATE_mem];
/* number of doubles used from PRIVATE_MEM_ARR (== pmem_next - private_mem) */
static mut PMEM_USED: usize = 0;

static mut DTOA_RESULT: *mut c_char = ptr::null_mut();

#[inline]
fn ti0() -> *mut ThInfo {
    ptr::addr_of_mut!(TI0)
}

pub unsafe fn Balloc(k: c_int) -> *mut Bigint {
    let x: c_int;
    let mut rv: *mut Bigint;
    let len: u32;

    let ti = ti0();
    if k <= Kmax {
        rv = (*ti).freelist[k as usize];
        if !rv.is_null() {
            (*ti).freelist[k as usize] = (*rv).next;
            (*rv).sign = 0;
            (*rv).wds = 0;
            return rv;
        }
    }
    x = 1 << k;
    len = ((std::mem::size_of::<Bigint>() + ((x - 1) as usize) * std::mem::size_of::<ULong>() + 8
        - 1)
        / 8) as u32;
    if k <= Kmax && (PMEM_USED + len as usize) <= PRIVATE_mem {
        rv = ptr::addr_of_mut!(PRIVATE_MEM_ARR).cast::<f64>().add(PMEM_USED) as *mut Bigint;
        PMEM_USED += len as usize;
    } else {
        rv = jsonp_malloc(len as usize * 8) as *mut Bigint;
    }
    (*rv).k = k;
    (*rv).maxwds = x;
    (*rv).sign = 0;
    (*rv).wds = 0;
    rv
}

pub unsafe fn Bfree(v: *mut Bigint) {
    if !v.is_null() {
        if (*v).k > Kmax {
            jsonp_free(v as *mut std::ffi::c_void);
        } else {
            let ti = ti0();
            (*v).next = (*ti).freelist[(*v).k as usize];
            (*ti).freelist[(*v).k as usize] = v;
        }
    }
}

#[inline]
pub unsafe fn bx(b: *mut Bigint) -> *mut ULong {
    ptr::addr_of_mut!((*b).x) as *mut ULong
}

pub unsafe fn multadd(b0: *mut Bigint, m: c_int, a: c_int) -> *mut Bigint {
    let mut b = b0;
    let mut i: c_int;
    let mut wds: c_int;
    let mut x: *mut ULong;
    let mut carry: u64;
    let mut y: u64;
    let b1: *mut Bigint;

    wds = (*b).wds;
    x = bx(b);
    i = 0;
    carry = a as u32 as u64;
    loop {
        y = (*x as u64) * (m as u32 as u64) + carry;
        carry = y >> 32;
        *x = (y & 0xffffffff) as ULong;
        x = x.add(1);
        i += 1;
        if i >= wds {
            break;
        }
    }
    if carry != 0 {
        if wds >= (*b).maxwds {
            b1 = Balloc((*b).k + 1);
            /* memcpy(&b1->sign, &b->sign, b->wds*sizeof(int) + 2*sizeof(int)) */
            ptr::copy_nonoverlapping(
                ptr::addr_of!((*b).sign) as *const u8,
                ptr::addr_of_mut!((*b1).sign) as *mut u8,
                ((*b).wds as usize) * 4 + 2 * 4,
            );
            Bfree(b);
            b = b1;
        }
        *bx(b).add(wds as usize) = carry as ULong;
        wds += 1;
        (*b).wds = wds;
    }
    b
}

pub unsafe fn s2b(s0: *const c_char, nd0: c_int, nd: c_int, y9: ULong, dplen: c_int) -> *mut Bigint {
    let mut b: *mut Bigint;
    let mut i: c_int;
    let k: c_int;
    let x: c_int;
    let mut y: c_int;
    let mut s = s0;

    x = (nd + 8) / 9;
    let mut kk = 0;
    y = 1;
    while x > y {
        y <<= 1;
        kk += 1;
    }
    k = kk;
    b = Balloc(k);
    *bx(b) = y9;
    (*b).wds = 1;
    i = 9;
    if 9 < nd0 {
        s = s.add(9);
        loop {
            b = multadd(b, 10, (*s as i32) - ('0' as i32));
            s = s.add(1);
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
        b = multadd(b, 10, (*s as i32) - ('0' as i32));
        s = s.add(1);
        i += 1;
    }
    b
}

pub fn hi0bits(x0: ULong) -> c_int {
    let mut x = x0;
    let mut k: c_int = 0;
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

pub unsafe fn lo0bits(y: *mut ULong) -> c_int {
    let mut k: c_int;
    let mut x: ULong = *y;

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

pub unsafe fn i2b(i: c_int) -> *mut Bigint {
    let b = Balloc(1);
    *bx(b) = i as ULong;
    (*b).wds = 1;
    b
}

pub unsafe fn mult(a0: *mut Bigint, b0: *mut Bigint) -> *mut Bigint {
    let mut a = a0;
    let mut b = b0;
    let c: *mut Bigint;
    let mut k: c_int;
    let wa: c_int;
    let wb: c_int;
    let mut wc: c_int;
    let mut x: *mut ULong;
    let mut xa: *mut ULong;
    let xae: *mut ULong;
    let mut xb: *mut ULong;
    let xbe: *mut ULong;
    let mut xc: *mut ULong;
    let mut xc0: *mut ULong;
    let mut y: ULong;
    let mut carry: u64;
    let mut z: u64;

    if (*a).wds < (*b).wds {
        let t = a;
        a = b;
        b = t;
    }
    k = (*a).k;
    wa = (*a).wds;
    wb = (*b).wds;
    wc = wa + wb;
    if wc > (*a).maxwds {
        k += 1;
    }
    c = Balloc(k);
    x = bx(c);
    xa = x.add(wc as usize);
    while x < xa {
        *x = 0;
        x = x.add(1);
    }
    xa = bx(a);
    xae = xa.add(wa as usize);
    xb = bx(b);
    xbe = xb.add(wb as usize);
    xc0 = bx(c);
    while xb < xbe {
        y = *xb;
        xb = xb.add(1);
        if y != 0 {
            x = xa;
            xc = xc0;
            carry = 0;
            loop {
                z = (*x as u64) * (y as u64) + (*xc as u64) + carry;
                x = x.add(1);
                carry = z >> 32;
                *xc = (z & 0xffffffff) as ULong;
                xc = xc.add(1);
                if x >= xae {
                    break;
                }
            }
            *xc = carry as ULong;
        }
        xc0 = xc0.add(1);
    }
    xc0 = bx(c);
    xc = xc0.add(wc as usize);
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

pub unsafe fn pow5mult(b0: *mut Bigint, k0: c_int) -> *mut Bigint {
    let mut b = b0;
    let mut k = k0;
    let mut b1: *mut Bigint;
    let mut p5: *mut Bigint;
    let mut p51: *mut Bigint;
    let i: c_int;
    static p05: [c_int; 3] = [5, 25, 125];

    i = k & 3;
    if i != 0 {
        b = multadd(b, p05[(i - 1) as usize], 0);
    }
    k >>= 2;
    if k == 0 {
        return b;
    }
    let ti = ti0();
    p5 = (*ti).p5s;
    if p5.is_null() {
        p5 = i2b(625);
        (*ti).p5s = p5;
        (*p5).next = ptr::null_mut();
    }
    loop {
        if k & 1 != 0 {
            b1 = mult(b, p5);
            Bfree(b);
            b = b1;
        }
        k >>= 1;
        if k == 0 {
            break;
        }
        p51 = (*p5).next;
        if p51.is_null() {
            p51 = mult(p5, p5);
            (*p5).next = p51;
            (*p51).next = ptr::null_mut();
        }
        p5 = p51;
    }
    b
}

pub unsafe fn lshift(b: *mut Bigint, k0: c_int) -> *mut Bigint {
    let mut k = k0;
    let mut i: c_int;
    let mut k1: c_int;
    let n: c_int;
    let mut n1: c_int;
    let b1: *mut Bigint;
    let mut x: *mut ULong;
    let mut x1: *mut ULong;
    let xe: *mut ULong;
    let mut z: ULong;

    n = k >> 5;
    k1 = (*b).k;
    n1 = n + (*b).wds + 1;
    i = (*b).maxwds;
    while n1 > i {
        k1 += 1;
        i <<= 1;
    }
    b1 = Balloc(k1);
    x1 = bx(b1);
    i = 0;
    while i < n {
        *x1 = 0;
        x1 = x1.add(1);
        i += 1;
    }
    x = bx(b);
    xe = x.add((*b).wds as usize);
    k &= 0x1f;
    if k != 0 {
        k1 = 32 - k;
        z = 0;
        loop {
            *x1 = (*x << k) | z;
            x1 = x1.add(1);
            z = *x >> k1;
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

pub unsafe fn cmp(a: *mut Bigint, b: *mut Bigint) -> c_int {
    let xa0: *mut ULong;
    let mut xa: *mut ULong;
    let xb0: *mut ULong;
    let mut xb: *mut ULong;
    let mut i: c_int;
    let j: c_int;

    i = (*a).wds;
    j = (*b).wds;
    i -= j;
    if i != 0 {
        return i;
    }
    xa0 = bx(a);
    xa = xa0.add(j as usize);
    xb0 = bx(b);
    xb = xb0.add(j as usize);
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

pub unsafe fn diff(a0: *mut Bigint, b0: *mut Bigint) -> *mut Bigint {
    let mut a = a0;
    let mut b = b0;
    let c: *mut Bigint;
    let mut i: c_int;
    let mut wa: c_int;
    let wb: c_int;
    let mut xa: *mut ULong;
    let xae: *mut ULong;
    let mut xb: *mut ULong;
    let xbe: *mut ULong;
    let mut xc: *mut ULong;
    let mut borrow: u64;
    let mut y: u64;

    i = cmp(a, b);
    if i == 0 {
        let c = Balloc(0);
        (*c).wds = 1;
        *bx(c) = 0;
        return c;
    }
    if i < 0 {
        let t = a;
        a = b;
        b = t;
        i = 1;
    } else {
        i = 0;
    }
    c = Balloc((*a).k);
    (*c).sign = i;
    wa = (*a).wds;
    xa = bx(a);
    xae = xa.add(wa as usize);
    wb = (*b).wds;
    xb = bx(b);
    xbe = xb.add(wb as usize);
    xc = bx(c);
    borrow = 0;
    loop {
        y = (*xa as u64).wrapping_sub(*xb as u64).wrapping_sub(borrow);
        xa = xa.add(1);
        xb = xb.add(1);
        borrow = (y >> 32) & 1;
        *xc = (y & 0xffffffff) as ULong;
        xc = xc.add(1);
        if xb >= xbe {
            break;
        }
    }
    while xa < xae {
        y = (*xa as u64).wrapping_sub(borrow);
        xa = xa.add(1);
        borrow = (y >> 32) & 1;
        *xc = (y & 0xffffffff) as ULong;
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

pub unsafe fn ulp(x: *mut U) -> f64 {
    let L: c_int;
    let mut u = U::new();

    L = (((*x).w0() & Exp_mask) as c_int) - (P - 1) * (Exp_msk1 as c_int);
    u.set_w0(L as ULong);
    u.set_w1(0);
    u.dval()
}

pub unsafe fn b2d(a: *mut Bigint, e: *mut c_int) -> f64 {
    let xa0: *mut ULong;
    let mut xa: *mut ULong;
    let w: ULong;
    let mut y: ULong;
    let z: ULong;
    let mut k: c_int;
    let mut d = U::new();

    xa0 = bx(a);
    xa = xa0.add((*a).wds as usize);
    xa = xa.sub(1);
    y = *xa;
    k = hi0bits(y);
    *e = 32 - k;
    if k < Ebits {
        d.set_w0(Exp_1 | (y >> (Ebits - k)));
        w = if xa > xa0 {
            xa = xa.sub(1);
            *xa
        } else {
            0
        };
        d.set_w1((y << ((32 - Ebits) + k)) | (w >> (Ebits - k)));
        return d.dval();
    }
    z = if xa > xa0 {
        xa = xa.sub(1);
        *xa
    } else {
        0
    };
    k -= Ebits;
    if k != 0 {
        d.set_w0(Exp_1 | (y << k) | (z >> (32 - k)));
        y = if xa > xa0 {
            xa = xa.sub(1);
            *xa
        } else {
            0
        };
        d.set_w1((z << k) | (y >> (32 - k)));
    } else {
        d.set_w0(Exp_1 | y);
        d.set_w1(z);
    }
    d.dval()
}

pub unsafe fn d2b(d: *mut U, e: *mut c_int, bits: *mut c_int) -> *mut Bigint {
    let b: *mut Bigint;
    let de: c_int;
    let mut k: c_int;
    let x: *mut ULong;
    let mut y: ULong;
    let mut z: ULong;
    let i: c_int;

    b = Balloc(1);
    x = bx(b);

    z = (*d).w0() & Frac_mask;
    (*d).set_w0((*d).w0() & 0x7fffffff);
    de = ((*d).w0() >> Exp_shift) as c_int;
    if de != 0 {
        z |= Exp_msk1;
    }
    y = (*d).w1();
    if y != 0 {
        k = lo0bits(&mut y);
        if k != 0 {
            *x.add(0) = y | (z << (32 - k));
            z >>= k;
        } else {
            *x.add(0) = y;
        }
        *x.add(1) = z;
        i = if z != 0 { 2 } else { 1 };
        (*b).wds = i;
    } else {
        k = lo0bits(&mut z);
        *x.add(0) = z;
        i = 1;
        (*b).wds = 1;
        k += 32;
    }
    if de != 0 {
        *e = de - Bias - (P - 1) + k;
        *bits = P - k;
    } else {
        *e = de - Bias - (P - 1) + 1 + k;
        *bits = 32 * i - hi0bits(*x.add((i - 1) as usize));
    }
    b
}

pub unsafe fn ratio(a: *mut Bigint, b: *mut Bigint) -> f64 {
    let mut da = U::new();
    let mut db = U::new();
    let mut k: c_int;
    let mut ka: c_int = 0;
    let mut kb: c_int = 0;

    da.set_dval(b2d(a, &mut ka));
    db.set_dval(b2d(b, &mut kb));
    k = ka - kb + 32 * ((*a).wds - (*b).wds);
    if k > 0 {
        da.set_w0(da.w0().wrapping_add((k as ULong).wrapping_mul(Exp_msk1)));
    } else {
        k = -k;
        db.set_w0(db.w0().wrapping_add((k as ULong).wrapping_mul(Exp_msk1)));
    }
    da.dval() / db.dval()
}

pub unsafe fn increment(b0: *mut Bigint) -> *mut Bigint {
    let mut b = b0;
    let mut x: *mut ULong;
    let xe: *mut ULong;
    let b1: *mut Bigint;

    x = bx(b);
    xe = x.add((*b).wds as usize);
    loop {
        if *x < 0xffffffff {
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
        b1 = Balloc((*b).k + 1);
        ptr::copy_nonoverlapping(
            ptr::addr_of!((*b).sign) as *const u8,
            ptr::addr_of_mut!((*b1).sign) as *mut u8,
            ((*b).wds as usize) * 4 + 2 * 4,
        );
        Bfree(b);
        b = b1;
    }
    let w = (*b).wds;
    *bx(b).add(w as usize) = 1;
    (*b).wds += 1;
    b
}

pub unsafe fn rshift(b: *mut Bigint, k0: c_int) {
    let mut k = k0;
    let mut x: *mut ULong;
    let mut x1: *mut ULong;
    let xe: *mut ULong;
    let mut y: ULong;
    let mut n: c_int;

    x1 = bx(b);
    x = x1;
    n = k >> 5;
    if n < (*b).wds {
        xe = x.add((*b).wds as usize);
        x = x.add(n as usize);
        k &= 31;
        if k != 0 {
            n = 32 - k;
            y = *x >> k;
            x = x.add(1);
            while x < xe {
                *x1 = y | (*x << n);
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
    let wds = x1.offset_from(bx(b)) as c_int;
    (*b).wds = wds;
    if wds == 0 {
        *bx(b) = 0;
    }
}

pub unsafe fn any_on(b: *mut Bigint, k0: c_int) -> ULong {
    let mut k = k0;
    let mut x: *mut ULong;
    let x0: *mut ULong;
    let mut x1: ULong;
    let x2: ULong;
    let mut n: c_int;
    let nwds: c_int;

    x = bx(b);
    nwds = (*b).wds;
    n = k >> 5;
    if n > nwds {
        n = nwds;
    } else if n < nwds {
        k &= 31;
        if k != 0 {
            x2 = *x.add(n as usize);
            x1 = x2;
            x1 >>= k;
            x1 <<= k;
            if x1 != x2 {
                return 1;
            }
        }
    }
    x0 = x;
    x = x.add(n as usize);
    while x > x0 {
        x = x.sub(1);
        if *x != 0 {
            return 1;
        }
    }
    0
}

pub unsafe fn dshift(b: *mut Bigint, p2: c_int) -> c_int {
    let mut rv = hi0bits(*bx(b).add(((*b).wds - 1) as usize)) - 4;
    if p2 > 0 {
        rv -= p2;
    }
    rv & 31
}

pub unsafe fn quorem(b: *mut Bigint, S: *mut Bigint) -> c_int {
    let mut n: c_int;
    let mut bxp: *mut ULong;
    let mut bxe: *mut ULong;
    let mut q: ULong;
    let mut sx: *mut ULong;
    let sxe: *mut ULong;
    let mut borrow: u64;
    let mut carry: u64;
    let mut y: u64;
    let mut ys: u64;

    n = (*S).wds;
    if (*b).wds < n {
        return 0;
    }
    sx = bx(S);
    n -= 1;
    sxe = sx.add(n as usize);
    bxp = bx(b);
    bxe = bxp.add(n as usize);
    q = *bxe / (*sxe + 1);
    if q != 0 {
        borrow = 0;
        carry = 0;
        loop {
            ys = (*sx as u64) * (q as u64) + carry;
            sx = sx.add(1);
            carry = ys >> 32;
            y = (*bxp as u64)
                .wrapping_sub(ys & 0xffffffff)
                .wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            *bxp = (y & 0xffffffff) as ULong;
            bxp = bxp.add(1);
            if sx > sxe {
                break;
            }
        }
        if *bxe == 0 {
            bxp = bx(b);
            loop {
                bxe = bxe.sub(1);
                if !(bxe > bxp && *bxe == 0) {
                    break;
                }
                n -= 1;
            }
            (*b).wds = n;
        }
    }
    if cmp(b, S) >= 0 {
        q += 1;
        borrow = 0;
        carry = 0;
        bxp = bx(b);
        sx = bx(S);
        loop {
            ys = (*sx as u64) + carry;
            sx = sx.add(1);
            carry = ys >> 32;
            y = (*bxp as u64)
                .wrapping_sub(ys & 0xffffffff)
                .wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            *bxp = (y & 0xffffffff) as ULong;
            bxp = bxp.add(1);
            if sx > sxe {
                break;
            }
        }
        bxp = bx(b);
        bxe = bxp.add(n as usize);
        if *bxe == 0 {
            loop {
                bxe = bxe.sub(1);
                if !(bxe > bxp && *bxe == 0) {
                    break;
                }
                n -= 1;
            }
            (*b).wds = n;
        }
    }
    q as c_int
}

pub unsafe fn rv_alloc(i: c_int) -> *mut c_char {
    let mut j: usize;
    let mut k: c_int;
    let r: *mut c_int;

    j = std::mem::size_of::<ULong>();
    k = 0;
    while std::mem::size_of::<Bigint>() - std::mem::size_of::<ULong>()
        - std::mem::size_of::<c_int>()
        + j
        <= (i as usize)
    {
        k += 1;
        j <<= 1;
    }
    r = Balloc(k) as *mut c_int;
    *r = k;
    DTOA_RESULT = r.add(1) as *mut c_char;
    DTOA_RESULT
}

pub unsafe fn nrv_alloc(
    s0str: *const c_char,
    s0: *mut c_char,
    s0len: usize,
    rve: *mut *mut c_char,
    n: c_int,
) -> *mut c_char {
    let rv: *mut c_char;
    let mut t: *mut c_char;
    let mut s = s0str;
    let mut buf = s0;

    if buf.is_null() {
        buf = rv_alloc(n);
    } else if s0len <= (n as usize) {
        rv = ptr::null_mut();
        t = (n as usize) as *mut c_char;
        if !rve.is_null() {
            *rve = t;
        }
        return rv;
    }
    rv = buf;
    t = buf;
    loop {
        *t = *s;
        s = s.add(1);
        if *t == 0 {
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
    let b = (s as *mut c_int).sub(1) as *mut Bigint;
    let kk = *(b as *mut c_int);
    (*b).k = kk;
    (*b).maxwds = 1 << kk;
    Bfree(b);
    if s == DTOA_RESULT {
        DTOA_RESULT = ptr::null_mut();
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
    crate::dtoa_r::dtoa_r(dd, mode, ndigits, decpt, sign, rve, ptr::null_mut(), 0)
}
