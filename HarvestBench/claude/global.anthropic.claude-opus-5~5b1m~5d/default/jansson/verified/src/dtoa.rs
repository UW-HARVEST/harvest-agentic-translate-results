//! Translation of `src/dtoa.c` (David M. Gay's dtoa/strtod).
//!
//! The configuration used by the C build is `IEEE_8087`, `MULTIPLE_THREADS`
//! undefined, `NO_LONG_LONG` undefined, `MALLOC = jsonp_malloc` and
//! `FREE = jsonp_free`.

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::dtoa_tables::*;
use crate::ffi;
use crate::memory::{jsonp_free, jsonp_malloc};

pub type ULong = u32;
pub type ULLong = u64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BF96 {
    pub b0: c_uint,
    pub b1: c_uint,
    pub b2: c_uint,
    pub e: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union U {
    pub d: f64,
    pub L: [ULong; 2],
    pub LL: ULLong,
}

impl U {
    #[inline]
    fn zero() -> U {
        U { LL: 0 }
    }
}

/* IEEE_8087: word0 is the high word (L[1]), word1 the low word (L[0]) */
macro_rules! word0 {
    ($x:expr) => {
        (*$x).L[1]
    };
}
macro_rules! word1 {
    ($x:expr) => {
        (*$x).L[0]
    };
}

#[repr(C)]
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
    pub x: [ULong; 1],
}

const KMAX: c_int = 7;
const PRIVATE_MEM_WORDS: usize = 288; /* (2304 + 7) / 8 */
const OFFSET_OF_SIGN: usize = 16;

struct ThInfo {
    Freelist: [*mut Bigint; (KMAX + 1) as usize],
    P5s: *mut Bigint,
}

static mut PRIVATE_MEM: [f64; PRIVATE_MEM_WORDS] = [0.0; PRIVATE_MEM_WORDS];
static mut PMEM_NEXT: usize = 0;
static mut TI0: ThInfo = ThInfo {
    Freelist: [core::ptr::null_mut(); (KMAX + 1) as usize],
    P5s: core::ptr::null_mut(),
};

/// `int dtoa_divmax = 2;`
#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

static mut DTOA_RESULT: *mut c_char = core::ptr::null_mut();


/* Table accessors. These use unchecked reads so that the (deliberate)
   out-of-range accesses performed by the original C code behave the same way
   instead of panicking. */

#[inline]
unsafe fn pten(i: c_int) -> *const BF96 {
    PTEN.as_ptr().offset(i as isize)
}

#[inline]
unsafe fn lhint(i: c_int) -> c_int {
    *LHINT.as_ptr().offset(i as isize) as c_int
}

/// `pfive[i]`; note that dtoa.c reads `pfive[-1]` in one branch.
#[inline]
unsafe fn pfive(i: c_int) -> ULLong {
    *PFIVE_PADDED.as_ptr().offset((i + 1) as isize)
}

#[inline]
unsafe fn xp(b: *mut Bigint) -> *mut ULong {
    core::ptr::addr_of_mut!((*b).x) as *mut ULong
}

#[inline]
unsafe fn xg(b: *mut Bigint, i: usize) -> ULong {
    *xp(b).add(i)
}

#[inline]
unsafe fn xs(b: *mut Bigint, i: usize, v: ULong) {
    *xp(b).add(i) = v;
}

unsafe fn Balloc(k: c_int) -> *mut Bigint {
    let x: c_int;
    let rv: *mut Bigint;
    let len: c_uint;

    if k <= KMAX && !TI0.Freelist[k as usize].is_null() {
        rv = TI0.Freelist[k as usize];
        TI0.Freelist[k as usize] = (*rv).next;
    } else {
        x = 1 << k;

        len = ((core::mem::size_of::<Bigint>()
            + (x as usize - 1) * core::mem::size_of::<ULong>()
            + core::mem::size_of::<f64>()
            - 1)
            / core::mem::size_of::<f64>()) as c_uint;
        if k <= KMAX && PMEM_NEXT + len as usize <= PRIVATE_MEM_WORDS {
            rv = core::ptr::addr_of_mut!(PRIVATE_MEM[PMEM_NEXT]) as *mut Bigint;
            PMEM_NEXT += len as usize;
        } else {
            rv = jsonp_malloc(len as usize * core::mem::size_of::<f64>()) as *mut Bigint;
        }

        (*rv).k = k;
        (*rv).maxwds = x;
    }

    (*rv).wds = 0;
    (*rv).sign = 0;
    rv
}

unsafe fn Bfree(v: *mut Bigint) {
    if !v.is_null() {
        if (*v).k > KMAX {
            jsonp_free(v as *mut c_void);
        } else {
            (*v).next = TI0.Freelist[(*v).k as usize];
            TI0.Freelist[(*v).k as usize] = v;
        }
    }
}

/// `Bcopy(x, y)`: copies `sign`, `wds` and the used words of `x`.
#[inline]
unsafe fn Bcopy(dst: *mut Bigint, src: *mut Bigint) {
    ffi::memcpy(
        (dst as *mut u8).add(OFFSET_OF_SIGN) as *mut c_void,
        (src as *const u8).add(OFFSET_OF_SIGN) as *const c_void,
        (*src).wds as usize * core::mem::size_of::<c_int>() + 2 * core::mem::size_of::<c_int>(),
    );
}

unsafe fn multadd(b_in: *mut Bigint, m: c_int, a: c_int) -> *mut Bigint {
    let mut b = b_in;
    let mut i: c_int;
    let mut wds: c_int;
    let mut x: *mut ULong;
    let mut carry: ULLong;
    let mut y: ULLong;
    let b1: *mut Bigint;

    wds = (*b).wds;
    x = xp(b);
    i = 0;
    carry = a as ULLong;
    loop {
        y = (*x as ULLong) * (m as ULLong) + carry;
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
            Bcopy(b1, b);
            Bfree(b);
            b = b1;
        }
        xs(b, wds as usize, carry as ULong);
        wds += 1;
        (*b).wds = wds;
    }
    b
}

unsafe fn s2b(s_in: *const c_char, nd0: c_int, nd: c_int, y9: ULong, dplen: c_int) -> *mut Bigint {
    let mut s = s_in;
    let mut b: *mut Bigint;
    let mut i: c_int;
    let mut k: c_int;
    let mut x: c_int;
    let mut y: c_int;

    x = (nd + 8) / 9;
    k = 0;
    y = 1;
    while x > y {
        y <<= 1;
        k += 1;
    }

    b = Balloc(k);
    xs(b, 0, y9);
    (*b).wds = 1;

    i = 9;
    if 9 < nd0 {
        s = s.add(9);
        loop {
            let d = *s as c_int - '0' as c_int;
            s = s.add(1);
            b = multadd(b, 10, d);
            i += 1;
            if i >= nd0 {
                break;
            }
        }
        s = s.offset(dplen as isize);
    } else {
        s = s.offset((dplen + 9) as isize);
    }
    while i < nd {
        let d = *s as c_int - '0' as c_int;
        s = s.add(1);
        b = multadd(b, 10, d);
        i += 1;
    }
    b
}

unsafe fn hi0bits(mut x: ULong) -> c_int {
    let mut k: c_int = 0;

    if (x & 0xffff0000) == 0 {
        k = 16;
        x <<= 16;
    }
    if (x & 0xff000000) == 0 {
        k += 8;
        x <<= 8;
    }
    if (x & 0xf0000000) == 0 {
        k += 4;
        x <<= 4;
    }
    if (x & 0xc0000000) == 0 {
        k += 2;
        x <<= 2;
    }
    if (x & 0x80000000) == 0 {
        k += 1;
        if (x & 0x40000000) == 0 {
            return 32;
        }
    }
    k
}

