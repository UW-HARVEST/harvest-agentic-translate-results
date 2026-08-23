//! Group element arithmetic, translated from ed25519_ref10.c (ge25519_*).
use crate::ed25519::fe25519::*;
use crate::ed25519::base::BASE;
use crate::ed25519::base2::BI;

#[derive(Clone, Copy)]
pub struct P2 {
    pub x: Fe,
    pub y: Fe,
    pub z: Fe,
}
#[derive(Clone, Copy)]
pub struct P3 {
    pub x: Fe,
    pub y: Fe,
    pub z: Fe,
    pub t: Fe,
}
#[derive(Clone, Copy)]
pub struct P1p1 {
    pub x: Fe,
    pub y: Fe,
    pub z: Fe,
    pub t: Fe,
}
#[derive(Clone, Copy)]
pub struct Precomp {
    pub yplusx: Fe,
    pub yminusx: Fe,
    pub xy2d: Fe,
}
#[derive(Clone, Copy)]
pub struct Cached {
    pub yplusx: Fe,
    pub yminusx: Fe,
    pub z: Fe,
    pub t2d: Fe,
}

impl P2 {
    fn new() -> P2 {
        P2 { x: fe_0(), y: fe_0(), z: fe_0() }
    }
}
impl P3 {
    pub fn new() -> P3 {
        P3 { x: fe_0(), y: fe_0(), z: fe_0(), t: fe_0() }
    }
}
impl P1p1 {
    fn new() -> P1p1 {
        P1p1 { x: fe_0(), y: fe_0(), z: fe_0(), t: fe_0() }
    }
}
impl Precomp {
    fn zero() -> Precomp {
        Precomp { yplusx: fe_1(), yminusx: fe_1(), xy2d: fe_0() }
    }
}
impl Cached {
    fn zero() -> Cached {
        Cached { yplusx: fe_1(), yminusx: fe_1(), z: fe_1(), t2d: fe_0() }
    }
}

/// optblocker: C declares `static volatile unsigned char optblocker_u8;` = 0.
pub const OPTBLOCKER: u8 = 0;

fn add_cached(p: &P3, q: &Cached) -> P1p1 {
    let mut r = P1p1::new();
    r.x = fe_add(p.y, p.x);
    r.y = fe_sub(p.y, p.x);
    r.z = fe_mul(r.x, q.yplusx);
    r.y = fe_mul(r.y, q.yminusx);
    r.t = fe_mul(q.t2d, p.t);
    r.x = fe_mul(p.z, q.z);
    let t0 = fe_add(r.x, r.x);
    r.x = fe_sub(r.z, r.y);
    r.y = fe_add(r.z, r.y);
    r.z = fe_add(t0, r.t);
    r.t = fe_sub(t0, r.t);
    r
}

fn sub_cached(p: &P3, q: &Cached) -> P1p1 {
    let mut r = P1p1::new();
    r.x = fe_add(p.y, p.x);
    r.y = fe_sub(p.y, p.x);
    r.z = fe_mul(r.x, q.yminusx);
    r.y = fe_mul(r.y, q.yplusx);
    r.t = fe_mul(q.t2d, p.t);
    r.x = fe_mul(p.z, q.z);
    let t0 = fe_add(r.x, r.x);
    r.x = fe_sub(r.z, r.y);
    r.y = fe_add(r.z, r.y);
    r.z = fe_sub(t0, r.t);
    r.t = fe_add(t0, r.t);
    r
}

fn add_precomp(p: &P3, q: &Precomp) -> P1p1 {
    let mut r = P1p1::new();
    r.x = fe_add(p.y, p.x);
    r.y = fe_sub(p.y, p.x);
    r.z = fe_mul(r.x, q.yplusx);
    r.y = fe_mul(r.y, q.yminusx);
    r.t = fe_mul(q.xy2d, p.t);
    let t0 = fe_add(p.z, p.z);
    r.x = fe_sub(r.z, r.y);
    r.y = fe_add(r.z, r.y);
    r.z = fe_add(t0, r.t);
    r.t = fe_sub(t0, r.t);
    r
}

fn sub_precomp(p: &P3, q: &Precomp) -> P1p1 {
    let mut r = P1p1::new();
    r.x = fe_add(p.y, p.x);
    r.y = fe_sub(p.y, p.x);
    r.z = fe_mul(r.x, q.yminusx);
    r.y = fe_mul(r.y, q.yplusx);
    r.t = fe_mul(q.xy2d, p.t);
    let t0 = fe_add(p.z, p.z);
    r.x = fe_sub(r.z, r.y);
    r.y = fe_add(r.z, r.y);
    r.z = fe_sub(t0, r.t);
    r.t = fe_add(t0, r.t);
    r
}

