//! Translation of `src/dtoa.c` (David M. Gay's `dtoa`/`strtod`).
//!
//! The active configuration is `IEEE_8087` (little endian), `Long == int`,
//! long long available, `MALLOC == jsonp_malloc`, `FREE == jsonp_free`,
//! default rounding (`Rounding == 1`), no `MULTIPLE_THREADS`.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use crate::dtoa_tables::*;
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::types::{memcpy, ERANGE};
use core::ffi::{c_char, c_int, c_void};
use core::mem::offset_of;
use core::ptr::{null_mut, read_volatile};

/* -------------------------------------------------------------- constants */

pub const Kmax: c_int = 7;

/* (2304 + sizeof(double) - 1) / sizeof(double) */
const PRIVATE_mem: usize = 288;

pub const Round_zero: c_int = 0;
pub const Round_near: c_int = 1;
pub const Round_up: c_int = 2;
pub const Round_down: c_int = 3;

/* ------------------------------------------------------------------ types */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BF96 {
    pub b0: u32,
    pub b1: u32,
    pub b2: u32,
    pub e: c_int,
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

#[repr(C)]
pub struct Bigint {
    pub next: *mut Bigint,
    pub k: c_int,
    pub maxwds: c_int,
    pub sign: c_int,
    pub wds: c_int,
    pub x: [u32; 1],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union U {
    pub d: f64,
    pub L: [u32; 2],
    pub LL: u64,
}

#[repr(C)]
struct ThInfo {
    Freelist: [*mut Bigint; (Kmax + 1) as usize],
    P5s: *mut Bigint,
}

/* -------------------------------------------------------------- mutable state */

static mut TI0: ThInfo = ThInfo {
    Freelist: [null_mut(); (Kmax + 1) as usize],
    P5s: null_mut(),
};

static mut private_mem: [f64; PRIVATE_mem] = [0.0; PRIVATE_mem];
/* `pmem_next - private_mem`, in units of double */
static mut pmem_off: usize = 0;

static mut dtoa_result: *mut c_char = null_mut();

/// `int dtoa_divmax = 2;` — exported data symbol.
#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

/* ------------------------------------------------------------- accessors */

/// Pointer to the flexible `x[]` member of a `Bigint`.
#[inline(always)]
pub unsafe fn bx(b: *mut Bigint) -> *mut u32 {
    (b as *mut u8).add(offset_of!(Bigint, x)) as *mut u32
}

/// Pointer to the `sign` member (start of the region copied by the
/// `Bcopy` macro).
#[inline(always)]
unsafe fn bsign(b: *mut Bigint) -> *mut u8 {
    (b as *mut u8).add(offset_of!(Bigint, sign))
}

#[inline(always)]
pub unsafe fn Bcopy(dst: *mut Bigint, src: *mut Bigint) {
    memcpy(
        bsign(dst) as *mut c_void,
        bsign(src) as *const c_void,
        (*src).wds as usize * 4 + 8,
    );
}

/* -------------------------------------------------------- Bigint plumbing */

pub unsafe fn Balloc(k: c_int) -> *mut Bigint {
    let x: c_int;
    let rv: *mut Bigint;
    let len: u32;

    if k <= Kmax && !TI0.Freelist[k as usize].is_null() {
        rv = TI0.Freelist[k as usize];
        TI0.Freelist[k as usize] = (*rv).next;
    } else {
        x = 1 << k;
        len = ((core::mem::size_of::<Bigint>() + (x as usize - 1) * 4 + 8 - 1) / 8) as u32;
        if k <= Kmax && pmem_off + len as usize <= PRIVATE_mem {
            rv = private_mem.as_mut_ptr().add(pmem_off) as *mut Bigint;
            pmem_off += len as usize;
        } else {
            rv = jsonp_malloc(len as usize * 8) as *mut Bigint;
        }
        (*rv).k = k;
        (*rv).maxwds = x;
    }
    (*rv).wds = 0;
    (*rv).sign = 0;
    rv
}

pub unsafe fn Bfree(v: *mut Bigint) {
    if !v.is_null() {
        if (*v).k > Kmax {
            jsonp_free(v as *mut c_void);
        } else {
            (*v).next = TI0.Freelist[(*v).k as usize];
            TI0.Freelist[(*v).k as usize] = v;
        }
    }
}

/* multiply by m and add a */
pub unsafe fn multadd(mut b: *mut Bigint, m: c_int, a: c_int) -> *mut Bigint {
    let mut i: c_int;
    let mut wds: c_int;
    let mut x: *mut u32;
    let mut carry: u64;
    let mut y: u64;
    let b1: *mut Bigint;

    wds = (*b).wds;
    x = bx(b);
    i = 0;
    carry = a as u64;
    loop {
        y = (*x as u64)
            .wrapping_mul(m as u64)
            .wrapping_add(carry);
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
            b1 = Balloc((*b).k + 1);
            Bcopy(b1, b);
            Bfree(b);
            b = b1;
        }
        *bx(b).add(wds as usize) = carry as u32;
        wds += 1;
        (*b).wds = wds;
    }
    b
}


/// `pfive[i]`.
///
/// The original C code can index `pfive[]` out of bounds for degenerate
/// `ndigits` arguments to `dtoa_r()` (mode 3/5 with a negative `ndigits`,
/// which never happens through jansson's own API).  Rather than panicking or
/// performing an out-of-bounds read, yield 0 for those indices.
#[inline(always)]
fn pfive_at(i: c_int) -> u64 {
    if i >= 0 && (i as usize) < PFIVE.len() {
        PFIVE[i as usize]
    } else {
        0
    }
}

pub unsafe fn s2b(
    mut s: *const c_char,
    nd0: c_int,
    nd: c_int,
    y9: u32,
    dplen: c_int,
) -> *mut Bigint {
    let mut b: *mut Bigint;
    let mut i: c_int;
    let mut k: c_int;
    let x: c_int;
    let mut y: c_int;

    x = (nd + 8) / 9;
    k = 0;
    y = 1;
    while x > y {
        y <<= 1;
        k += 1;
    }
    b = Balloc(k);
    *bx(b) = y9;
    (*b).wds = 1;
    i = 9;
    if 9 < nd0 {
        s = s.add(9);
        loop {
            b = multadd(b, 10, (*s as c_int) - '0' as c_int);
            s = s.add(1);
            i += 1;
            if i >= nd0 {
                break;
            }
        }
        s = s.offset(dplen as isize);
    } else {
        s = s.offset(dplen as isize + 9);
    }
    while i < nd {
        b = multadd(b, 10, (*s as c_int) - '0' as c_int);
        s = s.add(1);
        i += 1;
    }
    b
}

pub unsafe fn hi0bits(mut x: u32) -> c_int {
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

pub unsafe fn lo0bits(y: *mut u32) -> c_int {
    let mut k: c_int;
    let mut x: u32 = *y;

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
    let b: *mut Bigint;
    b = Balloc(1);
    *bx(b) = i as u32;
    (*b).wds = 1;
    b
}

pub unsafe fn mult(mut a: *mut Bigint, mut b: *mut Bigint) -> *mut Bigint {
    let mut c: *mut Bigint;
    let mut k: c_int;
    let wa: c_int;
    let wb: c_int;
    let mut wc: c_int;
    let mut x: *mut u32;
    let mut xa: *mut u32;
    let xae: *mut u32;
    let mut xb: *mut u32;
    let xbe: *mut u32;
    let mut xc: *mut u32;
    let mut xc0: *mut u32;
    let mut y: u32;
    let mut carry: u64;
    let mut z: u64;

    if (*a).wds < (*b).wds {
        c = a;
        a = b;
        b = c;
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
                z = (*x as u64)
                    .wrapping_mul(y as u64)
                    .wrapping_add(*xc as u64)
                    .wrapping_add(carry);
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

pub unsafe fn pow5mult(mut b: *mut Bigint, mut k: c_int) -> *mut Bigint {
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
    p5 = TI0.P5s;
    if p5.is_null() {
        p5 = i2b(625);
        TI0.P5s = p5;
        (*p5).next = null_mut();
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
            (*p51).next = null_mut();
        }
        p5 = p51;
    }
    b
}

pub unsafe fn lshift(b: *mut Bigint, mut k: c_int) -> *mut Bigint {
    let mut i: c_int;
    let mut k1: c_int;
    let n: c_int;
    let mut n1: c_int;
    let b1: *mut Bigint;
    let mut x: *mut u32;
    let mut x1: *mut u32;
    let xe: *mut u32;
    let mut z: u32;

    n = k >> 5;
    k1 = (*b).k;
    n1 = n + (*b).wds + 1;
    i = (*b).maxwds;
    while n1 > i {
        i <<= 1;
        k1 += 1;
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
    let mut xa: *mut u32;
    let xa0: *mut u32;
    let mut xb: *mut u32;
    let xb0: *mut u32;
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

pub unsafe fn diff(mut a: *mut Bigint, mut b: *mut Bigint) -> *mut Bigint {
    let mut c: *mut Bigint;
    let mut i: c_int;
    let mut wa: c_int;
    let wb: c_int;
    let mut xa: *mut u32;
    let xae: *mut u32;
    let mut xb: *mut u32;
    let xbe: *mut u32;
    let mut xc: *mut u32;
    let mut borrow: u64;
    let mut y: u64;

    i = cmp(a, b);
    if i == 0 {
        c = Balloc(0);
        (*c).wds = 1;
        *bx(c) = 0;
        return c;
    }
    if i < 0 {
        c = a;
        a = b;
        b = c;
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
        y = (*xa as u64)
            .wrapping_sub(*xb as u64)
            .wrapping_sub(borrow);
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
        y = (*xa as u64).wrapping_sub(borrow);
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

pub unsafe fn ulp(x: *mut U) -> f64 {
    let L: u32;
    let mut u = U { LL: 0 };

    L = ((*x).L[1] & 0x7ff00000).wrapping_sub(52 * 0x100000);
    u.L[1] = L;
    u.L[0] = 0;
    u.d
}

pub unsafe fn b2d(a: *mut Bigint, e: *mut c_int) -> f64 {
    let mut xa: *mut u32;
    let xa0: *mut u32;
    let w: u32;
    let mut y: u32;
    let z: u32;
    let mut k: c_int;
    let mut d = U { LL: 0 };

    xa0 = bx(a);
    xa = xa0.add((*a).wds as usize);
    xa = xa.sub(1);
    y = *xa;
    k = hi0bits(y);
    *e = 32 - k;
    if k < 11 {
        d.L[1] = 0x3ff00000 | (y >> (11 - k));
        w = if xa > xa0 {
            xa = xa.sub(1);
            *xa
        } else {
            0
        };
        d.L[0] = (y << (21 + k)) | (w >> (11 - k));
        return d.d;
    }
    z = if xa > xa0 {
        xa = xa.sub(1);
        *xa
    } else {
        0
    };
    k -= 11;
    if k != 0 {
        d.L[1] = 0x3ff00000 | (y << k) | (z >> (32 - k));
        y = if xa > xa0 {
            xa = xa.sub(1);
            *xa
        } else {
            0
        };
        d.L[0] = (z << k) | (y >> (32 - k));
    } else {
        d.L[1] = 0x3ff00000 | y;
        d.L[0] = z;
    }
    d.d
}

pub unsafe fn d2b(d: *mut U, e: *mut c_int, bits: *mut c_int) -> *mut Bigint {
    let b: *mut Bigint;
    let de: c_int;
    let mut k: c_int;
    let x: *mut u32;
    let mut y: u32;
    let mut z: u32;
    let i: c_int;

    b = Balloc(1);
    x = bx(b);

    z = (*d).L[1] & 0xfffff;
    (*d).L[1] &= 0x7fffffff;
    de = ((*d).L[1] >> 20) as c_int;
    if de != 0 {
        z |= 0x100000;
    }
    y = (*d).L[0];
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
        *e = de - 1023 - 52 + k;
        *bits = 53 - k;
    } else {
        *e = de - 1023 - 52 + 1 + k;
        *bits = 32 * i - hi0bits(*x.add((i - 1) as usize));
    }
    b
}

pub unsafe fn ratio(a: *mut Bigint, b: *mut Bigint) -> f64 {
    let mut da = U { LL: 0 };
    let mut db = U { LL: 0 };
    let mut k: c_int;
    let mut ka: c_int = 0;
    let mut kb: c_int = 0;

    da.d = b2d(a, &mut ka);
    db.d = b2d(b, &mut kb);
    k = ka - kb + 32 * ((*a).wds - (*b).wds);
    if k > 0 {
        da.L[1] = da.L[1].wrapping_add((k as u32).wrapping_mul(0x100000));
    } else {
        k = -k;
        db.L[1] = db.L[1].wrapping_add((k as u32).wrapping_mul(0x100000));
    }
    da.d / db.d
}

pub unsafe fn matchstr(sp: *mut *const c_char, t: *const c_char) -> c_int {
    let mut c: c_int;
    let mut d: c_int;
    let mut s: *const c_char = *sp;
    let mut t = t;

    loop {
        d = *t as c_int;
        t = t.add(1);
        if d == 0 {
            break;
        }
        s = s.add(1);
        c = *s as c_int;
        if c >= 'A' as c_int && c <= 'Z' as c_int {
            c += 'a' as c_int - 'A' as c_int;
        }
        if c != d {
            return 0;
        }
    }
    *sp = s.add(1);
    1
}

pub unsafe fn hexnan(rvp: *mut U, sp: *mut *const c_char) {
    let mut c: u32;
    let mut x: [u32; 2] = [0, 0];
    let mut s: *const c_char;
    let mut c1: u32;
    let mut havedig: c_int;
    let mut udx0: c_int;
    let mut xshift: c_int;

    havedig = 0;
    xshift = 0;
    udx0 = 1;
    s = *sp;
    loop {
        c = *(s.add(1) as *const u8) as u32;
        if c == 0 || c > b' ' as u32 {
            break;
        }
        s = s.add(1);
    }
    if *s.add(1) as u8 == b'0' && (*s.add(2) as u8 == b'x' || *s.add(2) as u8 == b'X') {
        s = s.add(2);
    }
    loop {
        s = s.add(1);
        c = *(s as *const u8) as u32;
        if c == 0 {
            break;
        }
        c1 = HEXDIG[c as usize] as u32;
        if c1 != 0 {
            c = c1 & 0xf;
        } else if c <= b' ' as u32 {
            if udx0 != 0 && havedig != 0 {
                udx0 = 0;
                xshift = 1;
            }
            continue;
        } else {
            loop {
                if c == b')' as u32 {
                    *sp = s.add(1);
                    break;
                }
                s = s.add(1);
                c = *(s as *const u8) as u32;
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
        (*rvp).L[1] = 0x7ff00000 | x[0];
        (*rvp).L[0] = x[1];
    }
}

pub unsafe fn increment(mut b: *mut Bigint) -> *mut Bigint {
    let mut x: *mut u32;
    let xe: *mut u32;
    let b1: *mut Bigint;

    x = bx(b);
    xe = x.add((*b).wds as usize);
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
        b1 = Balloc((*b).k + 1);
        Bcopy(b1, b);
        Bfree(b);
        b = b1;
    }
    *bx(b).add((*b).wds as usize) = 1;
    (*b).wds += 1;
    b
}

pub unsafe fn rshift(b: *mut Bigint, mut k: c_int) {
    let mut x: *mut u32;
    let mut x1: *mut u32;
    let xe: *mut u32;
    let mut y: u32;
    let mut n: c_int;

    x = bx(b);
    x1 = x;
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
        *bx(b) = 0;
    }
}

pub unsafe fn any_on(b: *mut Bigint, mut k: c_int) -> u32 {
    let mut n: c_int;
    let nwds: c_int;
    let mut x: *mut u32;
    let x0: *mut u32;
    let mut x1: u32;
    let x2: u32;

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

/* ------------------------------------------------------------------ gethex */

const emax_gethex: c_int = 0x7fe - 1023 - 53 + 1;
const emin_gethex: c_int = -1022 - 53 + 1;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethex(
    sp: *mut *const c_char,
    rvp: *mut U,
    rounding: c_int,
    sign: c_int,
) -> () {
    let mut b: *mut Bigint = null_mut();
    let mut d: c_char;
    let mut decpt: *const u8;
    let mut s0: *const u8;
    let mut s: *const u8;
    let s1: *const u8;
    let mut e: c_int;
    let mut e1: c_int;
    let mut L: u32;
    let mut lostbits: u32;
    let mut x: *mut u32;
    let mut big: c_int;
    let mut denorm: c_int;
    let mut esign: c_int;
    let mut havedig: c_int;
    let mut k: c_int;
    let mut n: c_int;
    let nb: c_int;
    let mut nbits: c_int;
    let nz: c_int;
    let mut up: c_int;
    let mut zret: c_int;
    let mut check_denorm: c_int = 0;

    havedig = 0;
    s0 = (*sp as *const u8).add(2);
    while *s0.add(havedig as usize) == b'0' {
        havedig += 1;
    }
    s0 = s0.add(havedig as usize);
    s = s0;
    decpt = core::ptr::null();
    zret = 0;
    e = 0;

    'pcheck: {
        if HEXDIG[*s as usize] != 0 {
            havedig += 1;
        } else {
            zret = 1;
            if *s != b'.' {
                break 'pcheck;
            }
            s = s.add(1);
            decpt = s;
            if HEXDIG[*s as usize] == 0 {
                break 'pcheck;
            }
            while *s == b'0' {
                s = s.add(1);
            }
            if HEXDIG[*s as usize] != 0 {
                zret = 0;
            }
            havedig = 1;
            s0 = s;
        }
        while HEXDIG[*s as usize] != 0 {
            s = s.add(1);
        }
        if *s == b'.' && decpt.is_null() {
            s = s.add(1);
            decpt = s;
            while HEXDIG[*s as usize] != 0 {
                s = s.add(1);
            }
        }
        if !decpt.is_null() {
            e = -((s.offset_from(decpt) as c_int) << 2);
        }
    }

    /* pcheck: */
    s1 = s;
    big = 0;
    esign = 0;
    if *s == b'p' || *s == b'P' {
        s = s.add(1);
        if *s == b'-' {
            esign = 1;
            s = s.add(1);
        } else if *s == b'+' {
            s = s.add(1);
        }
        n = HEXDIG[*s as usize] as c_int;
        if n == 0 || n > 0x19 {
            s = s1;
        } else {
            e1 = n - 0x10;
            loop {
                s = s.add(1);
                n = HEXDIG[*s as usize] as c_int;
                if !(n != 0 && n <= 0x19) {
                    break;
                }
                if (e1 as u32) & 0xf8000000 != 0 {
                    big = 1;
                }
                e1 = 10 * e1 + n - 0x10;
            }
            if esign != 0 {
                e1 = -e1;
            }
            e += e1;
        }
    }
    *sp = s as *const c_char;
    if havedig == 0 {
        *sp = (s0 as *const c_char).sub(1);
    }
    if zret != 0 {
        /* retz1 */
        (*rvp).d = 0.0;
        return;
    }
    if big != 0 {
        if esign != 0 {
            /* case Round_up:   if (sign) break; goto ret_tiny;
               case Round_down: if (!sign) break; goto ret_tiny;
               (any other rounding falls out of the switch) */
            let tiny = (rounding == Round_up && sign == 0)
                || (rounding == Round_down && sign != 0);
            if tiny {
                /* ret_tiny */
                crate::types::set_errno(ERANGE);
                (*rvp).L[1] = 0;
                (*rvp).L[0] = 1;
                return;
            }
            /* goto retz */
            crate::types::set_errno(ERANGE);
            (*rvp).d = 0.0;
            return;
        }
        let ovfl1 = rounding == Round_near
            || (rounding == Round_up && sign == 0)
            || (rounding == Round_down && sign != 0);
        if ovfl1 {
            crate::types::set_errno(ERANGE);
            (*rvp).L[1] = 0x7ff00000;
            (*rvp).L[0] = 0;
            return;
        }
        /* ret_big */
        (*rvp).L[1] = 0xfffff | 0x100000 * (1024 + 1023 - 1);
        (*rvp).L[0] = 0xffffffff;
        return;
    }

    n = (s1.offset_from(s0) as c_int) - 1;
    k = 0;
    while n > (1 << 3) - 1 {
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
    let mut s1w = s1;
    while s1w > s0 {
        s1w = s1w.sub(1);
        if *s1w == b'.' {
            continue;
        }
        d = HEXDIG[*s1w as usize] as c_char;
        if d != 0 {
            havedig = 1;
        } else if havedig == 0 {
            e += 4;
            continue;
        }
        if n == 32 {
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
    n = x.offset_from(bx(b)) as c_int;
    (*b).wds = n;
    nb = 32 * n - hi0bits(L);
    nbits = 53;
    lostbits = 0;
    x = bx(b);
    if nb > nbits {
        n = nb - nbits;
        if any_on(b, n) != 0 {
            lostbits = 1;
            k = n - 1;
            if *x.add((k >> 5) as usize) & (1u32 << (k & 31)) != 0 {
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
    if e > emax_gethex {
        /* ovfl */
        Bfree(b);
        crate::types::set_errno(ERANGE);
        (*rvp).L[1] = 0x7ff00000;
        (*rvp).L[0] = 0;
        return;
    }
    denorm = 0;
    let mut goto_normal = false;
    if e < emin_gethex {
        denorm = 1;
        n = emin_gethex - e;
        if n >= nbits {
            let mut tinyf = false;
            if rounding == Round_near {
                if n == nbits && (n < 2 || lostbits != 0 || any_on(b, n - 1) != 0) {
                    tinyf = true;
                }
            } else if rounding == Round_up {
                if sign == 0 {
                    tinyf = true;
                }
            } else if rounding == Round_down && sign != 0 {
                tinyf = true;
            }
            if tinyf {
                /* ret_tinyf */
                Bfree(b);
                crate::types::set_errno(ERANGE);
                (*rvp).L[1] = 0;
                (*rvp).L[0] = 1;
                return;
            }
            Bfree(b);
            /* retz */
            crate::types::set_errno(ERANGE);
            (*rvp).d = 0.0;
            return;
        }
        k = n - 1;
        if k == 0 {
            let mut do_emin_check = false;
            if rounding == Round_near {
                if (*bx(b).add(0) & 3) == 3 || (lostbits != 0 && (*bx(b).add(0) & 1) != 0) {
                    multadd(b, 1, 1);
                    do_emin_check = true;
                }
            } else if rounding == Round_up {
                if sign == 0 && (lostbits != 0 || (*bx(b).add(0) & 1) != 0) {
                    /* incr_denorm */
                    multadd(b, 1, 2);
                    check_denorm = 1;
                    lostbits = 0;
                    do_emin_check = true;
                }
            } else if rounding == Round_down
                && sign != 0
                && (lostbits != 0 || (*bx(b).add(0) & 1) != 0)
            {
                /* incr_denorm */
                multadd(b, 1, 2);
                check_denorm = 1;
                lostbits = 0;
                do_emin_check = true;
            }
            if do_emin_check {
                /* emin_check */
                if *bx(b).add(1) == (1u32 << 21) {
                    rshift(b, 1);
                    e = emin_gethex;
                    goto_normal = true;
                }
            }
        }
        if !goto_normal {
            let mut skip = false;
            if lostbits != 0 {
                lostbits = 1;
            } else if k > 0 {
                lostbits = any_on(b, k);
            } else if check_denorm != 0 {
                skip = true;
            }
            if !skip && *x.add((k >> 5) as usize) & (1u32 << (k & 31)) != 0 {
                lostbits |= 2;
            }
            /* no_lostbits: */
            nbits -= n;
            rshift(b, n);
            e = emin_gethex;
        }
    }
    if !goto_normal {
        if lostbits != 0 {
            up = 0;
            if rounding == Round_zero {
                /* nothing */
            } else if rounding == Round_near {
                if lostbits & 2 != 0 && ((lostbits & 1) | (*x.add(0) & 1)) != 0 {
                    up = 1;
                }
            } else if rounding == Round_up {
                up = 1 - sign;
            } else if rounding == Round_down {
                up = sign;
            }
            if up != 0 {
                k = (*b).wds;
                b = increment(b);
                x = bx(b);
                if denorm == 0 && {
                    n = nbits & 31;
                    (*b).wds > k || (n != 0 && hi0bits(*x.add((k - 1) as usize)) < 32 - n)
                } {
                    rshift(b, 1);
                    e += 1;
                    if e > 1023 {
                        /* ovfl */
                        Bfree(b);
                        crate::types::set_errno(ERANGE);
                        (*rvp).L[1] = 0x7ff00000;
                        (*rvp).L[0] = 0;
                        return;
                    }
                }
            }
        }
        if denorm != 0 {
            (*rvp).L[1] = if (*b).wds > 1 {
                *bx(b).add(1) & !0x100000
            } else {
                0
            };
        } else {
            (*rvp).L[1] =
                (*bx(b).add(1) & !0x100000) | (((e as u32).wrapping_add(0x3ff + 52)) << 20);
        }
    } else {
        /* normal: */
        (*rvp).L[1] = (*bx(b).add(1) & !0x100000) | (((e as u32).wrapping_add(0x3ff + 52)) << 20);
    }
    (*rvp).L[0] = *bx(b).add(0);
    Bfree(b);
}

/* ------------------------------------------------------------------ dtoa */

pub unsafe fn dshift(b: *mut Bigint, p2: c_int) -> c_int {
    let mut rv = hi0bits(*bx(b).add(((*b).wds - 1) as usize)) - 4;
    if p2 > 0 {
        rv -= p2;
    }
    rv & 31
}

pub unsafe fn quorem(b: *mut Bigint, S: *mut Bigint) -> c_int {
    let mut n: c_int;
    let mut bx_: *mut u32;
    let mut bxe: *mut u32;
    let mut q: u32;
    let mut sx: *mut u32;
    let sxe: *mut u32;
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
    bx_ = bx(b);
    bxe = bx_.add(n as usize);
    q = *bxe / (*sxe + 1);
    if q != 0 {
        borrow = 0;
        carry = 0;
        loop {
            ys = (*sx as u64).wrapping_mul(q as u64).wrapping_add(carry);
            sx = sx.add(1);
            carry = ys >> 32;
            y = (*bx_ as u64)
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
            bx_ = bx(b);
            loop {
                bxe = bxe.sub(1);
                if !(bxe > bx_ && *bxe == 0) {
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
        bx_ = bx(b);
        sx = bx(S);
        loop {
            ys = (*sx as u64).wrapping_add(carry);
            sx = sx.add(1);
            carry = ys >> 32;
            y = (*bx_ as u64)
                .wrapping_sub(ys & 0xffffffff)
                .wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            *bx_ = (y & 0xffffffff) as u32;
            bx_ = bx_.add(1);
            if sx > sxe {
                break;
            }
        }
        bx_ = bx(b);
        bxe = bx_.add(n as usize);
        if *bxe == 0 {
            loop {
                bxe = bxe.sub(1);
                if !(bxe > bx_ && *bxe == 0) {
                    break;
                }
                n -= 1;
            }
            (*b).wds = n;
        }
    }
    q as c_int
}

unsafe fn rv_alloc(i: c_int) -> *mut c_char {
    let mut j: usize;
    let mut k: c_int;
    let r: *mut c_int;

    j = 4;
    k = 0;
    while core::mem::size_of::<Bigint>() - 4 - 4 + j <= i as usize {
        j <<= 1;
        k += 1;
    }
    r = Balloc(k) as *mut c_int;
    *r = k;
    dtoa_result = r.add(1) as *mut c_char;
    dtoa_result
}

unsafe fn nrv_alloc(
    s: *const c_char,
    mut s0: *mut c_char,
    s0len: usize,
    rve: *mut *mut c_char,
    n: c_int,
) -> *mut c_char {
    let rv: *mut c_char;
    let mut t: *mut c_char;
    let mut s = s;

    if s0.is_null() {
        s0 = rv_alloc(n);
    } else if s0len <= n as usize {
        rv = null_mut();
        t = n as usize as *mut c_char;
        if !rve.is_null() {
            *rve = t;
        }
        return rv;
    }
    rv = s0;
    t = s0;
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
    let k = *(b as *mut c_int);
    (*b).k = k;
    (*b).maxwds = 1 << k;
    Bfree(b);
    if s == dtoa_result {
        dtoa_result = null_mut();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum St {
    Start,
    UseExact,
    NoDiv,
    Toobig,
    FastFailed,
    FastFailed1,
    NoDigits,
    OneDigit,
    Roundup,
    Roundoff,
    Ret,
    Retc,
    Ret1,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa_r(
    dd: f64,
    mode: c_int,
    ndigits: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
    buf: *mut c_char,
    blen: usize,
) -> *mut c_char {
    let mut mode = mode;
    let mut ndigits = ndigits;
    let mut buf = buf;
    let mut blen = blen;

    let mut bbits: c_int = 0;
    let mut b2: c_int;
    let mut b5: c_int;
    let mut be: c_int;
    let mut dig: c_int = 0;
    let mut i: c_int;
    let mut ilim: c_int;
    let mut _ilim1: c_int;
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
    let mut u = U { LL: 0 };
    let mut s: *mut c_char;
    let mut p10: *const BF96;
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
    let mut sres: u64;
    let mut sulp: u64;
    let mut tv0: u64;
    let mut tv1: u64;
    let mut tv2: u64;
    let mut tv3: u64;
    let mut ulp_: u64 = 0;
    let mut ulplo: u64 = 0;
    let mut ulpmask: u64 = 0;
    let mut ures: u64;
    let mut ureslo: u64;
    let mut zb: u64;
    let mut eulp: c_int = 0;
    let mut k1: c_int = 0;
    let mut n2: c_int = 0;
    let mut ulpadj: c_int;
    let mut ulpshift: c_int;

    u.d = dd;
    if u.L[1] & 0x80000000 != 0 {
        *sign = 1;
        u.L[1] &= !0x80000000;
    } else {
        *sign = 0;
    }

    if u.L[1] & 0x7ff00000 == 0x7ff00000 {
        *decpt = 9999;
        if u.L[0] == 0 && (u.L[1] & 0xfffff) == 0 {
            return nrv_alloc(b"Infinity\0".as_ptr() as *const c_char, buf, blen, rve, 8);
        }
        return nrv_alloc(b"NaN\0".as_ptr() as *const c_char, buf, blen, rve, 3);
    }
    if u.d == 0.0 {
        *decpt = 1;
        return nrv_alloc(b"0\0".as_ptr() as *const c_char, buf, blen, rve, 1);
    }

    dbits = (u.LL & 0xfffffffffffff) << 11;
    be = (u.LL >> 52) as c_int;
    if be != 0 {
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

    j = LHINT[(be + 51) as usize] as c_int;
    p10 = PTEN.as_ptr().add(j as usize);
    dbhi = dbits >> 32;
    dblo = dbits & 0xffffffff;
    i = be - 0x3fe;
    if i < (*p10).e
        || (i == (*p10).e
            && (dbhi < (*p10).b0 as u64
                || (dbhi == (*p10).b0 as u64 && dblo < (*p10).b1 as u64)))
    {
        j -= 1;
    }
    k = j - 342;

    if mode < 0 || mode > 9 {
        mode = 0;
    }
    if mode > 5 {
        mode -= 4;
    }
    leftright = 1;
    ilim = -1;
    _ilim1 = -1;
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
            i = ndigits;
            ilim = i;
            _ilim1 = i;
        }
        3 | 5 => {
            if mode == 3 {
                leftright = 0;
            }
            i = ndigits + k + 1;
            ilim = i;
            _ilim1 = i - 1;
            if i <= 0 {
                i = 1;
            }
        }
        _ => {}
    }

    if buf.is_null() {
        buf = rv_alloc(i);
        blen = core::mem::size_of::<Bigint>()
            + ((1usize << *(buf as *mut c_int).sub(1)) - 1) * 4
            - 4;
    } else if blen <= i as usize {
        buf = null_mut();
        if !rve.is_null() {
            *rve = (i as usize) as *mut c_char;
        }
        return buf;
    }
    s = buf;

    spec_case = 0;
    if mode < 2 || leftright != 0 {
        if u.L[0] == 0 && (u.L[1] & 0xfffff) == 0 && (u.L[1] & 0x7fe00000) != 0 {
            spec_case = 1;
        }
    }

    b = null_mut();
    let mut state;
    if ilim < 0 && (mode == 3 || mode == 5) {
        S = null_mut();
        mhi = null_mut();
        state = St::NoDigits;
    } else {
        state = St::Start;
    }

    i = 1;
    j = 52 + 0x3ff - be;
    ulpshift = 0;
    ulplo = 0;

    'sm: loop {
        match state {
            St::Start => {
                if k < 0 {
                    if k < -25 {
                        state = St::Toobig;
                        continue 'sm;
                    }
                    res = dbits >> 11;
                    k1 = -(k + 1);
                    n2 = PFIVEBITS[k1 as usize] + 53;
                    j1 = j;
                    if n2 > 61 {
                        ulpshift = n2 - 61;
                        ulpmask = (1u64 << ulpshift) - 1;
                        if res & ulpmask != 0 {
                            state = St::Toobig;
                            continue 'sm;
                        }
                        j -= ulpshift;
                        res >>= ulpshift;
                    }
                    ulp_ = PFIVE[k1 as usize];
                    res = res.wrapping_mul(ulp_);
                    if ulpshift != 0 {
                        ulplo = ulp_;
                        ulp_ >>= ulpshift;
                    }
                    j += k;
                    if ilim == 0 {
                        S = null_mut();
                        mhi = null_mut();
                        if res > (5u64 << j) {
                            state = St::OneDigit;
                        } else {
                            state = St::NoDigits;
                        }
                        continue 'sm;
                    }
                    state = St::NoDiv;
                    continue 'sm;
                }
                if ilim == 0 && j + k >= 0 {
                    S = null_mut();
                    mhi = null_mut();
                    if (dbits >> 11) > (pfive_at(k - 1) << j) {
                        state = St::OneDigit;
                    } else {
                        state = St::NoDigits;
                    }
                    continue 'sm;
                }
                if k <= read_volatile(core::ptr::addr_of!(dtoa_divmax)) && j + k >= 0 {
                    state = St::UseExact;
                    continue 'sm;
                }
                state = St::Toobig;
                continue 'sm;
            }

            St::UseExact => {
                res = dbits >> 11;
                ulp_ = 1;
                if k <= 0 {
                    state = St::NoDiv;
                    continue 'sm;
                }
                j1 = j + k + 1;
                den = pfive_at(k - i) << (j1 - i);
                loop {
                    dig = (res / den) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res = res.wrapping_sub((dig as u64).wrapping_mul(den));
                    if res == 0 {
                        state = St::Retc;
                        continue 'sm;
                    }
                    if ilim < 0 {
                        ures = den - res;
                        if 2u64.wrapping_mul(res) <= ulp_
                            && (if spec_case != 0 {
                                4u64.wrapping_mul(res) <= ulp_
                            } else {
                                2u64.wrapping_mul(res) < ulp_ || dig & 1 != 0
                            })
                        {
                            /* ulp_reached */
                            if ures < res || (ures == res && dig & 1 != 0) {
                                state = St::Roundup;
                            } else {
                                state = St::Retc;
                            }
                            continue 'sm;
                        }
                        if 2u64.wrapping_mul(ures) < ulp_ {
                            state = St::Roundup;
                            continue 'sm;
                        }
                    } else if i == ilim {
                        ures = 2u64.wrapping_mul(res);
                        if ures > den
                            || (ures == den && dig & 1 != 0)
                            || (spec_case != 0 && res <= ulp_ && 2u64.wrapping_mul(res) >= ulp_)
                        {
                            state = St::Roundup;
                        } else {
                            state = St::Retc;
                        }
                        continue 'sm;
                    }
                    i += 1;
                    if j1 < i {
                        res = res.wrapping_mul(10);
                        ulp_ = ulp_.wrapping_mul(10);
                    } else {
                        if i > k {
                            break;
                        }
                        den = pfive_at(k - i) << (j1 - i);
                    }
                }
                /* the `break` above falls straight into the `no_div` loop */
                state = St::NoDiv;
                continue 'sm;
            }

            St::NoDiv => {
                loop {
                    den = res >> j;
                    dig = den as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res = res.wrapping_sub(den << j);
                    if res == 0 {
                        state = St::Retc;
                        continue 'sm;
                    }
                    if ilim < 0 {
                        ures = (1u64 << j).wrapping_sub(res);
                        if 2u64.wrapping_mul(res) <= ulp_
                            && (if spec_case != 0 {
                                4u64.wrapping_mul(res) <= ulp_
                            } else {
                                2u64.wrapping_mul(res) < ulp_ || dig & 1 != 0
                            })
                        {
                            /* ulp_reached */
                            if ures < res || (ures == res && dig & 1 != 0) {
                                state = St::Roundup;
                            } else {
                                state = St::Retc;
                            }
                            continue 'sm;
                        }
                        if 2u64.wrapping_mul(ures) < ulp_ {
                            state = St::Roundup;
                            continue 'sm;
                        }
                    }
                    j -= 1;
                    if i == ilim {
                        hb = 1u64 << j;
                        if res & hb != 0 && (dig & 1 != 0 || res & (hb - 1) != 0) {
                            state = St::Roundup;
                            continue 'sm;
                        }
                        if spec_case != 0 && res <= ulp_ && 2u64.wrapping_mul(res) >= ulp_ {
                            state = St::Roundup;
                        } else {
                            state = St::Retc;
                        }
                        continue 'sm;
                    }
                    i += 1;
                    res = res.wrapping_mul(5);
                    if ulpshift != 0 {
                        ulplo = 5u64.wrapping_mul(ulplo & ulpmask);
                        ulp_ = 5u64.wrapping_mul(ulp_).wrapping_add(ulplo >> ulpshift);
                    } else {
                        ulp_ = ulp_.wrapping_mul(5);
                    }
                }
            }

            St::Toobig => {
                if ilim > 28 {
                    state = St::FastFailed1;
                    continue 'sm;
                }
                p10 = PTEN.as_ptr().add((342 - k) as usize);
                tv0 = ((*p10).b2 as u64).wrapping_mul(dblo);
                tv1 = ((*p10).b1 as u64).wrapping_mul(dblo).wrapping_add(tv0 >> 32);
                tv2 = ((*p10).b2 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv1 & 0xffffffff);
                tv3 = ((*p10).b0 as u64)
                    .wrapping_mul(dblo)
                    .wrapping_add(tv1 >> 32)
                    .wrapping_add(tv2 >> 32);
                res3 = ((*p10).b1 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv3 & 0xffffffff);
                res = ((*p10).b0 as u64)
                    .wrapping_mul(dbhi)
                    .wrapping_add(tv3 >> 32)
                    .wrapping_add(res3 >> 32);
                be += (*p10).e - 0x3fe;
                j1 = be - 54 + ulpadj;
                eulp = j1;
                if res & 0x8000000000000000 == 0 {
                    be -= 1;
                    res3 <<= 1;
                    res = (res << 1) | ((res3 & 0x100000000) >> 32);
                }
                res0 = res;
                if ilim > 19 {
                    state = St::FastFailed;
                    continue 'sm;
                }
                res >>= 4 - be;
                ulp_ = (*p10).b0 as u64;
                ulp_ = (ulp_ << 29) | (((*p10).b1 as u64) >> 3);
                if ilim == 0 {
                    if res & 0x7fffffffffffffe == 0 || (!res) & 0x7fffffffffffffe == 0 {
                        state = St::FastFailed1;
                        continue 'sm;
                    }
                    S = null_mut();
                    mhi = null_mut();
                    if res >= 0x5000000000000000 {
                        state = St::OneDigit;
                    } else {
                        state = St::NoDigits;
                    }
                    continue 'sm;
                }
                rb = 1;
                loop {
                    dig = (res >> 60) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res &= 0xfffffffffffffff;
                    if ilim < 0 {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if eulp > 0 {
                            sulp = ulp_ << (eulp - 1);
                            if res <= ures {
                                if res + rb > ures - rb {
                                    state = St::FastFailed;
                                    continue 'sm;
                                }
                                if res < sulp {
                                    state = St::Retc;
                                    continue 'sm;
                                }
                            } else {
                                if res - rb <= ures + rb {
                                    state = St::FastFailed;
                                    continue 'sm;
                                }
                                if ures < sulp {
                                    state = St::Roundup;
                                    continue 'sm;
                                }
                            }
                        } else {
                            zb = (1u64 << (eulp + 63)).wrapping_neg();
                            if zb & res == 0 {
                                sres = res << (1 - eulp);
                                if sres < ulp_
                                    && (spec_case == 0 || 2u64.wrapping_mul(sres) < ulp_)
                                {
                                    if (res + rb) << (1 - eulp) >= ulp_ {
                                        state = St::FastFailed;
                                        continue 'sm;
                                    }
                                    if ures < res {
                                        if ures + rb >= res - rb {
                                            state = St::FastFailed;
                                            continue 'sm;
                                        }
                                        state = St::Roundup;
                                        continue 'sm;
                                    }
                                    if ures - rb < res + rb {
                                        state = St::FastFailed;
                                        continue 'sm;
                                    }
                                    state = St::Retc;
                                    continue 'sm;
                                }
                            }
                            if zb & ures == 0 && ures << (-eulp) < ulp_ {
                                if ures << (1 - eulp) < ulp_ {
                                    state = St::Roundup;
                                } else {
                                    state = St::FastFailed;
                                }
                                continue 'sm;
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        if ures < res {
                            if ures <= rb || res - rb <= ures + rb {
                                if j + k >= 0 && k >= 0 && k <= 27 {
                                    /* use_exact1 */
                                    s = buf;
                                    i = 1;
                                    state = St::UseExact;
                                    continue 'sm;
                                }
                                state = St::FastFailed;
                                continue 'sm;
                            }
                            state = St::Roundup;
                            continue 'sm;
                        }
                        if res <= rb || ures - rb <= res + rb {
                            if j + k >= 0 && k >= 0 && k <= 27 {
                                /* use_exact1 */
                                s = buf;
                                i = 1;
                                state = St::UseExact;
                                continue 'sm;
                            }
                            state = St::FastFailed;
                            continue 'sm;
                        }
                        state = St::Retc;
                        continue 'sm;
                    }
                    rb = rb.wrapping_mul(10);
                    if rb >= 0x1000000000000000 {
                        state = St::FastFailed;
                        continue 'sm;
                    }
                    res = res.wrapping_mul(10);
                    ulp_ = ulp_.wrapping_mul(5);
                    if ulp_ & 0x8000000000000000 != 0 {
                        eulp += 4;
                        ulp_ >>= 3;
                    } else {
                        eulp += 3;
                        ulp_ >>= 2;
                    }
                    i += 1;
                }
            }

            St::FastFailed => {
                s = buf;
                i = 4 - be;
                res = res0 >> i;
                reslo = 0xffffffff & res3;
                if i != 0 {
                    reslo = ((res0 << (64 - i)) >> 32) | (reslo >> i);
                }
                rb = 0;
                rblo = 4;
                ulp_ = (*p10).b0 as u64;
                ulp_ = (ulp_ << 29) | (((*p10).b1 as u64) >> 3);
                eulp = j1;
                i = 1;
                'ff: loop {
                    let mut goto_more96 = false;
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
                            sulp = (ulp_ << (eulp - 1)).wrapping_sub(rb);
                            if res <= ures {
                                if res < sulp && res + rb < ures - rb {
                                    state = St::Retc;
                                    continue 'sm;
                                }
                            } else if ures < sulp && res - rb > ures + rb {
                                state = St::Roundup;
                                continue 'sm;
                            }
                            state = St::FastFailed1;
                            continue 'sm;
                        } else {
                            zb = (1u64 << (eulp + 60)).wrapping_neg();
                            if zb & (res + rb) == 0 {
                                sres = (res - rb) << (1 - eulp);
                                if sres < ulp_
                                    && (spec_case == 0 || 2u64.wrapping_mul(sres) < ulp_)
                                {
                                    sres = res << (1 - eulp);
                                    j = eulp + 31;
                                    if j > 0 {
                                        sres = sres.wrapping_add((rblo + reslo) >> j);
                                    } else {
                                        sres = sres.wrapping_add((rblo + reslo) << (-j));
                                    }
                                    if sres.wrapping_add(rb << (1 - eulp)) >= ulp_ {
                                        state = St::FastFailed1;
                                        continue 'sm;
                                    }
                                    if sres >= ulp_ {
                                        goto_more96 = true;
                                    } else if ures < res || (ures == res && ureslo < reslo) {
                                        if ures + rb >= res - rb {
                                            state = St::FastFailed1;
                                            continue 'sm;
                                        }
                                        state = St::Roundup;
                                        continue 'sm;
                                    } else if ures - rb <= res + rb {
                                        state = St::FastFailed1;
                                        continue 'sm;
                                    } else {
                                        state = St::Retc;
                                        continue 'sm;
                                    }
                                }
                            }
                            if !goto_more96
                                && zb & ures == 0
                                && (ures - rb) << (1 - eulp) < ulp_
                            {
                                if (ures + rb) << (1 - eulp) < ulp_ {
                                    state = St::Roundup;
                                } else {
                                    state = St::FastFailed1;
                                }
                                continue 'sm;
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64.wrapping_sub(res);
                        sres = 0;
                        ureslo = 0;
                        if reslo != 0 {
                            ureslo = 0x100000000u64.wrapping_sub(reslo);
                            ures = ures.wrapping_sub(1);
                            sres = (reslo + rblo) >> 31;
                        }
                        sres = sres.wrapping_add(2u64.wrapping_mul(rb));
                        if ures <= res {
                            if ures <= sres || res - ures <= sres {
                                state = St::FastFailed1;
                            } else {
                                state = St::Roundup;
                            }
                            continue 'sm;
                        }
                        if res <= sres || ures - res <= sres {
                            state = St::FastFailed1;
                        } else {
                            state = St::Retc;
                        }
                        continue 'sm;
                    }
                    let _ = goto_more96;
                    /* more96: */
                    rblo = rblo.wrapping_mul(10);
                    rb = 10u64.wrapping_mul(rb).wrapping_add(rblo >> 32);
                    rblo &= 0xffffffff;
                    if rb >= 0x1000000000000000 {
                        state = St::FastFailed1;
                        continue 'sm;
                    }
                    reslo = reslo.wrapping_mul(10);
                    res = 10u64.wrapping_mul(res).wrapping_add(reslo >> 32);
                    reslo &= 0xffffffff;
                    ulp_ = ulp_.wrapping_mul(5);
                    if ulp_ & 0x8000000000000000 != 0 {
                        eulp += 4;
                        ulp_ >>= 3;
                    } else {
                        eulp += 3;
                        ulp_ >>= 2;
                    }
                    i += 1;
                    continue 'ff;
                }
            }

            St::FastFailed1 => {
                S = null_mut();
                mhi = null_mut();
                mlo = null_mut();
                b = d2b(&mut u, &mut be, &mut bbits);
                s = buf;
                i = ((u.L[1] >> 20) & 0x7ff) as c_int;
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
                mhi = null_mut();
                mlo = null_mut();
                if leftright != 0 {
                    i = if denorm != 0 {
                        be + (1023 + 52 - 1 + 1)
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
                    b2 += 1;
                    s2 += 1;
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
                    if ilim < 0 {
                        state = St::NoDigits;
                        continue 'sm;
                    }
                    S = multadd(S, 5, 0);
                    if cmp(b, S) <= 0 {
                        state = St::NoDigits;
                    } else {
                        state = St::OneDigit;
                    }
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
                        mhi = lshift(mhi, 1);
                    }
                    i = 1;
                    loop {
                        dig = quorem(b, S) + b'0' as c_int;
                        j = cmp(b, mlo);
                        delta = diff(S, mhi);
                        j1 = if (*delta).sign != 0 { 1 } else { cmp(b, delta) };
                        Bfree(delta);
                        if j1 == 0 && mode != 1 && (u.L[0] & 1) == 0 {
                            if dig == b'9' as c_int {
                                /* round_9_up */
                                *s = b'9' as c_char;
                                s = s.add(1);
                                state = St::Roundoff;
                                continue 'sm;
                            }
                            if j > 0 {
                                dig += 1;
                            }
                            *s = dig as c_char;
                            s = s.add(1);
                            state = St::Ret;
                            continue 'sm;
                        }
                        if j < 0 || (j == 0 && mode != 1 && (u.L[0] & 1) == 0) {
                            let mut accept = false;
                            if *bx(b).add(0) == 0 && (*b).wds <= 1 {
                                accept = true;
                            }
                            if !accept && j1 > 0 {
                                b = lshift(b, 1);
                                j1 = cmp(b, S);
                                let old = dig;
                                if j1 > 0 || (j1 == 0 && dig & 1 != 0) {
                                    dig += 1;
                                    if old == b'9' as c_int {
                                        /* round_9_up */
                                        *s = b'9' as c_char;
                                        s = s.add(1);
                                        state = St::Roundoff;
                                        continue 'sm;
                                    }
                                }
                            }
                            /* accept_dig */
                            *s = dig as c_char;
                            s = s.add(1);
                            state = St::Ret;
                            continue 'sm;
                        }
                        if j1 > 0 {
                            if dig == b'9' as c_int {
                                /* round_9_up */
                                *s = b'9' as c_char;
                                s = s.add(1);
                                state = St::Roundoff;
                                continue 'sm;
                            }
                            *s = (dig + 1) as c_char;
                            s = s.add(1);
                            state = St::Ret;
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
                        if *bx(b).add(0) == 0 && (*b).wds <= 1 {
                            state = St::Ret;
                            continue 'sm;
                        }
                        if i >= ilim {
                            break;
                        }
                        b = multadd(b, 10, 0);
                        i += 1;
                    }
                }
                b = lshift(b, 1);
                j = cmp(b, S);
                if j > 0 || (j == 0 && dig & 1 != 0) {
                    state = St::Roundoff;
                } else {
                    state = St::Ret;
                }
                continue 'sm;
            }

            St::NoDigits => {
                k = -1 - ndigits;
                state = St::Ret;
                continue 'sm;
            }

            St::OneDigit => {
                *s = b'1' as c_char;
                s = s.add(1);
                k += 1;
                state = St::Ret;
                continue 'sm;
            }

            St::Roundup => {
                let mut done = false;
                loop {
                    s = s.sub(1);
                    if *s != b'9' as c_char {
                        break;
                    }
                    if s == buf {
                        k += 1;
                        *s = b'1' as c_char;
                        s = s.add(1);
                        done = true;
                        break;
                    }
                }
                if !done {
                    *s += 1;
                    s = s.add(1);
                }
                state = St::Ret1;
                continue 'sm;
            }

            St::Roundoff => {
                let mut done = false;
                loop {
                    s = s.sub(1);
                    if *s != b'9' as c_char {
                        break;
                    }
                    if s == buf {
                        k += 1;
                        *s = b'1' as c_char;
                        s = s.add(1);
                        done = true;
                        break;
                    }
                }
                if !done {
                    *s += 1;
                    s = s.add(1);
                }
                state = St::Ret;
                continue 'sm;
            }

            St::Ret => {
                Bfree(S);
                if !mhi.is_null() {
                    if !mlo.is_null() && mlo != mhi {
                        Bfree(mlo);
                    }
                    Bfree(mhi);
                }
                state = St::Retc;
                continue 'sm;
            }

            St::Retc => {
                while s > buf && *s.sub(1) == b'0' as c_char {
                    s = s.sub(1);
                }
                state = St::Ret1;
                continue 'sm;
            }

            St::Ret1 => {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa(
    dd: f64,
    mode: c_int,
    ndigits: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
) -> *mut c_char {
    if !dtoa_result.is_null() {
        freedtoa(dtoa_result);
    }
    dtoa_r(dd, mode, ndigits, decpt, sign, rve, null_mut(), 0)
}
