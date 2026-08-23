//! curve25519 X25519, translated from x25519_ref10.c and scalarmult_curve25519.c.
use crate::ed25519::fe25519::*;
use crate::ed25519::ge25519;
use core::ffi::{c_int, c_void};

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

fn has_small_order(s: &[u8]) -> i32 {
    const BLOCKLIST: [[u8; 32]; 7] = [
        [0; 32],
        [
            0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ],
        [
            0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f,
            0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16,
            0x5f, 0x49, 0xb8, 0x00,
        ],
        [
            0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83,
            0xef, 0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd,
            0xd0, 0x9f, 0x11, 0x57,
        ],
        [
            0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        [
            0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        [
            0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
    ];
    let mut c = [0u8; 7];
    let mut j = 0usize;
    while j < 31 {
        for i in 0..7 {
            c[i] |= s[j] ^ BLOCKLIST[i][j];
        }
        j += 1;
    }
    for i in 0..7 {
        c[i] |= (s[j] & 0x7f) ^ BLOCKLIST[i][j];
    }
    let mut k: u32 = 0;
    for i in 0..7 {
        k |= (c[i] as u32).wrapping_sub(1);
    }
    ((k >> 8) & 1) as i32
}

pub unsafe fn ref10_mult(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    let psl = core::slice::from_raw_parts(p, 32);
    if has_small_order(psl) != 0 {
        return -1;
    }
    let mut t = [0u8; 32];
    for i in 0..32 {
        t[i] = *n.add(i);
    }
    t[0] &= 248;
    t[31] &= 127;
    t[31] |= 64;

    let x1 = fe_frombytes(psl);
    let mut x2 = fe_1();
    let mut z2 = fe_0();
    let mut x3 = x1;
    let mut z3 = fe_1();

    let mut swap: u32 = 0;
    let mut pos: i32 = 254;
    while pos >= 0 {
        let mut bit = (t[(pos / 8) as usize] >> (pos & 7)) as u32;
        bit &= 1;
        swap ^= bit;
        fe_cswap(&mut x2, &mut x3, swap);
        fe_cswap(&mut z2, &mut z3, swap);
        swap = bit;
        let a = fe_add(x2, z2);
        let b = fe_sub(x2, z2);
        let aa = fe_sq(a);
        let bb = fe_sq(b);
        x2 = fe_mul(aa, bb);
        let e = fe_sub(aa, bb);
        let mut da = fe_sub(x3, z3);
        da = fe_mul(da, a);
        let mut cb = fe_add(x3, z3);
        cb = fe_mul(cb, b);
        x3 = fe_add(da, cb);
        x3 = fe_sq(x3);
        z3 = fe_sub(da, cb);
        z3 = fe_sq(z3);
        z3 = fe_mul(z3, x1);
        z2 = fe_mul32(e, 121666);
        z2 = fe_add(z2, bb);
        z2 = fe_mul(z2, e);
        pos -= 1;
    }
    fe_cswap(&mut x2, &mut x3, swap);
    fe_cswap(&mut z2, &mut z3, swap);

    z2 = fe_invert(z2);
    x2 = fe_mul(x2, z2);
    let out = fe_tobytes(x2);
    core::ptr::copy_nonoverlapping(out.as_ptr(), q, 32);

    sodium_memzero(t.as_mut_ptr() as *mut c_void, 32);
    0
}

fn edwards_to_montgomery(edwards_y: Fe, edwards_z: Fe) -> Fe {
    let temp_x = fe_add(edwards_z, edwards_y);
    let mut temp_z = fe_sub(edwards_z, edwards_y);
    temp_z = fe_invert(temp_z);
    fe_mul(temp_x, temp_z)
}

pub unsafe fn ref10_base(q: *mut u8, n: *const u8) -> c_int {
    let mut t = [0u8; 32];
    for i in 0..32 {
        t[i] = *n.add(i);
    }
    t[0] &= 248;
    t[31] &= 127;
    t[31] |= 64;
    let a = ge25519::scalarmult_base(&t);
    let pk = edwards_to_montgomery(a.y, a.z);
    let out = fe_tobytes(pk);
    core::ptr::copy_nonoverlapping(out.as_ptr(), q, 32);
    0
}

/* ---- implementation struct (matches C layout) ---- */

pub type MultFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
pub type MultBaseFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;

#[repr(C)]
pub struct Curve25519Implementation {
    pub mult: MultFn,
    pub mult_base: MultBaseFn,
}
unsafe impl Sync for Curve25519Implementation {}

unsafe extern "C" fn c_ref10_mult(q: *mut u8, n: *const u8, p: *const u8) -> c_int {
    ref10_mult(q, n, p)
}
unsafe extern "C" fn c_ref10_base(q: *mut u8, n: *const u8) -> c_int {
    ref10_base(q, n)
}

#[unsafe(no_mangle)]
pub static crypto_scalarmult_curve25519_ref10_implementation: Curve25519Implementation =
    Curve25519Implementation {
        mult: c_ref10_mult,
        mult_base: c_ref10_base,
    };