pub fn p1p1_to_p2(p: &P1p1) -> P2 {
    let mut r = P2::new();
    r.x = fe_mul(p.x, p.t);
    r.y = fe_mul(p.y, p.z);
    r.z = fe_mul(p.z, p.t);
    r
}

pub fn p1p1_to_p3(p: &P1p1) -> P3 {
    let mut r = P3::new();
    r.x = fe_mul(p.x, p.t);
    r.y = fe_mul(p.y, p.z);
    r.z = fe_mul(p.z, p.t);
    r.t = fe_mul(p.x, p.y);
    r
}

pub fn p2_to_p3(p: &P2) -> P3 {
    let mut r = P3::new();
    r.x = p.x;
    r.y = p.y;
    r.z = p.z;
    r.t = fe_mul(p.x, p.y);
    r
}

fn p2_0() -> P2 {
    P2 { x: fe_0(), y: fe_1(), z: fe_1() }
}

fn p2_dbl(p: &P2) -> P1p1 {
    let mut r = P1p1::new();
    r.x = fe_sq(p.x);
    r.z = fe_sq(p.y);
    r.t = fe_sq2(p.z);
    r.y = fe_add(p.x, p.y);
    let t0 = fe_sq(r.y);
    r.y = fe_add(r.z, r.x);
    r.z = fe_sub(r.z, r.x);
    r.x = fe_sub(t0, r.y);
    r.t = fe_sub(r.t, r.z);
    r
}

fn p3_0() -> P3 {
    P3 { x: fe_0(), y: fe_1(), z: fe_1(), t: fe_0() }
}

fn p3_to_cached(p: &P3) -> Cached {
    let mut r = Cached::zero();
    r.yplusx = fe_add(p.y, p.x);
    r.yminusx = fe_sub(p.y, p.x);
    r.z = p.z;
    r.t2d = fe_mul(p.t, ED25519_D2);
    r
}

fn p3_to_p2(p: &P3) -> P2 {
    P2 { x: p.x, y: p.y, z: p.z }
}

pub fn p3_tobytes(h: &P3) -> [u8; 32] {
    let recip = fe_invert(h.z);
    let x = fe_mul(h.x, recip);
    let y = fe_mul(h.y, recip);
    let mut s = fe_tobytes(y);
    s[31] ^= (fe_isnegative(&x) << 7) as u8;
    s
}

pub fn tobytes_p2(h: &P2) -> [u8; 32] {
    let recip = fe_invert(h.z);
    let x = fe_mul(h.x, recip);
    let y = fe_mul(h.y, recip);
    let mut s = fe_tobytes(y);
    s[31] ^= (fe_isnegative(&x) << 7) as u8;
    s
}

fn p3_dbl(p: &P3) -> P1p1 {
    let q = p3_to_p2(p);
    p2_dbl(&q)
}

fn equal(b: i8, c: i8) -> u8 {
    let x = (b as u8) ^ (c as u8);
    let mut y = x as u32;
    y = y.wrapping_sub(1);
    (((y >> 29) as u8) ^ OPTBLOCKER) >> 2
}

fn negative(b: i8) -> u8 {
    let x = b as u8;
    ((x >> 5) ^ OPTBLOCKER) >> 2
}

fn ge_cmov(t: &mut Precomp, u: &Precomp, b: u8) {
    fe_cmov(&mut t.yplusx, &u.yplusx, b as u32);
    fe_cmov(&mut t.yminusx, &u.yminusx, b as u32);
    fe_cmov(&mut t.xy2d, &u.xy2d, b as u32);
}

fn ge_cmov_cached(t: &mut Cached, u: &Cached, b: u8) {
    fe_cmov(&mut t.yplusx, &u.yplusx, b as u32);
    fe_cmov(&mut t.yminusx, &u.yminusx, b as u32);
    fe_cmov(&mut t.z, &u.z, b as u32);
    fe_cmov(&mut t.t2d, &u.t2d, b as u32);
}

