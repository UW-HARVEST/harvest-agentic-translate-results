//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) that
//! provides the `stb_ds` SipHash-based byte hashing routine plus a small
//! table-printing helper. The exported ABI is:
//!
//!   * `size_t stbds_hash_bytes(void *p, size_t len, size_t seed);`
//!   * `void   siphash(int init);`
//!
//! `stbds_siphash_bytes` is `static` in the C source and therefore not part of
//! the public ABI; it is kept private here as well.
//!
//! The original C relies on several implementation-defined / undefined
//! behaviours (signed `int` shift overflow followed by sign-extension into
//! `size_t`). Those are reproduced bit-for-bit rather than "fixed", because the
//! translation must be output-identical to the C build.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Number of bits in a `size_t` on the supported targets (`sizeof(size_t) * 8`).
const SIZE_T_BITS: u32 = 64;

/// One SipHash round, matching the `stbds_sipround` macro expansion that the C
/// source spells out inline.
///
/// `(x << n) | (x >> (SIZE_T_BITS - n))` on an unsigned value is a rotate-left.
macro_rules! sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = $v1.rotate_left(13);
        $v1 ^= $v0;
        $v0 = $v0.rotate_left(SIZE_T_BITS / 2);
        $v2 = $v2.wrapping_add($v3);
        $v3 = $v3.rotate_left(16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = $v1.rotate_left(17);
        $v1 ^= $v2;
        $v2 = $v2.rotate_left(SIZE_T_BITS / 2);
        $v0 = $v0.wrapping_add($v3);
        $v3 = $v3.rotate_left(21);
        $v3 ^= $v0;
    }};
}

/// Reproduces C's `d[k] << shift` where `d[k]` is an `unsigned char` promoted to
/// `int`, the shift may overflow the `int`, and the resulting (possibly
/// negative) `int` is then converted to `size_t` — sign-extending into the upper
/// 32 bits.
#[inline]
fn int_shift_sext(byte: u8, shift: u32) -> usize {
    (((byte as u32) << shift) as i32) as i64 as u64 as usize
}

/// `static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)`
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;

    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;

    v0 = ((((0x736f_6d65usize) << 16) << 16).wrapping_add(0x7073_6575)) ^ seed;
    v1 = ((((0x646f_7261usize) << 16) << 16).wrapping_add(0x6e64_6f6d)) ^ !seed;
    v2 = ((((0x6c79_6765usize) << 16) << 16).wrapping_add(0x6e65_7261)) ^ seed;
    v3 = ((((0x7465_6462usize) << 16) << 16).wrapping_add(0x7974_6573)) ^ !seed;

    v0 ^= 0x0706_0504_0302_0100usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908usize ^ !seed;

    // for (i = 0; i + sizeof(size_t) <= len; i += sizeof(size_t), d += sizeof(size_t))
    i = 0;
    while i + core::mem::size_of::<usize>() <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        //
        // The right-hand side is an `int`; when d[3] >= 0x80 it becomes negative
        // and is sign-extended when stored into the `size_t`. Preserved as-is.
        let lo = (*d.add(0) as u32)
            | ((*d.add(1) as u32) << 8)
            | ((*d.add(2) as u32) << 16)
            | ((*d.add(3) as u32) << 24);
        data = (lo as i32) as i64 as u64 as usize;

        // data |= (size_t)(d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi = (*d.add(4) as u32)
            | ((*d.add(5) as u32) << 8)
            | ((*d.add(6) as u32) << 16)
            | ((*d.add(7) as u32) << 24);
        let hi_sext = (hi as i32) as i64 as u64 as usize;
        data |= (hi_sext << 16) << 16;

        v3 ^= data;
        for _j in 0..2 {
            sipround!(v0, v1, v2, v3);
        }
        v0 ^= data;

        i += core::mem::size_of::<usize>();
        d = d.add(core::mem::size_of::<usize>());
    }

    // data = len << (sizeof(size_t) * 8 - 8);
    data = len << (SIZE_T_BITS - 8);

    // switch (len - i) { case 7: ... case 0: break; }  -- fall-through chain.
    let rem = len.wrapping_sub(i);
    if rem >= 7 {
        // data |= ((size_t)d[6] << 24) << 24;
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if rem >= 6 {
        // data |= ((size_t)d[5] << 20) << 20;
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if rem >= 5 {
        // data |= ((size_t)d[4] << 16) << 16;
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if rem >= 4 {
        // data |= (d[3] << 24);   <-- `int` shift, sign-extended on conversion.
        data |= int_shift_sext(*d.add(3), 24);
    }
    if rem >= 3 {
        // data |= (d[2] << 16);
        data |= int_shift_sext(*d.add(2), 16);
    }
    if rem >= 2 {
        // data |= (d[1] << 8);
        data |= int_shift_sext(*d.add(1), 8);
    }
    if rem >= 1 {
        // data |= d[0];
        data |= *d.add(0) as usize;
    }

    v3 ^= data;
    for _j in 0..2 {
        sipround!(v0, v1, v2, v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _j in 0..4 {
        sipround!(v0, v1, v2, v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

/// `void siphash(int init)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    let mut mem: [u8; 64] = [0; 64];
    let mut z: c_int = init;

    // for (i=0; i < 64; ++i,z++) mem[i] = z;
    for i in 0..64usize {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }

    for i in 0..64usize {
        let hash = stbds_hash_bytes(mem.as_mut_ptr() as *mut c_void, i, 0);
        printf(b"  { \0".as_ptr() as *const c_char);
        for j in 0..8usize {
            let byte = ((hash >> (j * 8)) & 255) as u8;
            printf(b"0x%02x, \0".as_ptr() as *const c_char, byte as c_int);
        }
        printf(b" },\n\0".as_ptr() as *const c_char);
    }
}