unsafe fn lo0bits(y: *mut ULong) -> c_int {
    let mut k: c_int;
    let mut x: ULong = *y;

    if (x & 7) != 0 {
        if (x & 1) != 0 {
            return 0;
        }
        if (x & 2) != 0 {
            *y = x >> 1;
            return 1;
        }
        *y = x >> 2;
        return 2;
    }
    k = 0;
    if (x & 0xffff) == 0 {
        k = 16;
        x >>= 16;
    }
    if (x & 0xff) == 0 {
        k += 8;
        x >>= 8;
    }
    if (x & 0xf) == 0 {
        k += 4;
        x >>= 4;
    }
    if (x & 0x3) == 0 {
        k += 2;
        x >>= 2;
    }
    if (x & 1) == 0 {
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
    let b = Balloc(1);
    xs(b, 0, i as ULong);
    (*b).wds = 1;
    b
}

unsafe fn mult(a_in: *mut Bigint, b_in: *mut Bigint) -> *mut Bigint {
    let mut a = a_in;
    let mut b = b_in;
    let c: *mut Bigint;
    let mut k: c_int;
    let wa: c_int;
    let wb: c_int;
    let mut wc: c_int;
    let mut x: *mut ULong;
    let xa: *mut ULong;
    let xae: *mut ULong;
    let mut xb: *mut ULong;
    let xbe: *mut ULong;
    let mut xc: *mut ULong;
    let mut xc0: *mut ULong;
    let mut y: ULong;
    let mut carry: ULLong;
    let mut z: ULLong;

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
    x = xp(c);
    let xend = xp(c).offset(wc as isize);
    while x < xend {
        *x = 0;
        x = x.add(1);
    }
    let xa_p = xp(a);
    xa = xa_p;
    xae = xa.offset(wa as isize);
    xb = xp(b);
    xbe = xb.offset(wb as isize);
    xc0 = xp(c);

    while xb < xbe {
        y = *xb;
        xb = xb.add(1);
        if y != 0 {
            x = xa;
            xc = xc0;
            carry = 0;
            loop {
                z = (*x as ULLong) * (y as ULLong) + (*xc as ULLong) + carry;
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
    xc0 = xp(c);
    xc = xc0.offset(wc as isize);
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

unsafe fn pow5mult(b_in: *mut Bigint, k_in: c_int) -> *mut Bigint {
    let mut b = b_in;
    let mut k = k_in;
    let mut b1: *mut Bigint;
    let mut p5: *mut Bigint;
    let mut p51: *mut Bigint;
    let i: c_int;

    i = k & 3;
    if i != 0 {
        b = multadd(b, P05[(i - 1) as usize], 0);
    }

    k >>= 2;
    if k == 0 {
        return b;
    }
    p5 = TI0.P5s;
    if p5.is_null() {
        p5 = i2b(625);
        TI0.P5s = p5;
        (*p5).next = core::ptr::null_mut();
    }
    loop {
        if (k & 1) != 0 {
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
            (*p51).next = core::ptr::null_mut();
        }
        p5 = p51;
    }
    b
}

unsafe fn lshift(b: *mut Bigint, k_in: c_int) -> *mut Bigint {
    let mut k = k_in;
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
        i <<= 1;
        k1 += 1;
    }
    b1 = Balloc(k1);
    x1 = xp(b1);
    i = 0;
    while i < n {
        *x1 = 0;
        x1 = x1.add(1);
        i += 1;
    }
    x = xp(b);
    xe = x.offset((*b).wds as isize);

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

unsafe fn cmp(a: *mut Bigint, b: *mut Bigint) -> c_int {
    let mut i: c_int;
    let j: c_int;

    i = (*a).wds;
    j = (*b).wds;

    i -= j;
    if i != 0 {
        return i;
    }
    let xa0 = xp(a);
    let mut xa = xa0.offset(j as isize);
    let xb0 = xp(b);
    let mut xb = xb0.offset(j as isize);
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
    0
}

unsafe fn diff(a_in: *mut Bigint, b_in: *mut Bigint) -> *mut Bigint {
    let mut a = a_in;
    let mut b = b_in;
    let c: *mut Bigint;
    let mut i: c_int;
    let mut wa: c_int;
    let wb: c_int;
    let mut xa: *mut ULong;
    let xae: *mut ULong;
    let mut xb: *mut ULong;
    let xbe: *mut ULong;
    let mut xc: *mut ULong;
    let mut borrow: ULLong;
    let mut y: ULLong;

    i = cmp(a, b);
    if i == 0 {
        let c0 = Balloc(0);
        (*c0).wds = 1;
        xs(c0, 0, 0);
        return c0;
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
    xa = xp(a);
    xae = xa.offset(wa as isize);
    wb = (*b).wds;
    xb = xp(b);
    xbe = xb.offset(wb as isize);
    xc = xp(c);
    borrow = 0;

    loop {
        y = (*xa as ULLong)
            .wrapping_sub(*xb as ULLong)
            .wrapping_sub(borrow);
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
        y = (*xa as ULLong).wrapping_sub(borrow);
        xa = xa.add(1);
        borrow = (y >> 32) & 1;
        *xc = (y & 0xffffffff) as ULong;
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

unsafe fn ulp(x: *mut U) -> f64 {
    let L: ULong;
    let mut u = U::zero();

    L = (word0!(x) & 0x7ff00000).wrapping_sub((53 - 1) * 0x100000);
    word0!(&mut u) = L;
    word1!(&mut u) = 0;
    u.d
}

unsafe fn b2d(a: *mut Bigint, e: *mut c_int) -> f64 {
    let mut xa: *mut ULong;
    let xa0: *mut ULong;
    let w: ULong;
    let mut y: ULong;
    let z: ULong;
    let mut k: c_int;
    let mut d = U::zero();

    xa0 = xp(a);
    xa = xa0.offset((*a).wds as isize);
    xa = xa.offset(-1);
    y = *xa;

    k = hi0bits(y);
    *e = 32 - k;

    if k < 11 {
        word0!(&mut d) = 0x3ff00000 | (y >> (11 - k));
        w = if xa > xa0 {
            xa = xa.offset(-1);
            *xa
        } else {
            0
        };
        word1!(&mut d) = (y << ((32 - 11) + k)) | (w >> (11 - k));
        return d.d;
    }
    z = if xa > xa0 {
        xa = xa.offset(-1);
        *xa
    } else {
        0
    };
    k -= 11;
    if k != 0 {
        word0!(&mut d) = 0x3ff00000 | (y << k) | (z >> (32 - k));
        y = if xa > xa0 {
            xa = xa.offset(-1);
            *xa
        } else {
            0
        };
        word1!(&mut d) = (z << k) | (y >> (32 - k));
    } else {
        word0!(&mut d) = 0x3ff00000 | y;
        word1!(&mut d) = z;
    }
    d.d
}

unsafe fn d2b(d: *mut U, e: *mut c_int, bits: *mut c_int) -> *mut Bigint {
    let b: *mut Bigint;
    let de: c_int;
    let mut k: c_int;
    let x: *mut ULong;
    let mut y: ULong;
    let mut z: ULong;
    let i: c_int;

    b = Balloc(1);
    x = xp(b);

    z = word0!(d) & 0xfffff;
    word0!(d) &= 0x7fffffff;

    de = (word0!(d) >> 20) as c_int;
    if de != 0 {
        z |= 0x100000;
    }

    y = word1!(d);
    if y != 0 {
        k = lo0bits(&mut y);
        if k != 0 {
            *x = y | (z << (32 - k));
            z >>= k;
        } else {
            *x = y;
        }
        *x.add(1) = z;
        i = if z != 0 { 2 } else { 1 };
        (*b).wds = i;
    } else {
        k = lo0bits(&mut z);
        *x = z;
        i = 1;
        (*b).wds = 1;
        k += 32;
    }
    if de != 0 {
        *e = de - 1023 - (53 - 1) + k;
        *bits = 53 - k;
    } else {
        *e = de - 1023 - (53 - 1) + 1 + k;
        *bits = 32 * i - hi0bits(*x.offset((i - 1) as isize));
    }

    b
}

unsafe fn ratio(a: *mut Bigint, b: *mut Bigint) -> f64 {
    let mut da = U::zero();
    let mut db = U::zero();
    let mut k: c_int;
    let mut ka: c_int = 0;
    let mut kb: c_int = 0;

    da.d = b2d(a, &mut ka);
    db.d = b2d(b, &mut kb);

    k = ka - kb + 32 * ((*a).wds - (*b).wds);
    if k > 0 {
        word0!(&mut da) = word0!(&mut da).wrapping_add((k * 0x100000) as ULong);
    } else {
        k = -k;
        word0!(&mut db) = word0!(&mut db).wrapping_add((k * 0x100000) as ULong);
    }

    da.d / db.d
}

unsafe fn match_(sp: *mut *const c_char, t_in: *const c_char) -> c_int {
    let mut c: c_int;
    let mut d: c_int;
    let mut t = t_in;
    let mut s = *sp;

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

unsafe fn hexnan(rvp: *mut U, sp: *mut *const c_char) {
    let mut c: ULong;
    let mut x: [ULong; 2] = [0, 0];
    let mut s: *const c_char;
    let mut c1: c_int;
    let mut havedig: c_int;
    let mut udx0: c_int;
    let mut xshift: c_int;

    havedig = 0;
    xshift = 0;
    udx0 = 1;
    s = *sp;

    loop {
        c = *(s.add(1) as *const u8) as ULong;
        if c == 0 || c > b' ' as ULong {
            break;
        }
        s = s.add(1);
    }
    if *s.add(1) == b'0' as c_char
        && (*s.add(2) == b'x' as c_char || *s.add(2) == b'X' as c_char)
    {
        s = s.add(2);
    }
    loop {
        s = s.add(1);
        c = *(s as *const u8) as ULong;
        if c == 0 {
            break;
        }
        c1 = HEXDIG[c as usize] as c_int;
        if c1 != 0 {
            c = (c1 & 0xf) as ULong;
        } else if c <= b' ' as ULong {
            if udx0 != 0 && havedig != 0 {
                udx0 = 0;
                xshift = 1;
            }
            continue;
        } else {
            loop {
                if c == b')' as ULong {
                    *sp = s.add(1);
                    break;
                }
                s = s.add(1);
                c = *(s as *const u8) as ULong;
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
        word0!(rvp) = 0x7ff00000 | x[0];
        word1!(rvp) = x[1];
    }
}

unsafe fn increment(b_in: *mut Bigint) -> *mut Bigint {
    let mut b = b_in;
    let mut x: *mut ULong;
    let xe: *mut ULong;
    let b1: *mut Bigint;

    x = xp(b);
    xe = x.offset((*b).wds as isize);
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
    {
        if (*b).wds >= (*b).maxwds {
            b1 = Balloc((*b).k + 1);
            Bcopy(b1, b);
            Bfree(b);
            b = b1;
        }
        xs(b, (*b).wds as usize, 1);
        (*b).wds += 1;
    }
    b
}

unsafe fn rshift(b: *mut Bigint, k_in: c_int) {
    let mut k = k_in;
    let mut x: *mut ULong;
    let mut x1: *mut ULong;
    let xe: *mut ULong;
    let mut y: ULong;
    let mut n: c_int;

    x = xp(b);
    x1 = x;
    n = k >> 5;
    if n < (*b).wds {
        xe = x.offset((*b).wds as isize);
        x = x.offset(n as isize);
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
    (*b).wds = (x1 as usize - xp(b) as usize) as c_int / 4;
    if (*b).wds == 0 {
        xs(b, 0, 0);
    }
}

unsafe fn any_on(b: *mut Bigint, k_in: c_int) -> ULong {
    let mut k = k_in;
    let mut n: c_int;
    let nwds: c_int;
    let mut x: *mut ULong;
    let x0: *mut ULong;
    let mut x1: ULong;
    let x2: ULong;

    x = xp(b);
    nwds = (*b).wds;
    n = k >> 5;
    if n > nwds {
        n = nwds;
    } else if n < nwds {
        k &= 31;
        if k != 0 {
            x2 = *x.offset(n as isize);
            x1 = x2;
            x1 >>= k;
            x1 <<= k;
            if x1 != x2 {
                return 1;
            }
        }
    }
    x0 = x;
    x = x.offset(n as isize);
    while x > x0 {
        x = x.offset(-1);
        if *x != 0 {
            return 1;
        }
    }
    0
}

const Round_zero: c_int = 0;
const Round_near: c_int = 1;
const Round_up: c_int = 2;
const Round_down: c_int = 3;

#[inline]
unsafe fn gethex_ret_tiny(rvp: *mut U) {
    ffi::set_errno(ffi::ERANGE);
    word0!(rvp) = 0;
    word1!(rvp) = 1;
}

#[inline]
unsafe fn gethex_ret_big(rvp: *mut U) {
    word0!(rvp) = 0xfffff | 0x100000 * (1024 + 1023 - 1);
    word1!(rvp) = 0xffffffff;
}

#[inline]
unsafe fn gethex_ovfl1(rvp: *mut U) {
    ffi::set_errno(ffi::ERANGE);
    word0!(rvp) = 0x7ff00000;
    word1!(rvp) = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethex(
    sp: *mut *const c_char,
    rvp: *mut U,
    rounding: c_int,
    sign: c_int,
) -> () {
    let mut b: *mut Bigint = core::ptr::null_mut();
    let mut d: c_int;
    let mut decpt: *const u8;
    let mut s0: *const u8;
    let mut s: *const u8;
    let s1: *const u8;
    let mut e: c_int;
    let mut e1: c_int;
    let mut L: ULong;
    let mut lostbits: ULong;
    let mut x: *mut ULong;
    let mut big: c_int;
    let mut denorm: c_int;
    let mut esign: c_int;
    let mut havedig: c_int;
    let mut k: c_int;
    let mut n: c_int;
    let mut nb: c_int;
    let mut nbits: c_int;
    let mut nz: c_int;
    let mut up: c_int;
    let mut zret: c_int;

    const emax: c_int = 0x7fe - 1023 - 53 + 1;
    const emin: c_int = -1022 - 53 + 1;

    let mut check_denorm: c_int = 0;
    havedig = 0;
    s0 = (*sp as *const u8).add(2);
    while *s0.offset(havedig as isize) == b'0' {
        havedig += 1;
    }
    s0 = s0.offset(havedig as isize);
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
            e = -(((s as isize - decpt as isize) as c_int) << 2);
        }
    }
    /* pcheck: */
    s1 = s;
    big = 0;
    esign = 0;
    if *s == b'p' || *s == b'P' {
        'expdone: {
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
                break 'expdone;
            }
            e1 = n - 0x10;
            loop {
                s = s.add(1);
                n = HEXDIG[*s as usize] as c_int;
                if n == 0 || n > 0x19 {
                    break;
                }
                if (e1 & 0xf8000000u32 as c_int) != 0 {
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
        *sp = (s0 as *const c_char).offset(-1);
    }
    if zret != 0 {
        /* retz1 */
        (*rvp).d = 0.;
        return;
    }
    if big != 0 {
        if esign != 0 {
            let mut go_tiny = false;
            match rounding {
                // C:  case Round_up:   if (sign) break; goto ret_tiny;
                // i.e. a NON-zero sign breaks out to retz; sign == 0 rounds the
                // (positive) underflowing value UP to the smallest denormal.
                Round_up => {
                    if sign != 0 {
                        /* break */
                    } else {
                        go_tiny = true;
                    }
                }
                // C:  case Round_down: if (!sign) break; goto ret_tiny;
                // i.e. a zero sign breaks out to retz; a negative value rounds
                // DOWN (away from zero) to the smallest denormal.
                Round_down => {
                    if sign == 0 {
                        /* break */
                    } else {
                        go_tiny = true;
                    }
                }
                _ => {}
            }
            if go_tiny {
                gethex_ret_tiny(rvp);
                return;
            }
            /* retz */
            ffi::set_errno(ffi::ERANGE);
            (*rvp).d = 0.;
            return;
        }
        let mut go_big = false;
        match rounding {
            Round_near => {}
            Round_up => {
                if sign != 0 {
                    go_big = true;
                }
            }
            Round_down => {
                if sign == 0 {
                    go_big = true;
                }
            }
            _ => {
                go_big = true;
            }
        }
        if !go_big {
            gethex_ovfl1(rvp);
            return;
        }
        gethex_ret_big(rvp);
        return;
    }
    n = (s1 as isize - s0 as isize) as c_int - 1;
    k = 0;
    while n > (1 << (5 - 2)) - 1 {
        n >>= 1;
        k += 1;
    }
    b = Balloc(k);
    x = xp(b);
    havedig = 0;
    n = 0;
    nz = 0;
    let _ = nz;
    L = 0;

    let mut s1m = s1;
    while s1m > s0 {
        s1m = s1m.offset(-1);
        if *s1m == b'.' {
            continue;
        }

        d = HEXDIG[*s1m as usize] as c_int;
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
        L |= ((d & 0x0f) as ULong) << n;
        n += 4;
    }
    *x = L;
    x = x.add(1);
    n = (x as usize - xp(b) as usize) as c_int / 4;
    (*b).wds = n;
    nb = 32 * n - hi0bits(L);
    nbits = 53;
    lostbits = 0;
    x = xp(b);
    if nb > nbits {
        n = nb - nbits;
        if any_on(b, n) != 0 {
            lostbits = 1;
            k = n - 1;
            if (*x.offset((k >> 5) as isize) & (1 << (k & 31))) != 0 {
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
        x = xp(b);
    }
    if e > emax {
        /* ovfl */
        Bfree(b);
        gethex_ovfl1(rvp);
        return;
    }
    denorm = 0;
    let mut goto_normal = false;
    if e < emin {
        denorm = 1;
        n = emin - e;
        if n >= nbits {
            let mut go_tinyf = false;
            match rounding {
                Round_near => {
                    if n == nbits && (n < 2 || lostbits != 0 || any_on(b, n - 1) != 0) {
                        go_tinyf = true;
                    }
                }
                Round_up => {
                    if sign == 0 {
                        go_tinyf = true;
                    }
                }
                Round_down => {
                    if sign != 0 {
                        go_tinyf = true;
                    }
                }
                _ => {}
            }
            if go_tinyf {
                Bfree(b);
                gethex_ret_tiny(rvp);
                return;
            }

            Bfree(b);
            /* retz */
            ffi::set_errno(ffi::ERANGE);
            (*rvp).d = 0.;
            return;
        }
        k = n - 1;

        if k == 0 {
            let mut emin_check = false;
            match rounding {
                Round_near => {
                    if (xg(b, 0) & 3) == 3 || (lostbits != 0 && (xg(b, 0) & 1) != 0) {
                        multadd(b, 1, 1);
                        emin_check = true;
                    }
                }
                Round_up => {
                    if sign == 0 && (lostbits != 0 || (xg(b, 0) & 1) != 0) {
                        /* incr_denorm */
                        multadd(b, 1, 2);
                        check_denorm = 1;
                        lostbits = 0;
                        emin_check = true;
                    }
                }
                Round_down => {
                    if sign != 0 && (lostbits != 0 || (xg(b, 0) & 1) != 0) {
                        /* incr_denorm */
                        multadd(b, 1, 2);
                        check_denorm = 1;
                        lostbits = 0;
                        emin_check = true;
                    }
                }
                _ => {}
            }
            if emin_check && xg(b, 1) == (1 << (20 + 1)) {
                rshift(b, 1);
                e = emin;
                goto_normal = true;
            }
        }

        if !goto_normal {
            let mut no_lostbits = false;
            if lostbits != 0 {
                lostbits = 1;
            } else if k > 0 {
                lostbits = any_on(b, k);
            } else if check_denorm != 0 {
                no_lostbits = true;
            }

            if !no_lostbits && (*x.offset((k >> 5) as isize) & (1 << (k & 31))) != 0 {
                lostbits |= 2;
            }

            /* no_lostbits: */
            nbits -= n;
            rshift(b, n);
            e = emin;
        }
    }
    if !goto_normal {
        if lostbits != 0 {
            up = 0;
            match rounding {
                Round_zero => {}
                Round_near => {
                    if (lostbits & 2) != 0 && ((lostbits & 1) | (*x & 1)) != 0 {
                        up = 1;
                    }
                }
                Round_up => {
                    up = 1 - sign;
                }
                Round_down => {
                    up = sign;
                }
                _ => {}
            }
            if up != 0 {
                k = (*b).wds;
                b = increment(b);
                x = xp(b);
                n = nbits & 31;
                if denorm == 0
                    && ((*b).wds > k
                        || (n != 0 && hi0bits(*x.offset((k - 1) as isize)) < 32 - n))
                {
                    rshift(b, 1);
                    e += 1;
                    if e > 1023 {
                        /* ovfl */
                        Bfree(b);
                        gethex_ovfl1(rvp);
                        return;
                    }
                }
            }
        }
    }

    if !goto_normal && denorm != 0 {
        word0!(rvp) = if (*b).wds > 1 {
            xg(b, 1) & !0x100000
        } else {
            0
        };
    } else {
        /* normal: */
        word0!(rvp) = (xg(b, 1) & !0x100000) | (((e + 0x3ff + 52) as ULong) << 20);
    }
    word1!(rvp) = xg(b, 0);
    Bfree(b);
}

unsafe fn dshift(b: *mut Bigint, p2: c_int) -> c_int {
    let mut rv = hi0bits(xg(b, ((*b).wds - 1) as usize)) - 4;
    if p2 > 0 {
        rv -= p2;
    }
    rv & 31
}

unsafe fn quorem(b: *mut Bigint, S: *mut Bigint) -> c_int {
    let mut n: c_int;
    let mut bx: *mut ULong;
    let mut bxe: *mut ULong;
    let mut q: ULong;
    let mut sx: *mut ULong;
    let sxe: *mut ULong;
    let mut borrow: ULLong;
    let mut carry: ULLong;
    let mut y: ULLong;
    let mut ys: ULLong;

    n = (*S).wds;

    if (*b).wds < n {
        return 0;
    }
    sx = xp(S);
    n -= 1;
    sxe = sx.offset(n as isize);
    bx = xp(b);
    bxe = bx.offset(n as isize);
    q = *bxe / (*sxe + 1);
    if q != 0 {
        borrow = 0;
        carry = 0;
        loop {
            ys = (*sx as ULLong) * (q as ULLong) + carry;
            sx = sx.add(1);
            carry = ys >> 32;
            y = (*bx as ULLong)
                .wrapping_sub(ys & 0xffffffff)
                .wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            *bx = (y & 0xffffffff) as ULong;
            bx = bx.add(1);
            if sx > sxe {
                break;
            }
        }
        if *bxe == 0 {
            bx = xp(b);
            loop {
                bxe = bxe.offset(-1);
                if !(bxe > bx && *bxe == 0) {
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
        bx = xp(b);
        sx = xp(S);
        loop {
            ys = (*sx as ULLong) + carry;
            sx = sx.add(1);
            carry = ys >> 32;
            y = (*bx as ULLong)
                .wrapping_sub(ys & 0xffffffff)
                .wrapping_sub(borrow);
            borrow = (y >> 32) & 1;
            *bx = (y & 0xffffffff) as ULong;
            bx = bx.add(1);
            if sx > sxe {
                break;
            }
        }
        bx = xp(b);
        bxe = bx.offset(n as isize);
        if *bxe == 0 {
            loop {
                bxe = bxe.offset(-1);
                if !(bxe > bx && *bxe == 0) {
                    break;
                }
                n -= 1;
            }
            (*b).wds = n;
        }
    }
    q as c_int
}

unsafe fn sulp(x: *mut U, bc: *mut BCinfo) -> f64 {
    let mut u = U::zero();
    let rv: f64;
    let i: c_int;

    rv = ulp(x);
    i = 2 * 53 + 1 - ((word0!(x) & 0x7ff00000) >> 20) as c_int;
    if (*bc).scale == 0 || i <= 0 {
        return rv;
    }
    word0!(&mut u) = (0x3ff00000 + (i << 20)) as ULong;
    word1!(&mut u) = 0;
    rv * u.d
}

unsafe fn bigcomp(rv: *mut U, s0: *const c_char, bc: *mut BCinfo) {
    let mut b: *mut Bigint;
    let mut d: *mut Bigint;
    let mut b2: c_int;
    let mut bbits: c_int = 0;
    let mut d2: c_int;
    let mut dd: c_int = 0;
    let mut dig: c_int;
    let mut dsign: c_int;
    let mut i: c_int;
    let mut j: c_int;
    let nd: c_int;
    let mut nd0: c_int;
    let mut p2: c_int = 0;
    let p5: c_int;
    let mut speccase: c_int;

    dsign = (*bc).dsign;
    nd = (*bc).nd;
    nd0 = (*bc).nd0;
    p5 = nd + (*bc).e0 - 1;
    speccase = 0;

    let mut have_i = false;
    if (*rv).d == 0. {
        b = i2b(1);
        p2 = -1022 - 53 + 1;
        bbits = 1;

        word0!(rv) = (53 + 2) << 20;

        i = 0;

        speccase = 1;
        p2 -= 1;
        dsign = 0;
        have_i = true;
    } else {
        b = d2b(rv, &mut p2, &mut bbits);
        i = 0;
    }

    if !have_i {
        p2 -= (*bc).scale;

        i = 53 - bbits;
        j = 53 - (-1022) - 1 + p2;
        if i > j {
            i = j;
        }
        i += 1;
        b = lshift(b, i);
        xs(b, 0, xg(b, 0) | 1);
    }

    /* have_i: */
    p2 -= p5 + i;
    d = i2b(1);

    if p5 > 0 {
        d = pow5mult(d, p5);
    } else if p5 < 0 {
        b = pow5mult(b, -p5);
    }
    if p2 > 0 {
        b2 = p2;
        d2 = 0;
    } else {
        b2 = 0;
        d2 = -p2;
    }
    i = dshift(d, d2);
    b2 += i;
    if b2 > 0 {
        b = lshift(b, b2);
    }
    d2 += i;
    if d2 > 0 {
        d = lshift(d, d2);
    }

    dig = quorem(b, d);
    if dig == 0 {
        b = multadd(b, 10, 0);
        dig = quorem(b, d);
    }

    'ret: {
        i = 0;
        while i < nd0 {
            dd = *s0.offset(i as isize) as c_int - '0' as c_int - dig;
            i += 1;
            if dd != 0 {
                break 'ret;
            }
            if xg(b, 0) == 0 && (*b).wds == 1 {
                if i < nd {
                    dd = 1;
                }
                break 'ret;
            }
            b = multadd(b, 10, 0);
            dig = quorem(b, d);
        }
        j = (*bc).dp1;
        while {
            let old = i;
            i += 1;
            old < nd
        } {
            dd = *s0.offset(j as isize) as c_int - '0' as c_int - dig;
            j += 1;
            if dd != 0 {
                break 'ret;
            }
            if xg(b, 0) == 0 && (*b).wds == 1 {
                if i < nd {
                    dd = 1;
                }
                break 'ret;
            }
            b = multadd(b, 10, 0);
            dig = quorem(b, d);
        }
        if dig > 0 || xg(b, 0) != 0 || (*b).wds > 1 {
            dd = -1;
        }
    }
    /* ret: */
    Bfree(b);
    Bfree(d);
    if speccase != 0 {
        if dd <= 0 {
            (*rv).d = 0.;
        }
    } else if dd < 0 {
        if dsign == 0 {
            /* retlow1 */
            (*rv).d -= sulp(rv, bc);
        }
    } else if dd > 0 {
        if dsign != 0 {
            /* rethi1 */
            (*rv).d += sulp(rv, bc);
        }
    } else {
        let mut odd = false;
        j = (((word0!(rv) & 0x7ff00000) >> 20) as c_int) - (*bc).scale;
        if j <= 0 {
            i = 1 - j;
            if i <= 31 {
                if (word1!(rv) & (0x1u32 << i)) != 0 {
                    odd = true;
                }
            } else if (word0!(rv) & (0x1u32 << (i - 32))) != 0 {
                odd = true;
            }
        } else if (word1!(rv) & 1) != 0 {
            odd = true;
        }
        if odd {
            if dsign != 0 {
                (*rv).d += sulp(rv, bc);
            } else {
                (*rv).d -= sulp(rv, bc);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod__unused(s00_in: *const c_char, se: *mut *mut c_char) -> f64 {
    let mut bb2: c_int;
    let mut bb5: c_int;
    let mut bbe: c_int = 0;
    let mut bd2: c_int;
    let mut bd5: c_int;
    let mut bbbits: c_int = 0;
    let mut bs2: c_int;
    let mut c: c_int;
    let mut e: c_int;
    let mut e1: c_int;
    let mut esign: c_int;
    let mut i: c_int;
    let mut j: c_int;
    let mut k: c_int;
    let mut nd: c_int;
    let mut nd0: c_int;
    let mut nf: c_int;
    let mut nz: c_int;
    let mut nz0: c_int;
    let mut nz1: c_int;
    let mut sign: c_int;
    let mut s: *const c_char;
    let mut s0: *const c_char;
    let mut s1: *const c_char;
    let mut aadj: f64;
    let mut aadj1: f64 = 0.;
    let mut L: c_int;
    let mut aadj2 = U::zero();
    let mut adj = U::zero();
    let mut rv = U::zero();
    let mut rv0 = U::zero();
    let mut y: ULong;
    let mut z: ULong;
    let mut bc = BCinfo {
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
    };
    let mut bb: *mut Bigint = core::ptr::null_mut();
    let mut bb1: *mut Bigint;
    let mut bd: *mut Bigint = core::ptr::null_mut();
    let mut bd0: *mut Bigint = core::ptr::null_mut();
    let mut bs: *mut Bigint = core::ptr::null_mut();
    let mut delta: *mut Bigint = core::ptr::null_mut();

    let mut bhi: ULLong;
    let mut blo: ULLong;
    let mut brv: ULLong;
    let mut t00: ULLong;
    let t01: ULLong;
    let t02: ULLong;
    let mut t10: ULLong;
    let t11: ULLong;
    let mut terv: ULLong;
    let mut tg: ULLong;
    let mut tlo: ULLong;
    let mut yz: ULLong;
    let mut p10: *const BF96;
    let mut bexact: c_int;
    let mut erv: c_int;

    let mut Lsb: ULong;
    let mut Lsb1: ULong;

    let mut req_bigcomp: c_int = 0;
    let mut s00 = s00_in;

    sign = 0;
    nz0 = 0;
    nz1 = 0;
    nz = 0;
    bc.dplen = 0;
    bc.uflchk = 0;
    rv.d = 0.;

    'ret: {
        'range_err: {
            /* ---- leading sign / whitespace ---- */
            s = s00;
            let mut ret0 = false;
            loop {
                let ch = *s;
                if ch == b'-' as c_char {
                    sign = 1;
                    s = s.add(1);
                    if *s != 0 {
                        break;
                    }
                    ret0 = true;
                    break;
                } else if ch == b'+' as c_char {
                    s = s.add(1);
                    if *s != 0 {
                        break;
                    }
                    ret0 = true;
                    break;
                } else if ch == 0 {
                    ret0 = true;
                    break;
                } else if ch == b'\t' as c_char
                    || ch == b'\n' as c_char
                    || ch == 0x0b
                    || ch == 0x0c
                    || ch == b'\r' as c_char
                    || ch == b' ' as c_char
                {
                    s = s.add(1);
                    continue;
                } else {
                    break;
                }
            }
            if ret0 {
                s = s00;
                sign = 0;
                break 'ret;
            }

            /* break2: */
            if *s == b'0' as c_char {
                if *s.add(1) == b'x' as c_char || *s.add(1) == b'X' as c_char {
                    let mut sp: *const c_char = s;
                    gethex(&mut sp, &mut rv, 1, sign);
                    s = sp;
                    break 'ret;
                }

                nz0 = 1;
                loop {
                    s = s.add(1);
                    if *s != b'0' as c_char {
                        break;
                    }
                }
                if *s == 0 {
                    break 'ret;
                }
            }
            s0 = s;
            nd = 0;
            nf = 0;

            yz = 0;
            loop {
                c = *s as c_int;
                if !(c >= '0' as c_int && c <= '9' as c_int) {
                    break;
                }
                if nd < 19 {
                    yz = 10 * yz + (c - '0' as c_int) as ULLong;
                }
                nd += 1;
                s = s.add(1);
            }
            nd0 = nd;
            bc.dp0 = (s as isize - s0 as isize) as c_int;
            bc.dp1 = bc.dp0;
            s1 = s;
            while s1 > s0 {
                s1 = s1.offset(-1);
                if *s1 != b'0' as c_char {
                    break;
                }
                nz1 += 1;
            }
            let mut dig_done = false;
            if c == '.' as c_int {
                s = s.add(1);
                c = *s as c_int;
                bc.dp1 = (s as isize - s0 as isize) as c_int;
                bc.dplen = bc.dp1 - bc.dp0;
                let mut have_dig = false;
                if nd == 0 {
                    while c == '0' as c_int {
                        nz += 1;
                        s = s.add(1);
                        c = *s as c_int;
                    }
                    if c > '0' as c_int && c <= '9' as c_int {
                        bc.dp0 = (s0 as isize - s as isize) as c_int;
                        bc.dp1 = bc.dp0 + bc.dplen;
                        s0 = s;
                        nf += nz;
                        nz = 0;
                        have_dig = true;
                    } else {
                        dig_done = true;
                    }
                }
                if !dig_done {
                    loop {
                        if !have_dig && !(c >= '0' as c_int && c <= '9' as c_int) {
                            break;
                        }
                        have_dig = false;
                        /* have_dig: */
                        nz += 1;
                        c -= '0' as c_int;
                        if c != 0 {
                            nf += nz;
                            i = 1;

                            while i < nz {
                                nd += 1;
                                if nd <= 19 {
                                    yz *= 10;
                                }
                                i += 1;
                            }
                            nd += 1;
                            if nd <= 19 {
                                yz = 10 * yz + c as ULLong;
                            }
                            nz = 0;
                            nz1 = 0;
                        }
                        s = s.add(1);
                        c = *s as c_int;
                    }
                }
            }
            /* dig_done: */
            e = 0;
            if c == 'e' as c_int || c == 'E' as c_int {
                if nd == 0 && nz == 0 && nz0 == 0 {
                    s = s00;
                    sign = 0;
                    break 'ret;
                }
                s00 = s;
                esign = 0;
                s = s.add(1);
                c = *s as c_int;
                if c == '-' as c_int {
                    esign = 1;
                    s = s.add(1);
                    c = *s as c_int;
                } else if c == '+' as c_int {
                    s = s.add(1);
                    c = *s as c_int;
                }
                if c >= '0' as c_int && c <= '9' as c_int {
                    while c == '0' as c_int {
                        s = s.add(1);
                        c = *s as c_int;
                    }
                    if c > '0' as c_int && c <= '9' as c_int {
                        L = c - '0' as c_int;
                        loop {
                            s = s.add(1);
                            c = *s as c_int;
                            if !(c >= '0' as c_int && c <= '9' as c_int) {
                                break;
                            }
                            if L <= 19999 {
                                L = 10 * L + c - '0' as c_int;
                            }
                        }
                        if L > 19999 {
                            e = 19999;
                        } else {
                            e = L;
                        }
                        if esign != 0 {
                            e = -e;
                        }
                    } else {
                        e = 0;
                    }
                } else {
                    s = s00;
                }
            }
            if nd == 0 {
                if nz == 0 && nz0 == 0 {
                    let mut done = false;
                    if bc.dplen == 0 {
                        if c == 'i' as c_int || c == 'I' as c_int {
                            let mut sp: *const c_char = s;
                            if match_(&mut sp, b"nf\0".as_ptr() as *const c_char) != 0 {
                                s = sp.offset(-1);
                                let mut sp2: *const c_char = s;
                                if match_(&mut sp2, b"inity\0".as_ptr() as *const c_char) == 0 {
                                    s = sp2.add(1);
                                } else {
                                    s = sp2;
                                }
                                word0!(&mut rv) = 0x7ff00000;
                                word1!(&mut rv) = 0;
                                done = true;
                            } else {
                                s = sp;
                            }
                        } else if c == 'n' as c_int || c == 'N' as c_int {
                            let mut sp: *const c_char = s;
                            if match_(&mut sp, b"an\0".as_ptr() as *const c_char) != 0 {
                                s = sp;
                                word0!(&mut rv) = 0x7ff80000;
                                word1!(&mut rv) = 0;

                                if *s == b'(' as c_char {
                                    let mut sp3: *const c_char = s;
                                    hexnan(&mut rv, &mut sp3);
                                    s = sp3;
                                }

                                done = true;
                            } else {
                                s = sp;
                            }
                        }
                    }
                    if done {
                        break 'ret;
                    }

                    /* ret0: */
                    s = s00;
                    sign = 0;
                }
                break 'ret;
            }
            e -= nf;
            e1 = e;
            bc.e0 = e;

            if nd0 == 0 {
                nd0 = nd;
            }
            bd0 = core::ptr::null_mut();
            if nd <= 15 {
                rv.d = yz as f64;

                if e == 0 {
                    break 'ret;
                }

                if e > 0 {
                    if e <= 22 {
                        rv.d *= TENS[e as usize];
                        break 'ret;
                    }
                    i = 15 - nd;
                    if e <= 22 + i {
                        e -= i;
                        rv.d *= TENS[i as usize];
                        rv.d *= TENS[e as usize];

                        break 'ret;
                    }
                } else if e >= -22 {
                    rv.d /= TENS[(-e) as usize];
                    break 'ret;
                }
            }

            k = if nd < 19 { nd } else { 19 };

            e1 += nd - k;
            i = e1 + 342;

            let mut goto_ovfl = false;
            let mut goto_undfl = false;
            let mut many_digits = false;
            let mut ret_now = false;

            'fast: {
                if i < 0 {
                    goto_undfl = true;
                    break 'fast;
                }
                if i > 650 {
                    goto_ovfl = true;
                    break 'fast;
                }
                p10 = pten(i);
                brv = yz;

                i = 0;
                if (brv & 0xffffffff00000000u64) == 0 {
                    i = 32;
                    brv <<= 32;
                }
                if (brv & 0xffff000000000000u64) == 0 {
                    i += 16;
                    brv <<= 16;
                }
                if (brv & 0xff00000000000000u64) == 0 {
                    i += 8;
                    brv <<= 8;
                }
                if (brv & 0xf000000000000000u64) == 0 {
                    i += 4;
                    brv <<= 4;
                }
                if (brv & 0xc000000000000000u64) == 0 {
                    i += 2;
                    brv <<= 2;
                }
                if (brv & 0x8000000000000000u64) == 0 {
                    i += 1;
                    brv <<= 1;
                }
                erv = (64 + 0x3fe) + (*p10).e - i;
                if erv <= 0 && nd > 19 {
                    many_digits = true;
                    break 'fast;
                }
                bhi = brv >> 32;
                blo = brv & 0xffffffff;

                t01 = bhi * (*p10).b1 as ULLong;
                t10 = blo * (*p10).b0 as ULLong + (t01 & 0xffffffff);
                t00 = bhi * (*p10).b0 as ULLong + (t01 >> 32) + (t10 >> 32);

                /* Common tail helpers for the two parity branches. */
                let mut do_roundup = false;
                let mut do_noround = false;
                let mut do_roundup1 = false;
                let mut do_noround1 = false;
                let mut do_denormal = false;
                let mut do_denormal1 = false;
                let mut do_tiniest = false;
                let mut do_smallest_normal = false;

                if (t00 & 0x8000000000000000u64) != 0 {
                    if (t00 & 0x3ff) != 0 && (!t00 & 0x3fe) != 0 {
                        if nd > 19
                            && ((((t00 + (1u64 << i) + 2) & 0x400) ^ (t00 & 0x400)) != 0)
                        {
                            many_digits = true;
                            break 'fast;
                        }
                        if erv <= 0 {
                            do_denormal = true;
                        } else if (t00 & 0x400) != 0 && (t00 & 0xbff) != 0 {
                            do_roundup = true;
                        } else {
                            do_noround = true;
                        }
                    }
                } else if (t00 & 0x1ff) != 0 && (!t00 & 0x1fe) != 0 {
                    if nd > 19 && ((((t00 + (1u64 << i) + 2) & 0x200) ^ (t00 & 0x200)) != 0) {
                        many_digits = true;
                        break 'fast;
                    }
                    if erv <= 1 {
                        do_denormal1 = true;
                    } else if (t00 & 0x200) != 0 {
                        do_roundup1 = true;
                    } else {
                        do_noround1 = true;
                    }
                }

                if !(do_roundup
                    || do_noround
                    || do_roundup1
                    || do_noround1
                    || do_denormal
                    || do_denormal1)
                {
                    t02 = bhi * (*p10).b2 as ULLong;
                    t11 = blo * (*p10).b1 as ULLong + (t02 & 0xffffffff);
                    bexact = 1;
                    if e1 < 0 || e1 > 41 || ((t10 | t11) & 0xffffffff) != 0 || nd > 19 {
                        bexact = 0;
                    }
                    tlo = (t10 & 0xffffffff) + (t02 >> 32) + (t11 >> 32);
                    if bexact == 0 && (tlo + 0x10) >> 32 > tlo >> 32 {
                        many_digits = true;
                        break 'fast;
                    }
                    t00 += tlo >> 32;
                    if (t00 & 0x8000000000000000u64) != 0 {
                        if erv <= 0 {
                            if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x3ff)) == 0 {
                                many_digits = true;
                                break 'fast;
                            }
                            do_denormal = true;
                        } else if bexact != 0 {
                            if (t00 & 0x400) != 0
                                && (((tlo & 0xffffffff) | (t00 & 0xbff)) != 0)
                            {
                                do_roundup = true;
                            } else {
                                do_noround = true;
                            }
                        } else if ((tlo & 0xfffffff0) | (t00 & 0x3ff)) != 0
                            && (nd <= 19
                                || ((t00 + (1u64 << i)) & 0xfffffffffffffc00u64)
                                    == (t00 & 0xfffffffffffffc00u64))
                        {
                            if (t00 & 0x400) != 0 {
                                do_roundup = true;
                            } else {
                                do_noround = true;
                            }
                        }
                    } else if erv <= 1 {
                        if nd >= 20 || ((tlo & 0xfffffff0) | (t00 & 0x1ff)) == 0 {
                            many_digits = true;
                            break 'fast;
                        }
                        do_denormal1 = true;
                    } else if bexact != 0 {
                        if (t00 & 0x200) != 0 && ((t00 & 0x5ff) != 0 || tlo != 0) {
                            do_roundup1 = true;
                        } else {
                            do_noround1 = true;
                        }
                    } else if ((tlo & 0xfffffff0) | (t00 & 0x1ff)) != 0
                        && (nd <= 19
                            || ((t00 + (1u64 << i)) & 0x7ffffffffffffe00u64)
                                == (t00 & 0x7ffffffffffffe00u64))
                    {
                        if (t00 & 0x200) != 0 {
                            do_roundup1 = true;
                        } else {
                            do_noround1 = true;
                        }
                    }
                }

                /* ---- denormal / rounding tails ---- */
                if do_denormal {
                    if erv <= -52 {
                        if erv < -52 || (t00 & 0x7fffffffffffffffu64) == 0 {
                            goto_undfl = true;
                            break 'fast;
                        }
                        do_tiniest = true;
                    } else {
                        tg = 1u64 << (11 - erv);
                        t00 &= !(tg - 1);

                        if (t00 & tg) != 0 {
                            t00 = t00.wrapping_add(tg << 1);
                            if (t00 & 0x8000000000000000u64) == 0 {
                                erv += 1;
                                if erv > 0 {
                                    do_smallest_normal = true;
                                } else {
                                    t00 = 0x8000000000000000u64;
                                }
                            }
                        }

                        if !do_smallest_normal {
                            rv.LL = t00 >> (12 - erv);
                            ffi::set_errno(ffi::ERANGE);
                            break 'ret;
                        }
                    }
                }

                if do_denormal1 {
                    if erv <= -51 {
                        if erv < -51 || (t00 & 0x3fffffffffffffffu64) == 0 {
                            goto_undfl = true;
                            break 'fast;
                        }
                        do_tiniest = true;
                    } else {
                        tg = 1u64 << (11 - erv);

                        if (t00 & tg) != 0 {
                            t00 = t00.wrapping_add(tg << 1);
                            if (0x8000000000000000u64 & t00) != 0 && erv == 1 {
                                do_smallest_normal = true;
                            }
                        }

                        if !do_smallest_normal {
                            if erv <= -52 {
                                goto_undfl = true;
                                break 'fast;
                            }
                            rv.LL = t00 >> (12 - erv);
                            ffi::set_errno(ffi::ERANGE);
                            break 'ret;
                        }
                    }
                }

                if do_tiniest {
                    rv.LL = 1;
                    ffi::set_errno(ffi::ERANGE);
                    break 'ret;
                }

                if do_smallest_normal {
                    rv.LL = 0x0010000000000000u64;
                    break 'ret;
                }

                if do_roundup {
                    t00 = t00.wrapping_add(0x800);
                    if (t00 & 0x8000000000000000u64) == 0 {
                        if erv >= 0x7fe {
                            goto_ovfl = true;
                            break 'fast;
                        }
                        terv = (erv + 1) as ULLong;
                        rv.LL = terv << 52;
                        break 'ret;
                    }
                    do_noround = true;
                }
                if do_noround {
                    if erv >= 0x7ff {
                        goto_ovfl = true;
                        break 'fast;
                    }
                    terv = erv as ULLong;
                    rv.LL = (terv << 52) | ((t00 & 0x7ffffffffffff800u64) >> 11);
                    break 'ret;
                }

                if do_roundup1 {
                    t00 = t00.wrapping_add(0x400);
                    if (t00 & 0x4000000000000000u64) == 0 {
                        if erv >= 0x7ff {
                            goto_ovfl = true;
                            break 'fast;
                        }
                        terv = erv as ULLong;
                        rv.LL = terv << 52;
                        break 'ret;
                    }
                    do_noround1 = true;
                }
                if do_noround1 {
                    if erv >= 0x800 {
                        goto_ovfl = true;
                        break 'fast;
                    }
                    terv = (erv - 1) as ULLong;
                    rv.LL = (terv << 52) | ((t00 & 0x3ffffffffffffc00u64) >> 10);
                    break 'ret;
                }

                many_digits = true;
            }

            if ret_now {
                break 'ret;
            }
            if goto_ovfl {
                word0!(&mut rv) = 0x7ff00000;
                word1!(&mut rv) = 0;
                break 'range_err;
            }
            if goto_undfl {
                rv.d = 0.;
                break 'range_err;
            }
            let _ = many_digits;

            /* many_digits: */
            if nd > 17 {
                if nd > 18 {
                    yz /= 100;
                    e1 += 2;
                } else {
                    yz /= 10;
                    e1 += 1;
                }
                y = (yz / 100000000) as ULong;
            } else if nd > 9 {
                i = nd - 9;
                y = ((yz >> i) / pfive(i - 1)) as ULong;
            } else {
                y = yz as ULong;
            }
            rv.d = yz as f64;

            bc.scale = 0;

            let mut goto_ovfl = false;
            let mut goto_undfl = false;

            'scaling: {
                if e1 > 0 {
                    i = e1 & 15;
                    if i != 0 {
                        rv.d *= TENS[i as usize];
                    }
                    e1 &= !15;
                    if e1 != 0 {
                        if e1 > 308 {
                            goto_ovfl = true;
                            break 'scaling;
                        }
                        e1 >>= 4;
                        j = 0;
                        while e1 > 1 {
                            if (e1 & 1) != 0 {
                                rv.d *= BIGTENS[j as usize];
                            }
                            j += 1;
                            e1 >>= 1;
                        }

                        word0!(&mut rv) = word0!(&mut rv).wrapping_sub(53 * 0x100000);
                        rv.d *= BIGTENS[j as usize];
                        z = word0!(&mut rv) & 0x7ff00000;
                        if z > 0x100000 * (1024 + 1023 - 53) {
                            goto_ovfl = true;
                            break 'scaling;
                        }
                        if z > 0x100000 * (1024 + 1023 - 1 - 53) {
                            word0!(&mut rv) = 0xfffff | 0x100000 * (1024 + 1023 - 1);
                            word1!(&mut rv) = 0xffffffff;
                        } else {
                            word0!(&mut rv) = word0!(&mut rv).wrapping_add(53 * 0x100000);
                        }
                    }
                } else if e1 < 0 {
                    e1 = -e1;
                    i = e1 & 15;
                    if i != 0 {
                        rv.d /= TENS[i as usize];
                    }
                    e1 >>= 4;
                    if e1 != 0 {
                        if e1 >= 1 << 5 {
                            goto_undfl = true;
                            break 'scaling;
                        }

                        if (e1 & 0x10) != 0 {
                            bc.scale = 2 * 53;
                        }
                        j = 0;
                        while e1 > 0 {
                            if (e1 & 1) != 0 {
                                rv.d *= TINYTENS[j as usize];
                            }
                            j += 1;
                            e1 >>= 1;
                        }
                        if bc.scale != 0 {
                            j = 2 * 53 + 1 - (((word0!(&mut rv) & 0x7ff00000) >> 20) as c_int);
                            if j > 0 {
                                if j >= 32 {
                                    if j > 54 {
                                        goto_undfl = true;
                                        break 'scaling;
                                    }
                                    word1!(&mut rv) = 0;
                                    if j >= 53 {
                                        word0!(&mut rv) = (53 + 2) * 0x100000;
                                    } else {
                                        word0!(&mut rv) &= 0xffffffffu32 << (j - 32);
                                    }
                                } else {
                                    word1!(&mut rv) &= 0xffffffffu32 << j;
                                }
                            }
                        }
                        if rv.d == 0. {
                            goto_undfl = true;
                            break 'scaling;
                        }
                    }
                }
            }

            if goto_ovfl {
                word0!(&mut rv) = 0x7ff00000;
                word1!(&mut rv) = 0;
                break 'range_err;
            }
            if goto_undfl {
                rv.d = 0.;
                break 'range_err;
            }

            bc.nd = nd - nz1;
            bc.nd0 = nd0;

            if nd > 40 {
                i = 18;
                j = 18;
                if i > nd0 {
                    j += bc.dplen;
                }
                loop {
                    j -= 1;
                    if j < bc.dp1 && j >= bc.dp0 {
                        j = bc.dp0 - 1;
                    }
                    if *s0.offset(j as isize) != b'0' as c_char {
                        break;
                    }
                    i -= 1;
                }
                e += nd - i;
                nd = i;
                if nd0 > nd {
                    nd0 = nd;
                }
                if nd < 9 {
                    y = 0;
                    i = 0;
                    while i < nd0 {
                        y = 10 * y + (*s0.offset(i as isize) as ULong) - b'0' as ULong;
                        i += 1;
                    }
                    j = bc.dp1;
                    while i < nd {
                        y = 10 * y + (*s0.offset(j as isize) as ULong) - b'0' as ULong;
                        j += 1;
                        i += 1;
                    }
                }
            }

            bd0 = s2b(s0, nd0, nd, y, bc.dplen);

            let mut goto_ovfl = false;
            let mut goto_undfl = false;

            'corr: loop {
                'cont: {
                    bd = Balloc((*bd0).k);
                    Bcopy(bd, bd0);
                    bb = d2b(&mut rv, &mut bbe, &mut bbbits);
                    bs = i2b(1);

                    if e >= 0 {
                        bb2 = 0;
                        bb5 = 0;
                        bd2 = e;
                        bd5 = e;
                    } else {
                        bb2 = -e;
                        bb5 = -e;
                        bd2 = 0;
                        bd5 = 0;
                    }
                    if bbe >= 0 {
                        bb2 += bbe;
                    } else {
                        bd2 -= bbe;
                    }
                    bs2 = bb2;

                    Lsb = 1;
                    Lsb1 = 0;
                    j = bbe - bc.scale;
                    i = j + bbbits - 1;
                    j = 53 + 1 - bbbits;
                    if i < -1022 {
                        i = -1022 - i;
                        j -= i;
                        if i < 32 {
                            Lsb <<= i;
                        } else if i < 52 {
                            Lsb1 = Lsb << (i - 32);
                        } else {
                            Lsb1 = 0x7ff00000;
                        }
                    }
                    bb2 += j;
                    bd2 += j;

                    bd2 += bc.scale;

                    i = if bb2 < bd2 { bb2 } else { bd2 };
                    if i > bs2 {
                        i = bs2;
                    }
                    if i > 0 {
                        bb2 -= i;
                        bd2 -= i;
                        bs2 -= i;
                    }
                    if bb5 > 0 {
                        bs = pow5mult(bs, bb5);
                        bb1 = mult(bs, bb);
                        Bfree(bb);
                        bb = bb1;
                    }
                    if bb2 > 0 {
                        bb = lshift(bb, bb2);
                    }
                    if bd5 > 0 {
                        bd = pow5mult(bd, bd5);
                    }
                    if bd2 > 0 {
                        bd = lshift(bd, bd2);
                    }
                    if bs2 > 0 {
                        bs = lshift(bs, bs2);
                    }
                    delta = diff(bb, bd);
                    bc.dsign = (*delta).sign;
                    (*delta).sign = 0;
                    i = cmp(delta, bs);
                    if bc.nd > nd && i <= 0 {
                        if bc.dsign != 0 {
                            req_bigcomp = 1;
                            break 'corr;
                        }
                        i = -1;
                    }
                    if i < 0 {
                        if bc.dsign != 0
                            || word1!(&mut rv) != 0
                            || (word0!(&mut rv) & 0xfffff) != 0
                            || (word0!(&mut rv) & 0x7ff00000) <= (2 * 53 + 1) * 0x100000
                        {
                            break 'corr;
                        }
                        if xg(delta, 0) == 0 && (*delta).wds <= 1 {
                            break 'corr;
                        }
                        delta = lshift(delta, 1);
                        if cmp(delta, bs) > 0 {
                            /* goto drop_down */
                            if bc.scale != 0 {
                                L = (word0!(&mut rv) & 0x7ff00000) as c_int;
                                if L <= (2 * 53 + 1) * 0x100000 {
                                    if L > (53 + 2) * 0x100000 {
                                        break 'corr;
                                    }
                                    if bc.nd > nd {
                                        bc.uflchk = 1;
                                        break 'corr;
                                    }
                                    goto_undfl = true;
                                    break 'corr;
                                }
                            }

                            L = ((word0!(&mut rv) & 0x7ff00000) - 0x100000) as c_int;

                            word0!(&mut rv) = (L as ULong) | 0xfffff;
                            word1!(&mut rv) = 0xffffffff;

                            if bc.nd > nd {
                                break 'cont;
                            }

                            break 'corr;
                        }
                        break 'corr;
                    }
                    if i == 0 {
                        let mut drop_down = false;
                        if bc.dsign != 0 {
                            if (word0!(&mut rv) & 0xfffff) == 0xfffff
                                && word1!(&mut rv)
                                    == (if bc.scale != 0
                                        && {
                                            y = word0!(&mut rv) & 0x7ff00000;
                                            y <= 2 * 53 * 0x100000
                                        }
                                    {
                                        0xffffffffu32
                                            & (0xffffffffu32
                                                << (2 * 53 + 1 - (y >> 20) as c_int))
                                    } else {
                                        0xffffffff
                                    })
                            {
                                if word0!(&mut rv) == (0xfffff | 0x100000 * (1024 + 1023 - 1))
                                    && word1!(&mut rv) == 0xffffffff
                                {
                                    goto_ovfl = true;
                                    break 'corr;
                                }
                                word0!(&mut rv) = (word0!(&mut rv) & 0x7ff00000) + 0x100000;
                                word1!(&mut rv) = 0;

                                bc.dsign = 0;

                                break 'corr;
                            }
                        } else if (word0!(&mut rv) & 0xfffff) == 0 && word1!(&mut rv) == 0 {
                            drop_down = true;
                        }

                        if drop_down {
                            /* drop_down: */
                            if bc.scale != 0 {
                                L = (word0!(&mut rv) & 0x7ff00000) as c_int;
                                if L <= (2 * 53 + 1) * 0x100000 {
                                    if L > (53 + 2) * 0x100000 {
                                        break 'corr;
                                    }
                                    if bc.nd > nd {
                                        bc.uflchk = 1;
                                        break 'corr;
                                    }
                                    goto_undfl = true;
                                    break 'corr;
                                }
                            }

                            L = ((word0!(&mut rv) & 0x7ff00000) - 0x100000) as c_int;

                            word0!(&mut rv) = (L as ULong) | 0xfffff;
                            word1!(&mut rv) = 0xffffffff;

                            if bc.nd > nd {
                                break 'cont;
                            }

                            break 'corr;
                        }

                        if Lsb1 != 0 {
                            if (word0!(&mut rv) & Lsb1) == 0 {
                                break 'corr;
                            }
                        } else if (word1!(&mut rv) & Lsb) == 0 {
                            break 'corr;
                        }

                        if bc.dsign != 0 {
                            rv.d += sulp(&mut rv, &mut bc);
                        } else {
                            rv.d -= sulp(&mut rv, &mut bc);

                            if rv.d == 0. {
                                if bc.nd > nd {
                                    bc.uflchk = 1;
                                    break 'corr;
                                }
                                goto_undfl = true;
                                break 'corr;
                            }
                        }

                        bc.dsign = 1 - bc.dsign;

                        break 'corr;
                    }
                    aadj = ratio(delta, bs);
                    if aadj <= 2. {
                        if bc.dsign != 0 {
                            aadj = 1.;
                            aadj1 = 1.;
                        } else if word1!(&mut rv) != 0 || (word0!(&mut rv) & 0xfffff) != 0 {
                            if word1!(&mut rv) == 1 && word0!(&mut rv) == 0 {
                                if bc.nd > nd {
                                    bc.uflchk = 1;
                                    break 'corr;
                                }
                                goto_undfl = true;
                                break 'corr;
                            }

                            aadj = 1.;
                            aadj1 = -1.;
                        } else {
                            if aadj < 2. / 2. {
                                aadj = 1. / 2.;
                            } else {
                                aadj *= 0.5;
                            }
                            aadj1 = -aadj;
                        }
                    } else {
                        aadj *= 0.5;
                        aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                    }
                    y = word0!(&mut rv) & 0x7ff00000;

                    if y == 0x100000 * (1024 + 1023 - 1) {
                        rv0.d = rv.d;
                        word0!(&mut rv) = word0!(&mut rv).wrapping_sub(53 * 0x100000);
                        adj.d = aadj1 * ulp(&mut rv);
                        rv.d += adj.d;
                        if (word0!(&mut rv) & 0x7ff00000) >= 0x100000 * (1024 + 1023 - 53) {
                            if word0!(&mut rv0) == (0xfffff | 0x100000 * (1024 + 1023 - 1))
                                && word1!(&mut rv0) == 0xffffffff
                            {
                                goto_ovfl = true;
                                break 'corr;
                            }
                            word0!(&mut rv) = 0xfffff | 0x100000 * (1024 + 1023 - 1);
                            word1!(&mut rv) = 0xffffffff;
                            break 'cont;
                        } else {
                            word0!(&mut rv) = word0!(&mut rv).wrapping_add(53 * 0x100000);
                        }
                    } else if bc.scale != 0 && y <= 2 * 53 * 0x100000 {
                        if aadj <= 2147483647. {
                            z = aadj as ULong;
                            if (z as i32) <= 0 {
                                z = 1;
                            }
                            aadj = z as f64;
                            aadj1 = if bc.dsign != 0 { aadj } else { -aadj };
                        }
                        aadj2.d = aadj1;
                        word0!(&mut aadj2) = word0!(&mut aadj2)
                            .wrapping_add(((2 * 53 + 1) * 0x100000 - y as c_int) as ULong);
                        aadj1 = aadj2.d;
                        adj.d = aadj1 * ulp(&mut rv);
                        rv.d += adj.d;
                        if rv.d == 0. {
                            req_bigcomp = 1;
                            break 'corr;
                        }
                    } else {
                        adj.d = aadj1 * ulp(&mut rv);
                        rv.d += adj.d;
                    }
                    z = word0!(&mut rv) & 0x7ff00000;

                    if bc.nd == nd && bc.scale == 0 && y == z {
                        L = aadj as c_int;
                        aadj -= L as f64;

                        if bc.dsign != 0
                            || word1!(&mut rv) != 0
                            || (word0!(&mut rv) & 0xfffff) != 0
                        {
                            if aadj < 0.4999999 || aadj > 0.5000001 {
                                break 'corr;
                            }
                        } else if aadj < 0.4999999 / 2. {
                            break 'corr;
                        }
                    }
                }
                /* cont: */
                Bfree(bb);
                Bfree(bd);
                Bfree(bs);
                Bfree(delta);
            }
            Bfree(bb);
            Bfree(bd);
            Bfree(bs);
            Bfree(bd0);
            Bfree(delta);

            if goto_ovfl {
                word0!(&mut rv) = 0x7ff00000;
                word1!(&mut rv) = 0;
                bd0 = core::ptr::null_mut();
                break 'range_err;
            }
            if goto_undfl {
                rv.d = 0.;
                bd0 = core::ptr::null_mut();
                break 'range_err;
            }

            if req_bigcomp != 0 {
                bd0 = core::ptr::null_mut();
                bc.e0 += nz1;
                bigcomp(&mut rv, s0, &mut bc);
                y = word0!(&mut rv) & 0x7ff00000;
                if y == 0x7ff00000 {
                    word0!(&mut rv) = 0x7ff00000;
                    word1!(&mut rv) = 0;
                    break 'range_err;
                }
                if y == 0 && rv.d == 0. {
                    rv.d = 0.;
                    break 'range_err;
                }
            }

            if bc.scale != 0 {
                word0!(&mut rv0) = 0x3ff00000 - 2 * 53 * 0x100000;
                word1!(&mut rv0) = 0;
                rv.d *= rv0.d;

                if (word0!(&mut rv) & 0x7ff00000) == 0 {
                    ffi::set_errno(ffi::ERANGE);
                }
            }

            break 'ret;
        }
        /* range_err: */
        if !bd0.is_null() {
            Bfree(bb);
            Bfree(bd);
            Bfree(bs);
            Bfree(bd0);
            Bfree(delta);
        }
        ffi::set_errno(ffi::ERANGE);
    }
    /* ret: */
    if !se.is_null() {
        *se = s as *mut c_char;
    }
    if sign != 0 {
        -rv.d
    } else {
        rv.d
    }
}

unsafe fn rv_alloc(i: c_int) -> *mut c_char {
    let mut j: usize;
    let mut k: c_int;
    let r: *mut c_int;

    j = core::mem::size_of::<ULong>();
    k = 0;
    while core::mem::size_of::<Bigint>() - core::mem::size_of::<ULong>()
        - core::mem::size_of::<c_int>()
        + j
        <= i as usize
    {
        j <<= 1;
        k += 1;
    }
    r = Balloc(k) as *mut c_int;
    *r = k;

    DTOA_RESULT = r.add(1) as *mut c_char;
    DTOA_RESULT
}

unsafe fn nrv_alloc(
    s_in: *const c_char,
    s0_in: *mut c_char,
    s0len: usize,
    rve: *mut *mut c_char,
    n: c_int,
) -> *mut c_char {
    let mut s = s_in;
    let rv: *mut c_char;
    let mut t: *mut c_char;
    let mut s0 = s0_in;

    if s0.is_null() {
        s0 = rv_alloc(n);
    } else if s0len <= n as usize {
        rv = core::ptr::null_mut();
        t = rv.offset(n as isize);
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
    /* rve_chk: */
    if !rve.is_null() {
        *rve = t;
    }
    rv
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freedtoa(s: *mut c_char) {
    let b = (s as *mut c_int).offset(-1) as *mut Bigint;
    (*b).k = *(b as *mut c_int);
    (*b).maxwds = 1 << (*b).k;
    Bfree(b);

    if s == DTOA_RESULT {
        DTOA_RESULT = core::ptr::null_mut();
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum St {
    UseExact,
    NoDiv,
    UlpReached,
    Roundup,
    Toobig,
    FastFailed,
    FastFailed1,
    NoDigits,
    OneDigit,
    Ret,
    Retc,
    Ret1,
}

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

    let mut bbits: c_int = 0;
    let mut b2: c_int;
    let mut b5: c_int;
    let mut be: c_int;
    let mut dig: c_int = 0;
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

    let mut b: *mut Bigint = core::ptr::null_mut();
    let mut b1: *mut Bigint;
    let mut delta: *mut Bigint;
    let mut mlo: *mut Bigint = core::ptr::null_mut();
    let mut mhi: *mut Bigint = core::ptr::null_mut();
    let mut S: *mut Bigint = core::ptr::null_mut();
    let mut u = U::zero();
    let mut s: *mut c_char;

    let mut p10: *const BF96;
    let mut dbhi: ULLong;
    let mut dbits: ULLong;
    let mut dblo: ULLong;
    let mut den: ULLong;
    let mut hb: ULLong;
    let mut rb: ULLong;
    let mut rblo: ULLong;
    let mut res: ULLong = 0;
    let mut res0: ULLong = 0;
    let mut res3: ULLong = 0;
    let mut reslo: ULLong;
    let mut sres: ULLong;
    let mut sulp_: ULLong;
    let mut tv0: ULLong;
    let mut tv1: ULLong;
    let mut tv2: ULLong;
    let mut tv3: ULLong;
    let mut ulp_: ULLong = 0;
    let mut ulplo: ULLong = 0;
    let mut ulpmask: ULLong = 0;
    let mut ures: ULLong = 0;
    let mut ureslo: ULLong;
    let mut zb: ULLong;
    let mut eulp: c_int = 0;
    let k1: c_int;
    let n2: c_int;
    let mut ulpadj: c_int;
    let mut ulpshift: c_int = 0;

    u.d = dd;
    if (word0!(&mut u) & 0x80000000) != 0 {
        *sign = 1;
        word0!(&mut u) &= !0x80000000;
    } else {
        *sign = 0;
    }

    if (word0!(&mut u) & 0x7ff00000) == 0x7ff00000 {
        *decpt = 9999;

        if word1!(&mut u) == 0 && (word0!(&mut u) & 0xfffff) == 0 {
            return nrv_alloc(b"Infinity\0".as_ptr() as *const c_char, buf, blen, rve, 8);
        }

        return nrv_alloc(b"NaN\0".as_ptr() as *const c_char, buf, blen, rve, 3);
    }

    if u.d == 0. {
        *decpt = 1;
        return nrv_alloc(b"0\0".as_ptr() as *const c_char, buf, blen, rve, 1);
    }
    dbits = (u.LL & 0xfffffffffffff) << 11;
    be = (u.LL >> 52) as c_int;
    if be != 0 {
        dbits |= 0x8000000000000000u64;
        denorm = 0;
        ulpadj = 0;
    } else {
        denorm = 1;
        ulpadj = be + 1;
        dbits <<= 1;
        if (dbits & 0xffffffff00000000u64) == 0 {
            dbits <<= 32;
            be -= 32;
        }
        if (dbits & 0xffff000000000000u64) == 0 {
            dbits <<= 16;
            be -= 16;
        }
        if (dbits & 0xff00000000000000u64) == 0 {
            dbits <<= 8;
            be -= 8;
        }
        if (dbits & 0xf000000000000000u64) == 0 {
            dbits <<= 4;
            be -= 4;
        }
        if (dbits & 0xc000000000000000u64) == 0 {
            dbits <<= 2;
            be -= 2;
        }
        if (dbits & 0x8000000000000000u64) == 0 {
            dbits <<= 1;
            be -= 1;
        }
        ulpadj -= be;
    }
    j = lhint(be + 51);
    p10 = pten(j);
    dbhi = dbits >> 32;
    dblo = dbits & 0xffffffff;
    i = be - 0x3fe;
    if i < (*p10).e
        || (i == (*p10).e
            && (dbhi < (*p10).b0 as ULLong
                || (dbhi == (*p10).b0 as ULLong && dblo < (*p10).b1 as ULLong)))
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
            i = ndigits;
            ilim = ndigits;
            ilim1 = ndigits;
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
        blen = core::mem::size_of::<Bigint>()
            + ((1usize << *(buf as *mut c_int).offset(-1)) - 1) * core::mem::size_of::<ULong>()
            - core::mem::size_of::<c_int>();
    } else if blen <= i as usize {
        buf = core::ptr::null_mut();
        if !rve.is_null() {
            *rve = buf.offset(i as isize);
        }
        return buf;
    }
    s = buf;

    spec_case = 0;
    if mode < 2 || leftright != 0 {
        if word1!(&mut u) == 0
            && (word0!(&mut u) & 0xfffff) == 0
            && (word0!(&mut u) & (0x7ff00000 & !0x100000)) != 0
        {
            spec_case = 1;
        }
    }

    /* ---- state machine ---- */
    let mut st: St;

    b = core::ptr::null_mut();
    if ilim < 0 && (mode == 3 || mode == 5) {
        S = core::ptr::null_mut();
        mhi = core::ptr::null_mut();
        st = St::NoDigits;
    } else {
        i = 1;
        j = 52 + 0x3ff - be;
        ulpshift = 0;
        ulplo = 0;

        let mut chosen: Option<St> = None;

        if k < 0 {
            if k < -25 {
                chosen = Some(St::Toobig);
            } else {
                res = dbits >> 11;
                k1 = -(k + 1);
                n2 = PFIVEBITS[k1 as usize] + 53;
                j1 = j;
                let _ = j1;
                if n2 > 61 {
                    ulpshift = n2 - 61;
                    ulpmask = (1u64 << ulpshift) - 1;
                    if (res & ulpmask) != 0 {
                        chosen = Some(St::Toobig);
                    } else {
                        j -= ulpshift;
                        res >>= ulpshift;
                    }
                }

                if chosen.is_none() {
                    ulp_ = pfive(k1);
                    res *= ulp_;
                    if ulpshift != 0 {
                        ulplo = ulp_;
                        ulp_ >>= ulpshift;
                    }
                    j += k;
                    if ilim == 0 {
                        S = core::ptr::null_mut();
                        mhi = core::ptr::null_mut();
                        if res > (5u64 << j) {
                            chosen = Some(St::OneDigit);
                        } else {
                            chosen = Some(St::NoDigits);
                        }
                    } else {
                        chosen = Some(St::NoDiv);
                    }
                }
            }
        } else if ilim == 0 && j + k >= 0 {
            S = core::ptr::null_mut();
            mhi = core::ptr::null_mut();
            if (dbits >> 11) > (pfive(k - 1) << j) {
                chosen = Some(St::OneDigit);
            } else {
                chosen = Some(St::NoDigits);
            }
        } else if k <= dtoa_divmax && j + k >= 0 {
            chosen = Some(St::UseExact);
        } else {
            chosen = Some(St::Toobig);
        }

        st = chosen.unwrap();
    }

    'sm: loop {
        match st {
            St::UseExact => {
                res = dbits >> 11;
                ulp_ = 1;
                if k <= 0 {
                    st = St::NoDiv;
                    continue 'sm;
                }
                j1 = j + k + 1;
                den = pfive(k - i) << (j1 - i);
                let mut next = St::NoDiv;
                loop {
                    dig = (res / den) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res -= (dig as ULLong) * den;
                    if res == 0 {
                        next = St::Retc;
                        break;
                    }
                    if ilim < 0 {
                        ures = den - res;
                        if 2 * res <= ulp_
                            && (if spec_case != 0 {
                                4 * res <= ulp_
                            } else {
                                2 * res < ulp_ || (dig & 1) != 0
                            })
                        {
                            next = St::UlpReached;
                            break;
                        }
                        if 2 * ures < ulp_ {
                            next = St::Roundup;
                            break;
                        }
                    } else if i == ilim {
                        ures = 2 * res;
                        if ures > den
                            || (ures == den && (dig & 1) != 0)
                            || (spec_case != 0 && res <= ulp_ && 2 * res >= ulp_)
                        {
                            next = St::Roundup;
                        } else {
                            next = St::Retc;
                        }
                        break;
                    }
                    i += 1;
                    if j1 < i {
                        res *= 10;
                        ulp_ *= 10;
                    } else {
                        if i > k {
                            next = St::NoDiv;
                            break;
                        }
                        den = pfive(k - i) << (j1 - i);
                    }
                }
                st = next;
                continue 'sm;
            }

            St::NoDiv | St::UlpReached => {
                let mut next: St;
                if st == St::UlpReached {
                    /* entered from the exact loop: finish the ulp_reached tail */
                    if ures < res || (ures == res && (dig & 1) != 0) {
                        st = St::Roundup;
                    } else {
                        st = St::Retc;
                    }
                    continue 'sm;
                }
                loop {
                    den = res >> j;
                    dig = den as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res -= den << j;
                    if res == 0 {
                        next = St::Retc;
                        break;
                    }
                    if ilim < 0 {
                        ures = (1u64 << j) - res;
                        if 2 * res <= ulp_
                            && (if spec_case != 0 {
                                4 * res <= ulp_
                            } else {
                                2 * res < ulp_ || (dig & 1) != 0
                            })
                        {
                            /* ulp_reached: */
                            if ures < res || (ures == res && (dig & 1) != 0) {
                                next = St::Roundup;
                            } else {
                                next = St::Retc;
                            }
                            break;
                        }
                        if 2 * ures < ulp_ {
                            next = St::Roundup;
                            break;
                        }
                    }
                    j -= 1;
                    if i == ilim {
                        hb = 1u64 << j;
                        if (res & hb) != 0 && ((dig & 1) != 0 || (res & (hb - 1)) != 0) {
                            next = St::Roundup;
                            break;
                        }
                        if spec_case != 0 && res <= ulp_ && 2 * res >= ulp_ {
                            next = St::Roundup;
                        } else {
                            next = St::Retc;
                        }
                        break;
                    }
                    i += 1;
                    res *= 5;
                    if ulpshift != 0 {
                        ulplo = 5 * (ulplo & ulpmask);
                        ulp_ = 5 * ulp_ + (ulplo >> ulpshift);
                    } else {
                        ulp_ *= 5;
                    }
                }
                st = next;
                continue 'sm;
            }

            St::Roundup => {
                loop {
                    s = s.offset(-1);
                    if *s != b'9' as c_char {
                        break;
                    }
                    if s == buf {
                        k += 1;
                        *s = b'1' as c_char;
                        s = s.add(1);
                        st = St::Ret1;
                        continue 'sm;
                    }
                }
                *s += 1;
                s = s.add(1);
                st = St::Ret1;
                continue 'sm;
            }

            St::Toobig => {
                if ilim > 28 {
                    st = St::FastFailed1;
                    continue 'sm;
                }

                p10 = pten(342 - k);
                tv0 = (*p10).b2 as ULLong * dblo;
                tv1 = (*p10).b1 as ULLong * dblo + (tv0 >> 32);
                tv2 = (*p10).b2 as ULLong * dbhi + (tv1 & 0xffffffff);
                tv3 = (*p10).b0 as ULLong * dblo + (tv1 >> 32) + (tv2 >> 32);
                res3 = (*p10).b1 as ULLong * dbhi + (tv3 & 0xffffffff);
                res = (*p10).b0 as ULLong * dbhi + (tv3 >> 32) + (res3 >> 32);
                be += (*p10).e - 0x3fe;
                j1 = be - 54 + ulpadj;
                eulp = j1;
                if (res & 0x8000000000000000u64) == 0 {
                    be -= 1;
                    res3 <<= 1;
                    res = (res << 1) | ((res3 & 0x100000000u64) >> 32);
                }
                res0 = res;

                if ilim > 19 {
                    st = St::FastFailed;
                    continue 'sm;
                }
                res >>= 4 - be;
                ulp_ = (*p10).b0 as ULLong;
                ulp_ = (ulp_ << 29) | ((*p10).b1 as ULLong >> 3);

                if ilim == 0 {
                    if (res & 0x7fffffffffffffeu64) == 0 || ((!res) & 0x7fffffffffffffeu64) == 0 {
                        st = St::FastFailed1;
                        continue 'sm;
                    }
                    S = core::ptr::null_mut();
                    mhi = core::ptr::null_mut();
                    if res >= 0x5000000000000000u64 {
                        st = St::OneDigit;
                    } else {
                        st = St::NoDigits;
                    }
                    continue 'sm;
                }
                rb = 1;
                let mut next: St;
                loop {
                    dig = (res >> 60) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res &= 0xfffffffffffffff;
                    if ilim < 0 {
                        ures = 0x1000000000000000u64 - res;
                        if eulp > 0 {
                            sulp_ = ulp_ << (eulp - 1);
                            if res <= ures {
                                if res + rb > ures - rb {
                                    next = St::FastFailed;
                                    break;
                                }
                                if res < sulp_ {
                                    next = St::Retc;
                                    break;
                                }
                            } else {
                                if res - rb <= ures + rb {
                                    next = St::FastFailed;
                                    break;
                                }
                                if ures < sulp_ {
                                    next = St::Roundup;
                                    break;
                                }
                            }
                        } else {
                            zb = (1u64 << (eulp + 63)).wrapping_neg();
                            let mut handled = false;
                            let mut nx = St::Retc;
                            if (zb & res) == 0 {
                                sres = res << (1 - eulp);
                                if sres < ulp_ && (spec_case == 0 || 2 * sres < ulp_) {
                                    if (res + rb) << (1 - eulp) >= ulp_ {
                                        nx = St::FastFailed;
                                        handled = true;
                                    } else if ures < res {
                                        if ures + rb >= res - rb {
                                            nx = St::FastFailed;
                                        } else {
                                            nx = St::Roundup;
                                        }
                                        handled = true;
                                    } else if ures - rb < res + rb {
                                        nx = St::FastFailed;
                                        handled = true;
                                    } else {
                                        nx = St::Retc;
                                        handled = true;
                                    }
                                }
                            }
                            if handled {
                                next = nx;
                                break;
                            }
                            if (zb & ures) == 0 && ures << (-eulp) < ulp_ {
                                if ures << (1 - eulp) < ulp_ {
                                    next = St::Roundup;
                                } else {
                                    next = St::FastFailed;
                                }
                                break;
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64 - res;
                        if ures < res {
                            if ures <= rb || res - rb <= ures + rb {
                                if j + k >= 0 && k >= 0 && k <= 27 {
                                    /* use_exact1 */
                                    s = buf;
                                    i = 1;
                                    next = St::UseExact;
                                    break;
                                }
                                next = St::FastFailed;
                                break;
                            }
                            next = St::Roundup;
                            break;
                        }
                        if res <= rb || ures - rb <= res + rb {
                            if j + k >= 0 && k >= 0 && k <= 27 {
                                /* use_exact1 */
                                s = buf;
                                i = 1;
                                next = St::UseExact;
                                break;
                            }
                            next = St::FastFailed;
                            break;
                        }
                        next = St::Retc;
                        break;
                    }
                    rb *= 10;
                    if rb >= 0x1000000000000000u64 {
                        next = St::FastFailed;
                        break;
                    }
                    res *= 10;
                    ulp_ *= 5;
                    if (ulp_ & 0x8000000000000000u64) != 0 {
                        eulp += 4;
                        ulp_ >>= 3;
                    } else {
                        eulp += 3;
                        ulp_ >>= 2;
                    }
                    i += 1;
                }
                st = next;
                continue 'sm;
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
                ulp_ = (*p10).b0 as ULLong;
                ulp_ = (ulp_ << 29) | ((*p10).b1 as ULLong >> 3);
                eulp = j1;
                i = 1;
                let mut next: St;
                loop {
                    let mut more96 = false;
                    dig = (res >> 60) as c_int;
                    *s = (b'0' as c_int + dig) as c_char;
                    s = s.add(1);
                    res &= 0xfffffffffffffff;
                    if ilim < 0 {
                        ures = 0x1000000000000000u64 - res;
                        ureslo = 0;
                        if reslo != 0 {
                            ureslo = 0x100000000u64 - reslo;
                            ures -= 1;
                        }
                        if eulp > 0 {
                            sulp_ = (ulp_ << (eulp - 1)) - rb;
                            if res <= ures {
                                if res < sulp_ && res + rb < ures - rb {
                                    next = St::Retc;
                                    break;
                                }
                            } else if ures < sulp_ && res - rb > ures + rb {
                                next = St::Roundup;
                                break;
                            }
                            next = St::FastFailed1;
                            break;
                        } else {
                            zb = (1u64 << (eulp + 60)).wrapping_neg();
                            let mut handled = false;
                            let mut nx = St::Retc;
                            if (zb & (res + rb)) == 0 {
                                sres = (res - rb) << (1 - eulp);
                                if sres < ulp_ && (spec_case == 0 || 2 * sres < ulp_) {
                                    sres = res << (1 - eulp);
                                    j = eulp + 31;
                                    if j > 0 {
                                        sres += (rblo + reslo) >> j;
                                    } else {
                                        sres += (rblo + reslo) << (-j);
                                    }
                                    if sres + (rb << (1 - eulp)) >= ulp_ {
                                        nx = St::FastFailed1;
                                        handled = true;
                                    } else if sres >= ulp_ {
                                        more96 = true;
                                    } else if ures < res || (ures == res && ureslo < reslo) {
                                        if ures + rb >= res - rb {
                                            nx = St::FastFailed1;
                                        } else {
                                            nx = St::Roundup;
                                        }
                                        handled = true;
                                    } else if ures - rb <= res + rb {
                                        nx = St::FastFailed1;
                                        handled = true;
                                    } else {
                                        nx = St::Retc;
                                        handled = true;
                                    }
                                }
                            }
                            if handled {
                                next = nx;
                                break;
                            }
                            if !more96
                                && (zb & ures) == 0
                                && (ures - rb) << (1 - eulp) < ulp_
                            {
                                if (ures + rb) << (1 - eulp) < ulp_ {
                                    next = St::Roundup;
                                } else {
                                    next = St::FastFailed1;
                                }
                                break;
                            }
                        }
                    } else if i == ilim {
                        ures = 0x1000000000000000u64 - res;
                        sres = 0;
                        ureslo = 0;
                        if reslo != 0 {
                            ureslo = 0x100000000u64 - reslo;
                            ures -= 1;
                            sres = (reslo + rblo) >> 31;
                        }
                        let _ = ureslo;
                        sres += 2 * rb;
                        if ures <= res {
                            if ures <= sres || res - ures <= sres {
                                next = St::FastFailed1;
                                break;
                            }
                            next = St::Roundup;
                            break;
                        }
                        if res <= sres || ures - res <= sres {
                            next = St::FastFailed1;
                            break;
                        }
                        next = St::Retc;
                        break;
                    }
                    /* more96: */
                    let _ = more96;
                    rblo *= 10;
                    rb = 10 * rb + (rblo >> 32);
                    rblo &= 0xffffffff;
                    if rb >= 0x1000000000000000u64 {
                        next = St::FastFailed1;
                        break;
                    }
                    reslo *= 10;
                    res = 10 * res + (reslo >> 32);
                    reslo &= 0xffffffff;
                    ulp_ *= 5;
                    if (ulp_ & 0x8000000000000000u64) != 0 {
                        eulp += 4;
                        ulp_ >>= 3;
                    } else {
                        eulp += 3;
                        ulp_ >>= 2;
                    }
                    i += 1;
                }
                st = next;
                continue 'sm;
            }

            St::FastFailed1 => {
                S = core::ptr::null_mut();
                mhi = core::ptr::null_mut();
                mlo = core::ptr::null_mut();

                b = d2b(&mut u, &mut be, &mut bbits);

                s = buf;
                i = ((word0!(&mut u) >> 20) & (0x7ff00000u32 >> 20)) as c_int;
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
                mhi = core::ptr::null_mut();
                mlo = core::ptr::null_mut();
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
                    S = multadd(S, 5, 0);
                    if ilim < 0 || cmp(b, S) <= 0 {
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

                        if j1 == 0 && mode != 1 && (word1!(&mut u) & 1) == 0 {
                            if dig == b'9' as c_int {
                                /* round_9_up */
                                *s = b'9' as c_char;
                                s = s.add(1);
                                st = St::Roundup;
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

                        if j < 0 || (j == 0 && mode != 1 && (word1!(&mut u) & 1) == 0) {
                            let mut accept = false;
                            if xg(b, 0) == 0 && (*b).wds <= 1 {
                                accept = true;
                            }

                            if !accept && j1 > 0 {
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
                                    st = St::Roundup;
                                    continue 'sm;
                                }
                            }
                            /* accept_dig: */
                            *s = dig as c_char;
                            s = s.add(1);
                            st = St::Ret;
                            continue 'sm;
                        }
                        if j1 > 0 {
                            if dig == b'9' as c_int {
                                /* round_9_up */
                                *s = b'9' as c_char;
                                s = s.add(1);
                                st = St::Roundup;
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
                        dig = quorem(b, S) + b'0' as c_int;
                        *s = dig as c_char;
                        s = s.add(1);
                        if xg(b, 0) == 0 && (*b).wds <= 1 {
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
                b = lshift(b, 1);
                j = cmp(b, S);

                if j > 0 || (j == 0 && (dig & 1) != 0) {
                    /* roundoff: */
                    let mut done = false;
                    loop {
                        s = s.offset(-1);
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
                }
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

            St::Ret => {
                Bfree(S);
                if !mhi.is_null() {
                    if !mlo.is_null() && mlo != mhi {
                        Bfree(mlo);
                    }
                    Bfree(mhi);
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
    if !DTOA_RESULT.is_null() {
        freedtoa(DTOA_RESULT);
    }

    dtoa_r(
        dd,
        mode,
        ndigits,
        decpt,
        sign,
        rve,
        core::ptr::null_mut(),
        0,
    )
}