fn cmov8(precomp: &[Precomp; 8], b: i8) -> Precomp {
    let bnegative = negative(b);
    let babs = (b as i32 - (((bnegative as i8).wrapping_neg() as i32 & b as i32) * 2)) as i8;
    let mut t = Precomp::zero();
    ge_cmov(&mut t, &precomp[0], equal(babs, 1));
    ge_cmov(&mut t, &precomp[1], equal(babs, 2));
    ge_cmov(&mut t, &precomp[2], equal(babs, 3));
    ge_cmov(&mut t, &precomp[3], equal(babs, 4));
    ge_cmov(&mut t, &precomp[4], equal(babs, 5));
    ge_cmov(&mut t, &precomp[5], equal(babs, 6));
    ge_cmov(&mut t, &precomp[6], equal(babs, 7));
    ge_cmov(&mut t, &precomp[7], equal(babs, 8));
    let mut minust = Precomp::zero();
    minust.yplusx = t.yminusx;
    minust.yminusx = t.yplusx;
    minust.xy2d = fe_neg(t.xy2d);
    ge_cmov(&mut t, &minust, bnegative);
    t
}

fn cmov8_base(pos: usize, b: i8) -> Precomp {
    cmov8(&BASE[pos], b)
}

fn cmov8_cached(cached: &[Cached; 8], b: i8) -> Cached {
    let bnegative = negative(b);
    let babs = (b as i32 - (((bnegative as i8).wrapping_neg() as i32 & b as i32) * 2)) as i8;
    let mut t = Cached::zero();
    ge_cmov_cached(&mut t, &cached[0], equal(babs, 1));
    ge_cmov_cached(&mut t, &cached[1], equal(babs, 2));
    ge_cmov_cached(&mut t, &cached[2], equal(babs, 3));
    ge_cmov_cached(&mut t, &cached[3], equal(babs, 4));
    ge_cmov_cached(&mut t, &cached[4], equal(babs, 5));
    ge_cmov_cached(&mut t, &cached[5], equal(babs, 6));
    ge_cmov_cached(&mut t, &cached[6], equal(babs, 7));
    ge_cmov_cached(&mut t, &cached[7], equal(babs, 8));
    let mut minust = Cached::zero();
    minust.yplusx = t.yminusx;
    minust.yminusx = t.yplusx;
    minust.z = t.z;
    minust.t2d = fe_neg(t.t2d);
    ge_cmov_cached(&mut t, &minust, bnegative);
    t
}

fn slide_vartime(a: &[u8]) -> [i8; 256] {
    let mut r = [0i8; 256];
    for i in 0..256 {
        r[i] = (1 & (a[i >> 3] >> (i & 7))) as i8;
    }
    for i in 0..256 {
        if r[i] == 0 {
            continue;
        }
        let mut b = 1;
        while b <= 6 && i + b < 256 {
            if r[i + b] == 0 {
                b += 1;
                continue;
            }
            let ribs = (r[i + b] as i32) << b;
            let mut cmp = r[i] as i32 + ribs;
            if cmp <= 15 {
                r[i] = cmp as i8;
                r[i + b] = 0;
            } else {
                cmp = r[i] as i32 - ribs;
                if cmp < -15 {
                    break;
                }
                r[i] = cmp as i8;
                let mut k = i + b;
                while k < 256 {
                    if r[k] == 0 {
                        r[k] = 1;
                        break;
                    }
                    r[k] = 0;
                    k += 1;
                }
            }
            b += 1;
        }
    }
    r
}

pub fn frombytes(s: &[u8]) -> (P3, i32) {
    let mut h = P3::new();
    h.y = fe_frombytes(s);
    h.z = fe_1();
    let u = fe_sq(h.y);
    let mut v = fe_mul(u, ED25519_D);
    let u = fe_sub(u, h.z); /* u = y^2-1 */
    v = fe_add(v, h.z); /* v = dy^2+1 */

    h.x = fe_mul(u, v);
    h.x = fe_pow22523(h.x);
    h.x = fe_mul(u, h.x);

    let mut vxx = fe_sq(h.x);
    vxx = fe_mul(vxx, v);
    let m_root_check = fe_sub(vxx, u);
    let p_root_check = fe_add(vxx, u);
    let has_m_root = fe_iszero(&m_root_check);
    let has_p_root = fe_iszero(&p_root_check);
    let x_sqrtm1 = fe_mul(h.x, FE25519_SQRTM1);
    fe_cmov(&mut h.x, &x_sqrtm1, (1 - has_m_root) as u32);

    let negx = fe_neg(h.x);
    let cond = (fe_isnegative(&h.x) ^ (((s[31] >> 5) ^ OPTBLOCKER) >> 2) as i32) as u32;
    fe_cmov(&mut h.x, &negx, cond);
    h.t = fe_mul(h.x, h.y);

    (h, (has_m_root | has_p_root) - 1)
}

