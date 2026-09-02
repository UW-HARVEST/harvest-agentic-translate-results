//! Translation of c_src/libsodium/crypto_scalarmult/curve25519/ref10/x25519_ref10.c

use core::ffi::{c_int, c_void};

use crate::fe25519::{
    fe25519, fe25519_0, fe25519_1, fe25519_add, fe25519_copy, fe25519_cswap, fe25519_mul,
    fe25519_mul32, fe25519_sq, fe25519_sub, ge25519_p3,
};

extern "C" {
    fn _sodium_fe25519_frombytes(h: *mut i32, s: *const u8);
    fn _sodium_fe25519_invert(out: *mut i32, z: *const i32);
    fn _sodium_fe25519_tobytes(s: *mut u8, h: *const i32);
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

// Local repr(C) mirror of crypto_scalarmult_curve25519_implementation.
#[repr(C)]
pub struct crypto_scalarmult_curve25519_implementation {
    pub mult: extern "C" fn(q: *mut u8, n: *const u8, p: *const u8) -> c_int,
    pub mult_base: extern "C" fn(q: *mut u8, n: *const u8) -> c_int,
}

unsafe impl Sync for crypto_scalarmult_curve25519_implementation {}

// Reject small order points early. See https://eprint.iacr.org/2017/806.pdf .
unsafe fn has_small_order(s: *const u8) -> c_int {
    // CRYPTO_ALIGN(16) has no bearing on behaviour.
    static BLOCKLIST: [[u8; 32]; 7] = [
        // 0 (order 4)
        [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ],
        // 1 (order 1)
        [
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ],
        // 325606250916557431795983626356110631294008115727848805560023387167927233504 (order 8)
        [
            0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f,
            0xc4, 0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16,
            0x5f, 0x49, 0xb8, 0x00,
        ],
        // 39382357235489614581723060781553021112529911719440698176882885853963445705823 (order 8)
        [
            0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83,
            0xef, 0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd,
            0xd0, 0x9f, 0x11, 0x57,
        ],
        // p-1 (order 2)
        [
            0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        // p (=0, order 4)
        [
            0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
        // p+1 (=1, order 1)
        [
            0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0x7f,
        ],
    ];
    let mut c: [u8; 7] = [0; 7];
    let mut k: u32;
    let mut i: usize;
    let mut j: usize;

    // COMPILER_ASSERT(7 == sizeof blocklist / sizeof blocklist[0]);
    j = 0;
    while j < 31 {
        i = 0;
        while i < 7 {
            c[i] |= *s.add(j) ^ BLOCKLIST[i][j];
            i += 1;
        }
        j += 1;
    }
    // j == 31 here (as in C, referenced after the loop).
    i = 0;
    while i < 7 {
        c[i] |= (*s.add(j) & 0x7f) ^ BLOCKLIST[i][j];
        i += 1;
    }
    k = 0;
    i = 0;
    while i < 7 {
        k |= (c[i] as u32).wrapping_sub(1);
        i += 1;
    }
    ((k >> 8) & 1) as c_int
}

// HAVE_TI_MODE undefined: fe25519_sub_lazy is a plain per-limb subtraction.
#[inline]
unsafe fn fe25519_sub_lazy(h: *mut i32, f: *const i32, g: *const i32) {
    fe25519_sub(h, f, g);
}

extern "C" fn crypto_scalarmult_curve25519_ref10(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    unsafe {
        let mut t: [u8; 32] = [0; 32];
        let mut i: u32;
        let mut x1: fe25519 = [0; 10];
        let mut x2: fe25519 = [0; 10];
        let mut x3: fe25519 = [0; 10];
        let mut z2: fe25519 = [0; 10];
        let mut z3: fe25519 = [0; 10];
        let mut a: fe25519 = [0; 10];
        let mut b: fe25519 = [0; 10];
        let mut aa: fe25519 = [0; 10];
        let mut bb: fe25519 = [0; 10];
        let mut e: fe25519 = [0; 10];
        let mut da: fe25519 = [0; 10];
        let mut cb: fe25519 = [0; 10];
        let mut pos: c_int;
        let mut swap: u32;
        let mut bit: u32;

        if has_small_order(p) != 0 {
            return -1;
        }
        i = 0;
        while i < 32 {
            t[i as usize] = *n.add(i as usize);
            i += 1;
        }
        t[0] &= 248;
        t[31] &= 127;
        t[31] |= 64;
        _sodium_fe25519_frombytes(x1.as_mut_ptr(), p);
        fe25519_1(x2.as_mut_ptr());
        fe25519_0(z2.as_mut_ptr());
        fe25519_copy(x3.as_mut_ptr(), x1.as_ptr());
        fe25519_1(z3.as_mut_ptr());

        swap = 0;
        pos = 254;
        while pos >= 0 {
            bit = (t[(pos / 8) as usize] >> (pos & 7)) as u32;
            bit &= 1;
            swap ^= bit;
            fe25519_cswap(x2.as_mut_ptr(), x3.as_mut_ptr(), swap);
            fe25519_cswap(z2.as_mut_ptr(), z3.as_mut_ptr(), swap);
            swap = bit;
            fe25519_add(a.as_mut_ptr(), x2.as_ptr(), z2.as_ptr());
            fe25519_sub_lazy(b.as_mut_ptr(), x2.as_ptr(), z2.as_ptr());
            fe25519_sq(aa.as_mut_ptr(), a.as_ptr());
            fe25519_sq(bb.as_mut_ptr(), b.as_ptr());
            fe25519_mul(x2.as_mut_ptr(), aa.as_ptr(), bb.as_ptr());
            fe25519_sub_lazy(e.as_mut_ptr(), aa.as_ptr(), bb.as_ptr());
            fe25519_sub_lazy(da.as_mut_ptr(), x3.as_ptr(), z3.as_ptr());
            fe25519_mul(da.as_mut_ptr(), da.as_ptr(), a.as_ptr());
            fe25519_add(cb.as_mut_ptr(), x3.as_ptr(), z3.as_ptr());
            fe25519_mul(cb.as_mut_ptr(), cb.as_ptr(), b.as_ptr());
            fe25519_add(x3.as_mut_ptr(), da.as_ptr(), cb.as_ptr());
            fe25519_sq(x3.as_mut_ptr(), x3.as_ptr());
            fe25519_sub_lazy(z3.as_mut_ptr(), da.as_ptr(), cb.as_ptr());
            fe25519_sq(z3.as_mut_ptr(), z3.as_ptr());
            fe25519_mul(z3.as_mut_ptr(), z3.as_ptr(), x1.as_ptr());
            fe25519_mul32(z2.as_mut_ptr(), e.as_ptr(), 121666);
            fe25519_add(z2.as_mut_ptr(), z2.as_ptr(), bb.as_ptr());
            fe25519_mul(z2.as_mut_ptr(), z2.as_ptr(), e.as_ptr());
            pos -= 1;
        }
        fe25519_cswap(x2.as_mut_ptr(), x3.as_mut_ptr(), swap);
        fe25519_cswap(z2.as_mut_ptr(), z3.as_mut_ptr(), swap);

        _sodium_fe25519_invert(z2.as_mut_ptr(), z2.as_ptr());
        fe25519_mul(x2.as_mut_ptr(), x2.as_ptr(), z2.as_ptr());
        _sodium_fe25519_tobytes(q, x2.as_ptr());

        sodium_memzero(t.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u8; 32]>());

        0
    }
}

unsafe fn edwards_to_montgomery(
    montgomery_x: *mut i32,
    edwards_y: *const i32,
    edwards_z: *const i32,
) {
    let mut temp_x: fe25519 = [0; 10];
    let mut temp_z: fe25519 = [0; 10];

    fe25519_add(temp_x.as_mut_ptr(), edwards_z, edwards_y);
    fe25519_sub(temp_z.as_mut_ptr(), edwards_z, edwards_y);
    _sodium_fe25519_invert(temp_z.as_mut_ptr(), temp_z.as_ptr());
    fe25519_mul(montgomery_x, temp_x.as_ptr(), temp_z.as_ptr());
}

extern "C" fn crypto_scalarmult_curve25519_ref10_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    unsafe {
        let t: *mut u8 = q;
        let mut a = core::mem::MaybeUninit::<ge25519_p3>::uninit();
        let mut pk: fe25519 = [0; 10];
        let mut i: u32;

        i = 0;
        while i < 32 {
            *t.add(i as usize) = *n.add(i as usize);
            i += 1;
        }
        *t.add(0) &= 248;
        *t.add(31) &= 127;
        *t.add(31) |= 64;
        _sodium_ge25519_scalarmult_base(a.as_mut_ptr(), t);
        edwards_to_montgomery(
            pk.as_mut_ptr(),
            core::ptr::addr_of!((*a.as_ptr()).Y) as *const i32,
            core::ptr::addr_of!((*a.as_ptr()).Z) as *const i32,
        );
        _sodium_fe25519_tobytes(q, pk.as_ptr());

        0
    }
}

#[unsafe(no_mangle)]
pub static crypto_scalarmult_curve25519_ref10_implementation:
    crypto_scalarmult_curve25519_implementation =
    crypto_scalarmult_curve25519_implementation {
        mult: crypto_scalarmult_curve25519_ref10,
        mult_base: crypto_scalarmult_curve25519_ref10_base,
    };
