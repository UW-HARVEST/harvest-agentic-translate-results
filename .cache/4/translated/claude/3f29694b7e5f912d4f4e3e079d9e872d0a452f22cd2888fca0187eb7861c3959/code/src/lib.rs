//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C shared object):
//!   * `siphash`           -- from `include/lib.h`
//!   * `stbds_hash_bytes`  -- non-static definition in `src/lib.c`
//!
//! `stbds_siphash_bytes` is `static` in the C source, so it must NOT be
//! exported; it is a private Rust `fn` here.
//!
//! Fidelity notes -- the C code relies on several subtle integer-promotion
//! behaviours which are faithfully reproduced (they are *bugs* in the original
//! but must not be "fixed"):
//!
//!   * `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)` is evaluated in
//!     `int` arithmetic. When `d[3] >= 0x80` the shift sets bit 31, making the
//!     `int` negative; the subsequent conversion to `size_t` therefore
//!     *sign-extends*, flooding bits 32..63 with ones. The later
//!     `data |= <high word> << 16 << 16` then cannot clear those bits, so the
//!     top half of `data` stays `0xFFFFFFFF`.
//!   * The same sign-extension happens in the `switch` tail for `case 4`
//!     (`data |= (d[3] << 24)`), which is why e.g. lengths 4, 5, 6 and 7 hash
//!     identically for all-`0xFF` input.
//!
//! All arithmetic is therefore modelled with explicit `i32`/`usize` casts and
//! wrapping operations so the result is bit-exact with the C build.

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// libc `printf`, used so the emitted bytes and stream buffering behaviour
    /// are identical to the C library's `printf` calls.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Bit width of `size_t`, i.e. the C expression `(sizeof(size_t) * 8)`.
const SIZE_T_BITS: u32 = usize::BITS;

/// One SipHash round, exactly as the macro expansion in the C source.
///
/// The C code writes the rotations out longhand as
/// `((v << k) | (v >> ((sizeof(size_t) * 8) - k)))`, which for an unsigned
/// type is precisely a left rotation.
#[inline(always)]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(SIZE_T_BITS / 2);

    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;

    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(SIZE_T_BITS / 2);

    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

/// Reproduces `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)` evaluated in
/// C `int` arithmetic, then converted to `size_t` (sign-extending).
#[inline(always)]
fn word_as_signed_int(b0: u8, b1: u8, b2: u8, b3: u8) -> i32 {
    (b0 as i32)
        | ((b1 as i32).wrapping_shl(8))
        | ((b2 as i32).wrapping_shl(16))
        | ((b3 as i32).wrapping_shl(24))
}

/// Translation of the `static` C function `stbds_siphash_bytes`.
///
/// # Safety
/// `p` must be valid for reads of `len` bytes (same contract as the C code).
unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
    let mut j: usize;
    let (mut v0, mut v1, mut v2, mut v3): (usize, usize, usize, usize);
    let mut data: usize;

    v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    const WORD: usize = core::mem::size_of::<usize>();

    i = 0;
    while i.wrapping_add(WORD) <= len {
        // SAFETY: the loop condition guarantees 8 readable bytes at `d`.
        let b = unsafe { core::slice::from_raw_parts(d, WORD) };

        // Low half: computed in `int`, then sign-extended into `size_t`.
        data = word_as_signed_int(b[0], b[1], b[2], b[3]) as usize;
        // High half: also computed in `int` and sign-extended, but the two
        // 16-bit shifts push those extension bits out of the word again.
        data |= ((word_as_signed_int(b[4], b[5], b[6], b[7]) as usize) << 16) << 16;

        v3 ^= data;
        j = 0;
        while j < 2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            j += 1;
        }
        v0 ^= data;

        i = i.wrapping_add(WORD);
        d = unsafe { d.add(WORD) };
    }

    data = len.wrapping_shl(SIZE_T_BITS - 8);

    // The C `switch (len - i)` falls through from the highest matching case
    // down to `case 1`, so an equivalent chain of `>=` tests is used here.
    // `len - i` is always in 0..=7 because the loop above consumed every full
    // `size_t`-sized block.
    let rem = len - i;
    // SAFETY: `rem` bytes remain readable at `d`; each branch only touches
    // indices strictly below `rem`.
    unsafe {
        if rem >= 7 {
            data |= ((*d.add(6) as usize) << 24) << 24;
        }
        if rem >= 6 {
            data |= ((*d.add(5) as usize) << 20) << 20;
        }
        if rem >= 5 {
            data |= ((*d.add(4) as usize) << 16) << 16;
        }
        if rem >= 4 {
            // `int` shift -> sign-extends when d[3] >= 0x80.
            data |= ((*d.add(3) as i32).wrapping_shl(24)) as usize;
        }
        if rem >= 3 {
            data |= ((*d.add(2) as i32).wrapping_shl(16)) as usize;
        }
        if rem >= 2 {
            data |= ((*d.add(1) as i32).wrapping_shl(8)) as usize;
        }
        if rem >= 1 {
            data |= (*d as i32) as usize;
        }
    }

    v3 ^= data;
    j = 0;
    while j < 2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    j = 0;
    while j < 4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }

    v0 ^ v1 ^ v2 ^ v3
}

/// `size_t stbds_hash_bytes(void *p, size_t len, size_t seed);`
///
/// # Safety
/// `p` must be valid for reads of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

/// `void siphash(int init);`
#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];
    let mut z: c_int = init;

    // for (i = 0; i < 64; ++i, z++) mem[i] = z;
    for i in 0..64usize {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }

    for i in 0..64usize {
        let hash = unsafe { stbds_hash_bytes(mem.as_mut_ptr() as *mut c_void, i, 0) };
        unsafe {
            printf(c"  { ".as_ptr());
            for j in 0..8usize {
                printf(
                    c"0x%02x, ".as_ptr(),
                    (((hash >> (j * 8)) & 255) as u8) as c_int,
                );
            }
            printf(c" },\n".as_ptr());
        }
    }
}