pub fn frombytes_negate_vartime(s: &[u8]) -> (P3, i32) {
    let mut h = P3::new();
    h.y = fe_frombytes(s);
    h.z = fe_1();
    let u = fe_sq(h.y);
    let mut v = fe_mul(u, ED25519_D);
    let u = fe_sub(u, h.z);
    v = fe_add(v, h.z);

    let mut v3 = fe_sq(v);
    v3 = fe_mul(v3, v);
    h.x = fe_sq(v3);
    h.x = fe_mul(h.x, v);
    h.x = fe_mul(h.x, u);

    h.x = fe_pow22523(h.x);
    h.x = fe_mul(h.x, v3);
    h.x = fe_mul(h.x, u);

    let mut vxx = fe_sq(h.x);
    vxx = fe_mul(vxx, v);
    let m_root_check = fe_sub(vxx, u);
    if fe_iszero(&m_root_check) == 0 {
        let p_root_check = fe_add(vxx, u);
        if fe_iszero(&p_root_check) == 0 {
            return (h, -1);
        }
        h.x = fe_mul(h.x, FE25519_SQRTM1);
    }

    if fe_isnegative(&h.x) == (s[31] >> 7) as i32 {
        h.x = fe_neg(h.x);
    }
    h.t = fe_mul(h.x, h.y);

    (h, 0)
}

pub fn scalarmult_base(a: &[u8]) -> P3 {
    let mut e = [0i8; 64];
    for i in 0..32 {
        e[2 * i] = ((a[i] >> 0) & 15) as i8;
        e[2 * i + 1] = ((a[i] >> 4) & 15) as i8;
    }
    let mut carry: i8 = 0;
    for i in 0..63 {
        e[i] = e[i].wrapping_add(carry);
        carry = e[i].wrapping_add(8);
        carry >>= 4;
        e[i] = e[i].wrapping_sub(carry.wrapping_mul(1 << 4));
    }
    e[63] = e[63].wrapping_add(carry);

    let mut h = p3_0();

    let mut i = 1;
    while i < 64 {
        let t = cmov8_base(i / 2, e[i]);
        let r = add_precomp(&h, &t);
        h = p1p1_to_p3(&r);
        i += 2;
    }

    let mut r = p3_dbl(&h);
    let mut s = p1p1_to_p2(&r);
    r = p2_dbl(&s);
    s = p1p1_to_p2(&r);
    r = p2_dbl(&s);
    s = p1p1_to_p2(&r);
    r = p2_dbl(&s);
    h = p1p1_to_p3(&r);

    let mut i = 0;
    while i < 64 {
        let t = cmov8_base(i / 2, e[i]);
        let r = add_precomp(&h, &t);
        h = p1p1_to_p3(&r);
        i += 2;
    }
    h
}

pub fn scalarmult(a: &[u8], p: &P3) -> P3 {
    let mut pi = [Cached::zero(); 8];
    pi[0] = p3_to_cached(p);

    let t2 = p3_dbl(p);
    let p2 = p1p1_to_p3(&t2);
    pi[1] = p3_to_cached(&p2);

    let t3 = add_cached(p, &pi[1]);
    let p3v = p1p1_to_p3(&t3);
    pi[2] = p3_to_cached(&p3v);

    let t4 = p3_dbl(&p2);
    let p4 = p1p1_to_p3(&t4);
    pi[3] = p3_to_cached(&p4);

    let t5 = add_cached(p, &pi[3]);
    let p5 = p1p1_to_p3(&t5);
    pi[4] = p3_to_cached(&p5);

    let t6 = p3_dbl(&p3v);
    let p6 = p1p1_to_p3(&t6);
    pi[5] = p3_to_cached(&p6);

    let t7 = add_cached(p, &pi[5]);
    let p7 = p1p1_to_p3(&t7);
    pi[6] = p3_to_cached(&p7);

    let t8 = p3_dbl(&p4);
    let p8 = p1p1_to_p3(&t8);
    pi[7] = p3_to_cached(&p8);

    let mut e = [0i8; 64];
    for i in 0..32 {
        e[2 * i] = ((a[i] >> 0) & 15) as i8;
        e[2 * i + 1] = ((a[i] >> 4) & 15) as i8;
    }
    let mut carry: i8 = 0;
    for i in 0..63 {
        e[i] = e[i].wrapping_add(carry);
        carry = e[i].wrapping_add(8);
        carry >>= 4;
        e[i] = e[i].wrapping_sub(carry.wrapping_mul(1 << 4));
    }
    e[63] = e[63].wrapping_add(carry);

    let mut h = p3_0();

    let mut i = 63usize;
    while i != 0 {
        let t = cmov8_cached(&pi, e[i]);
        let r = add_cached(&h, &t);

        let mut s = p1p1_to_p2(&r);
        let mut r2 = p2_dbl(&s);
        s = p1p1_to_p2(&r2);
        r2 = p2_dbl(&s);
        s = p1p1_to_p2(&r2);
        r2 = p2_dbl(&s);
        s = p1p1_to_p2(&r2);
        r2 = p2_dbl(&s);

        h = p1p1_to_p3(&r2);
        i -= 1;
    }
    let t = cmov8_cached(&pi, e[0]);
    let r = add_cached(&h, &t);
    h = p1p1_to_p3(&r);
    h
}

