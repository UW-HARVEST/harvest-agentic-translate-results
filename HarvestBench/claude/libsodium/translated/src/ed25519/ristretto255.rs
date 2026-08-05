//! Ristretto group, translated from ed25519_ref10.c (ristretto255_*).
use crate::ed25519::fe25519::*;
use crate::ed25519::ge25519::{p3_add, P3, OPTBLOCKER};

fn sqrt_ratio_m1(u: Fe, v: Fe) -> (Fe, i32) {
    let mut v3 = fe_sq(v);
    v3 = fe_mul(v3, v); /* v3 = v^3 */
    let mut x = fe_sq(v3);
    x = fe_mul(x, u);
    x = fe_mul(x, v); /* x = uv^7 */

    x = fe_pow22523(x);
    x = fe_mul(x, v3);
    x = fe_mul(x, u); /* x = uv^3(uv^7)^((q-5)/8) */

    let mut vxx = fe_sq(x);
    vxx = fe_mul(vxx, v); /* vx^2 */
    let m_root_check = fe_sub(vxx, u);
    let p_root_check = fe_add(vxx, u);
    let mut f_root_check = fe_mul(u, FE25519_SQRTM1);
    f_root_check = fe_add(vxx, f_root_check);
    let has_m_root = fe_iszero(&m_root_check);
    let has_p_root = fe_iszero(&p_root_check);
    let has_f_root = fe_iszero(&f_root_check);
    let x_sqrtm1 = fe_mul(x, FE25519_SQRTM1);

    fe_cmov(&mut x, &x_sqrtm1, (has_p_root | has_f_root) as u32);
    fe_abs(&mut x);

    (x, has_m_root | has_p_root)
}

fn is_canonical(s: &[u8]) -> i32 {
    let mut c = (s[31] & 0x7f) ^ 0x7f;
    let mut i = 30;
    while i > 0 {
        c |= s[i] ^ 0xff;
        i -= 1;
    }
    let c = ((c as u32).wrapping_sub(1)) >> 8;
    let d = (0xedu32.wrapping_sub(1).wrapping_sub(s[0] as u32)) >> 8;
    let e = ((s[31] >> 5) ^ OPTBLOCKER) >> 2;
    1 - (((c & d) | e as u32 | s[0] as u32) & 1) as i32
}

pub fn frombytes(s: &[u8]) -> (P3, i32) {
    let mut h = P3::new();
    if is_canonical(s) == 0 {
        return (h, -1);
    }
    let s_ = fe_frombytes(s);
    let ss = fe_sq(s_); /* ss = s^2 */

    let mut u1 = fe_1();
    u1 = fe_sub(u1, ss); /* u1 = 1-ss */
    let u1u1 = fe_sq(u1);

    let mut u2 = fe_1();
    u2 = fe_add(u2, ss); /* u2 = 1+ss */
    let u2u2 = fe_sq(u2);

    let mut v = fe_mul(ED25519_D, u1u1);
    v = fe_neg(v);
    v = fe_sub(v, u2u2);

    let v_u2u2 = fe_mul(v, u2u2);

    let one = fe_1();
    let (inv_sqrt, notsquare) = sqrt_ratio_m1(one, v_u2u2);
    h.x = fe_mul(inv_sqrt, u2);
    h.y = fe_mul(inv_sqrt, h.x);
    h.y = fe_mul(h.y, v);

    h.x = fe_mul(h.x, s_);
    h.x = fe_add(h.x, h.x);
    fe_abs(&mut h.x);
    h.y = fe_mul(u1, h.y);
    h.z = fe_1();
    h.t = fe_mul(h.x, h.y);

    let ret = -(((1 - notsquare) | fe_isnegative(&h.t) | fe_iszero(&h.y)));
    (h, ret)
}