pub fn double_scalarmult_vartime(a: &[u8], big_a: &P3, b: &[u8]) -> P2 {
    let aslide = slide_vartime(a);
    let bslide = slide_vartime(b);

    let mut ai = [Cached::zero(); 8];
    ai[0] = p3_to_cached(big_a);

    let t = p3_dbl(big_a);
    let a2 = p1p1_to_p3(&t);

    for k in 1..8 {
        let t = add_cached(&a2, &ai[k - 1]);
        let u = p1p1_to_p3(&t);
        ai[k] = p3_to_cached(&u);
    }

    let mut r = p2_0();

    let mut i: i32 = 255;
    while i >= 0 {
        if aslide[i as usize] != 0 || bslide[i as usize] != 0 {
            break;
        }
        i -= 1;
    }

    while i >= 0 {
        let mut t = p2_dbl(&r);

        if aslide[i as usize] > 0 {
            let u = p1p1_to_p3(&t);
            t = add_cached(&u, &ai[(aslide[i as usize] / 2) as usize]);
        } else if aslide[i as usize] < 0 {
            let u = p1p1_to_p3(&t);
            t = sub_cached(&u, &ai[((-aslide[i as usize]) / 2) as usize]);
        }

        if bslide[i as usize] > 0 {
            let u = p1p1_to_p3(&t);
            t = add_precomp(&u, &BI[(bslide[i as usize] / 2) as usize]);
        } else if bslide[i as usize] < 0 {
            let u = p1p1_to_p3(&t);
            t = sub_precomp(&u, &BI[((-bslide[i as usize]) / 2) as usize]);
        }

        r = p1p1_to_p2(&t);
        i -= 1;
    }
    r
}

fn p3p3_dbl(p: &P3) -> P3 {
    let p1p1 = p3_dbl(p);
    p1p1_to_p3(&p1p1)
}

fn p3_neg(p: &P3) -> P3 {
    P3 {
        x: fe_neg(p.x),
        y: p.y,
        z: p.z,
        t: fe_neg(p.t),
    }
}

pub fn p3_add(p: &P3, q: &P3) -> P3 {
    let q_cached = p3_to_cached(q);
    let p1p1 = add_cached(p, &q_cached);
    p1p1_to_p3(&p1p1)
}

pub fn p3_sub(p: &P3, q: &P3) -> P3 {
    let q_neg = p3_neg(q);
    p3_add(p, &q_neg)
}

fn p3_dbladd(r: &mut P3, n: i32, q: &P3) {
    let mut p2 = p3_to_p2(r);
    let mut p1p1 = P1p1::new();
    for _ in 0..n {
        p1p1 = p2_dbl(&p2);
        p2 = p1p1_to_p2(&p1p1);
    }
    *r = p1p1_to_p3(&p1p1);
    *r = p3_add(r, q);
}

fn mul_l(p: &P3) -> P3 {
    let _10 = p3p3_dbl(p);
    let _11 = p3_add(p, &_10);
    let _100 = p3_add(p, &_11);
    let _110 = p3_add(&_10, &_100);
    let _1000 = p3_add(&_10, &_110);
    let _1011 = p3_add(&_11, &_1000);
    let _10000 = p3p3_dbl(&_1000);
    let _100000 = p3p3_dbl(&_10000);
    let _100110 = p3_add(&_110, &_100000);
    let _1000000 = p3p3_dbl(&_100000);
    let _1010000 = p3_add(&_10000, &_1000000);
    let _1010011 = p3_add(&_11, &_1010000);
    let _1100011 = p3_add(&_10000, &_1010011);
    let _1100111 = p3_add(&_100, &_1100011);
    let _1101011 = p3_add(&_100, &_1100111);
    let _10010011 = p3_add(&_1000000, &_1010011);
    let _10010111 = p3_add(&_100, &_10010011);
    let _10111101 = p3_add(&_100110, &_10010111);
    let _11010011 = p3_add(&_1000000, &_10010011);
    let _11100111 = p3_add(&_1010000, &_10010111);
    let _11101101 = p3_add(&_110, &_11100111);
    let _11110101 = p3_add(&_1000, &_11101101);

    let mut r = p3_add(&_1011, &_11110101);
    p3_dbladd(&mut r, 126, &_1010011);
    p3_dbladd(&mut r, 9, &_10);
    r = p3_add(&r, &_11110101);
    p3_dbladd(&mut r, 7, &_1100111);
    p3_dbladd(&mut r, 9, &_11110101);
    p3_dbladd(&mut r, 11, &_10111101);
    p3_dbladd(&mut r, 8, &_11100111);
    p3_dbladd(&mut r, 9, &_1101011);
    p3_dbladd(&mut r, 6, &_1011);
    p3_dbladd(&mut r, 14, &_10010011);
    p3_dbladd(&mut r, 10, &_1100011);
    p3_dbladd(&mut r, 9, &_10010111);
    p3_dbladd(&mut r, 10, &_11110101);
    p3_dbladd(&mut r, 8, &_11010011);
    p3_dbladd(&mut r, 8, &_11101101);
    r
}