pub fn p3_tobytes(h: &P3) -> [u8; 32] {
    let u1 = fe_add(h.z, h.y); /* u1 = Z+Y */
    let zmy = fe_sub(h.z, h.y); /* zmy = Z-Y */
    let u1 = fe_mul(u1, zmy);
    let u2 = fe_mul(h.x, h.y);

    let mut u1_u2u2 = fe_sq(u2);
    u1_u2u2 = fe_mul(u1, u1_u2u2);

    let one = fe_1();
    let (inv_sqrt, _) = sqrt_ratio_m1(one, u1_u2u2);
    let den1 = fe_mul(inv_sqrt, u1);
    let den2 = fe_mul(inv_sqrt, u2);
    let mut z_inv = fe_mul(den1, den2);
    z_inv = fe_mul(z_inv, h.t);

    let ix = fe_mul(h.x, FE25519_SQRTM1);
    let iy = fe_mul(h.y, FE25519_SQRTM1);
    let eden = fe_mul(den1, ED25519_INVSQRTAMD);

    let t_z_inv = fe_mul(h.t, z_inv);
    let rotate = fe_isnegative(&t_z_inv);

    let mut x_ = h.x;
    let mut y_ = h.y;
    let mut den_inv = den2;

    fe_cmov(&mut x_, &iy, rotate as u32);
    fe_cmov(&mut y_, &ix, rotate as u32);
    fe_cmov(&mut den_inv, &eden, rotate as u32);

    let x_z_inv = fe_mul(x_, z_inv);
    fe_cneg(&mut y_, fe_isnegative(&x_z_inv) as u32);

    let mut s_ = fe_sub(h.z, y_);
    s_ = fe_mul(den_inv, s_);
    fe_abs(&mut s_);
    fe_tobytes(s_)
}

fn elligator(t: &Fe) -> P3 {
    let one = fe_1();
    let mut r = fe_sq(*t);
    r = fe_mul(FE25519_SQRTM1, r);
    let mut u = fe_add(r, one);
    u = fe_mul(u, ED25519_ONEMSQD);
    let mut c = fe_1();
    c = fe_neg(c);
    let rpd = fe_add(r, ED25519_D);
    let mut v = fe_mul(r, ED25519_D);
    v = fe_sub(c, v);
    v = fe_mul(v, rpd);

    let (mut s, sq) = sqrt_ratio_m1(u, v);
    let wasnt_square = 1 - sq;
    let mut s_prime = fe_mul(s, *t);
    fe_abs(&mut s_prime);
    s_prime = fe_neg(s_prime);
    fe_cmov(&mut s, &s_prime, wasnt_square as u32);
    fe_cmov(&mut c, &r, wasnt_square as u32);

    let mut n = fe_sub(r, one);
    n = fe_mul(n, c);
    n = fe_mul(n, ED25519_SQDMONE);
    n = fe_sub(n, v);

    let mut w0 = fe_add(s, s);
    w0 = fe_mul(w0, v);
    let w1 = fe_mul(n, ED25519_SQRTADM1);
    let ss = fe_sq(s);
    let w2 = fe_sub(one, ss);
    let w3 = fe_add(one, ss);

    let mut p = P3::new();
    p.x = fe_mul(w0, w3);
    p.y = fe_mul(w2, w1);
    p.z = fe_mul(w1, w3);
    p.t = fe_mul(w0, w2);
    p
}

pub fn from_hash(h: &[u8]) -> [u8; 32] {
    let r0 = fe_frombytes(&h[0..]);
    let r1 = fe_frombytes(&h[32..]);
    let p0 = elligator(&r0);
    let p1 = elligator(&r1);
    let p = p3_add(&p0, &p1);
    p3_tobytes(&p)
}

/* ---- exported C-ABI symbols ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_frombytes(h: *mut i32, s: *const u8) -> i32 {
    let sl = core::slice::from_raw_parts(s, 32);
    let (p, ret) = frombytes(sl);
    p.write(h);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const i32) {
    let p = P3::read(h);
    let out = p3_tobytes(&p);
    core::ptr::copy_nonoverlapping(out.as_ptr(), s, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_ristretto255_from_hash(s: *mut u8, h: *const u8) {
    let hs = core::slice::from_raw_parts(h, 64);
    let out = from_hash(hs);
    core::ptr::copy_nonoverlapping(out.as_ptr(), s, 32);
}