pub fn is_on_curve(p: &P3) -> i32 {
    let x2 = fe_sq(p.x);
    let y2 = fe_sq(p.y);
    let z2 = fe_sq(p.z);
    let mut t0 = fe_sub(y2, x2);
    t0 = fe_mul(t0, z2);

    let mut t1 = fe_mul(x2, y2);
    t1 = fe_mul(t1, ED25519_D);
    let z4 = fe_sq(z2);
    t1 = fe_add(t1, z4);
    t0 = fe_sub(t0, t1);

    fe_iszero(&t0)
}

pub fn is_on_main_subgroup(p: &P3) -> i32 {
    let pl = mul_l(p);
    let t = fe_sub(pl.y, pl.z);
    fe_iszero(&pl.x) & fe_iszero(&t)
}

pub fn is_canonical(s: &[u8]) -> i32 {
    let mut c = (s[31] & 0x7f) ^ 0x7f;
    let mut i = 30;
    while i > 0 {
        c |= s[i] ^ 0xff;
        i -= 1;
    }
    let c = ((c as u32).wrapping_sub(1)) >> 8;
    let d = (0xedu32.wrapping_sub(1).wrapping_sub(s[0] as u32)) >> 8;
    1 - (c & d & 1) as i32
}

pub fn has_small_order(p: &P3) -> i32 {
    let mut ret = 0;
    ret |= fe_iszero(&p.x);
    ret |= fe_iszero(&p.y);
    ret |= fe_iszero(&p.z);
    let y_sqrtm1 = fe_mul(p.y, FE25519_SQRTM1);
    let c = fe_sub(y_sqrtm1, p.x);
    ret |= fe_iszero(&c);
    let c = fe_add(y_sqrtm1, p.x);
    ret |= fe_iszero(&c);
    ret
}

/* montgomery to edwards */
fn mont_to_ed(x: &Fe, y: &Fe) -> (Fe, Fe) {
    let one = fe_1();
    let x_plus_one = fe_add(*x, one);
    let x_minus_one = fe_sub(*x, one);

    let mut x_plus_one_y_inv = fe_mul(x_plus_one, *y);
    x_plus_one_y_inv = fe_invert(x_plus_one_y_inv);
    let mut xed = fe_mul(*x, ED25519_SQRTAM2);
    xed = fe_mul(xed, x_plus_one_y_inv);
    xed = fe_mul(xed, x_plus_one);

    let mut yed = fe_mul(x_plus_one_y_inv, *y);
    yed = fe_mul(yed, x_minus_one);
    fe_cmov(&mut yed, &one, fe_iszero(&x_plus_one_y_inv) as u32);
    (xed, yed)
}

fn xmont_to_ymont(x: &Fe) -> (Fe, i32) {
    let x2 = fe_sq(*x);
    let x3 = fe_mul(*x, x2);
    let x2m = fe_mul32(x2, ED25519_A_32);
    let mut y = fe_add(x3, *x);
    y = fe_add(y, x2m);
    fe_sqrt(y)
}

pub fn clear_cofactor(p3: &mut P3) {
    let mut p1 = p3_dbl(p3);
    let mut p2 = p1p1_to_p2(&p1);
    p1 = p2_dbl(&p2);
    p2 = p1p1_to_p2(&p1);
    p1 = p2_dbl(&p2);
    *p3 = p1p1_to_p3(&p1);
}

fn elligator2(r: &Fe) -> (Fe, Fe, i32) {
    let mut rr2 = fe_sq2(*r);
    rr2[0] = rr2[0].wrapping_add(1);
    rr2 = fe_invert(rr2);
    let mut x = fe_mul32(rr2, ED25519_A_32);
    x = fe_neg(x);

    let mut x2 = fe_sq(x);
    let x3 = fe_mul(x, x2);
    x2 = fe_mul32(x2, ED25519_A_32);
    let mut gx1 = fe_add(x3, x);
    gx1 = fe_add(gx1, x2);

    let notsquare = fe_notsquare(gx1);

    let negx = fe_neg(x);
    fe_cmov(&mut x, &negx, notsquare as u32);
    x2 = fe_0();
    fe_cmov(&mut x2, &ED25519_A, notsquare as u32);
    x = fe_sub(x, x2);

    let (y, ok) = xmont_to_ymont(&x);
    if ok != 0 {
        panic!("abort"); /* matches abort() */
    }
    (x, y, notsquare)
}

pub fn from_uniform(r: &[u8]) -> [u8; 32] {
    let mut s = [0u8; 32];
    s.copy_from_slice(&r[0..32]);
    let x_sign = ((s[31] >> 5) ^ OPTBLOCKER) >> 2;
    s[31] &= 0x7f;
    let r_fe = fe_frombytes(&s);

    let (x, y, _notsquare) = elligator2(&r_fe);

    let mut p3 = P3::new();
    let (px, py) = mont_to_ed(&x, &y);
    p3.x = px;
    p3.y = py;
    let negxed = fe_neg(p3.x);
    let cond = (fe_isnegative(&p3.x) ^ x_sign as i32) as u32;
    fe_cmov(&mut p3.x, &negxed, cond);

    p3.z = fe_1();
    p3.t = fe_mul(p3.x, p3.y);
    clear_cofactor(&mut p3);
    p3_tobytes(&p3)
}

pub fn from_hash(h: &[u8; 64]) -> [u8; 32] {
    let fe_f = fe_reduce64(h, OPTBLOCKER);
    let (x, y, notsquare) = elligator2(&fe_f);

    let y_sign = notsquare ^ 1;
    let mut y = y;
    let negy = fe_neg(y);
    let cond = (fe_isnegative(&y) ^ y_sign) as u32;
    fe_cmov(&mut y, &negy, cond);

    let mut p3 = P3::new();
    let (px, py) = mont_to_ed(&x, &y);
    p3.x = px;
    p3.y = py;

    p3.z = fe_1();
    p3.t = fe_mul(p3.x, p3.y);
    clear_cofactor(&mut p3);
    p3_tobytes(&p3)
}

/* ---- FFI helpers: C ge25519 structs are fe25519[10] arrays sequentially ----
   Layout: p2 = X,Y,Z (30 i32); p3/p1p1/cached = 40 i32; precomp = 30 i32. */

impl P3 {
    pub unsafe fn read(ptr: *const i32) -> P3 {
        let mut a = [0i32; 40];
        core::ptr::copy_nonoverlapping(ptr, a.as_mut_ptr(), 40);
        P3 {
            x: a[0..10].try_into().unwrap(),
            y: a[10..20].try_into().unwrap(),
            z: a[20..30].try_into().unwrap(),
            t: a[30..40].try_into().unwrap(),
        }
    }
    pub unsafe fn write(&self, ptr: *mut i32) {
        let mut a = [0i32; 40];
        a[0..10].copy_from_slice(&self.x);
        a[10..20].copy_from_slice(&self.y);
        a[20..30].copy_from_slice(&self.z);
        a[30..40].copy_from_slice(&self.t);
        core::ptr::copy_nonoverlapping(a.as_ptr(), ptr, 40);
    }
}
impl P2 {
    pub unsafe fn read(ptr: *const i32) -> P2 {
        let mut a = [0i32; 30];
        core::ptr::copy_nonoverlapping(ptr, a.as_mut_ptr(), 30);
        P2 {
            x: a[0..10].try_into().unwrap(),
            y: a[10..20].try_into().unwrap(),
            z: a[20..30].try_into().unwrap(),
        }
    }
    pub unsafe fn write(&self, ptr: *mut i32) {
        let mut a = [0i32; 30];
        a[0..10].copy_from_slice(&self.x);
        a[10..20].copy_from_slice(&self.y);
        a[20..30].copy_from_slice(&self.z);
        core::ptr::copy_nonoverlapping(a.as_ptr(), ptr, 30);
    }
}
impl P1p1 {
    pub unsafe fn read(ptr: *const i32) -> P1p1 {
        let mut a = [0i32; 40];
        core::ptr::copy_nonoverlapping(ptr, a.as_mut_ptr(), 40);
        P1p1 {
            x: a[0..10].try_into().unwrap(),
            y: a[10..20].try_into().unwrap(),
            z: a[20..30].try_into().unwrap(),
            t: a[30..40].try_into().unwrap(),
        }
    }
    pub unsafe fn write(&self, ptr: *mut i32) {
        let mut a = [0i32; 40];
        a[0..10].copy_from_slice(&self.x);
        a[10..20].copy_from_slice(&self.y);
        a[20..30].copy_from_slice(&self.z);
        a[30..40].copy_from_slice(&self.t);
        core::ptr::copy_nonoverlapping(a.as_ptr(), ptr, 40);
    }
}

/* ---- exported C-ABI symbols ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_tobytes(s: *mut u8, h: *const i32) {
    let p = P2::read(h);
    let out = tobytes_p2(&p);
    core::ptr::copy_nonoverlapping(out.as_ptr(), s, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const i32) {
    let p = P3::read(h);
    let out = p3_tobytes(&p);
    core::ptr::copy_nonoverlapping(out.as_ptr(), s, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes(h: *mut i32, s: *const u8) -> i32 {
    let sl = core::slice::from_raw_parts(s, 32);
    let (p, ret) = frombytes(sl);
    p.write(h);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_frombytes_negate_vartime(
    h: *mut i32,
    s: *const u8,
) -> i32 {
    let sl = core::slice::from_raw_parts(s, 32);
    let (p, ret) = frombytes_negate_vartime(sl);
    if ret != 0 {
        // ge25519_frombytes_negate_vartime() takes an early `return -1` BEFORE
        // `fe25519_mul(h->T, h->X, h->Y)`, so on the failure path the C leaves
        // h->T exactly as the caller had it and only X, Y and Z are stored.
        core::ptr::copy_nonoverlapping(p.x.as_ptr(), h, 10);
        core::ptr::copy_nonoverlapping(p.y.as_ptr(), h.add(10), 10);
        core::ptr::copy_nonoverlapping(p.z.as_ptr(), h.add(20), 10);
        return ret;
    }
    p.write(h);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p2(r: *mut i32, p: *const i32) {
    let pp = P1p1::read(p);
    p1p1_to_p2(&pp).write(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p1p1_to_p3(r: *mut i32, p: *const i32) {
    let pp = P1p1::read(p);
    p1p1_to_p3(&pp).write(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p2_to_p3(r: *mut i32, p: *const i32) {
    let pp = P2::read(p);
    p2_to_p3(&pp).write(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_add(r: *mut i32, p: *const i32, q: *const i32) {
    let pp = P3::read(p);
    let qq = P3::read(q);
    p3_add(&pp, &qq).write(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_p3_sub(r: *mut i32, p: *const i32, q: *const i32) {
    let pp = P3::read(p);
    let qq = P3::read(q);
    p3_sub(&pp, &qq).write(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult_base(h: *mut i32, a: *const u8) {
    let a = core::slice::from_raw_parts(a, 32);
    scalarmult_base(a).write(h);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_scalarmult(h: *mut i32, a: *const u8, p: *const i32) {
    let a = core::slice::from_raw_parts(a, 32);
    let pp = P3::read(p);
    scalarmult(a, &pp).write(h);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_double_scalarmult_vartime(
    r: *mut i32,
    a: *const u8,
    big_a: *const i32,
    b: *const u8,
) {
    let a = core::slice::from_raw_parts(a, 32);
    let b = core::slice::from_raw_parts(b, 32);
    let ap = P3::read(big_a);
    double_scalarmult_vartime(a, &ap, b).write(r);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_clear_cofactor(p3: *mut i32) {
    let mut p = P3::read(p3);
    clear_cofactor(&mut p);
    p.write(p3);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_canonical(s: *const u8) -> i32 {
    let s = core::slice::from_raw_parts(s, 32);
    is_canonical(s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_curve(p: *const i32) -> i32 {
    is_on_curve(&P3::read(p))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_is_on_main_subgroup(p: *const i32) -> i32 {
    is_on_main_subgroup(&P3::read(p))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_has_small_order(p: *const i32) -> i32 {
    has_small_order(&P3::read(p))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_uniform(s: *mut u8, r: *const u8) {
    let r = core::slice::from_raw_parts(r, 32);
    let out = from_uniform(r);
    core::ptr::copy_nonoverlapping(out.as_ptr(), s, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ge25519_from_hash(s: *mut u8, h: *const u8) {
    let hs = core::slice::from_raw_parts(h, 64);
    let mut hh = [0u8; 64];
    hh.copy_from_slice(hs);
    let out = from_hash(&hh);
    core::ptr::copy_nonoverlapping(out.as_ptr(), s, 32);
}
