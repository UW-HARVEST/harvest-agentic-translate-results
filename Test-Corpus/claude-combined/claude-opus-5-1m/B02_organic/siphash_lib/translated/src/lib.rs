//! Rust translation of c_src/src/lib.c
//!
//! Faithful (bit-for-bit) translation of the stb_ds.h-style siphash in the C
//! source. Several quirks of the original C are reproduced exactly, including
//! the well-known sign-extension bug when assembling 64-bit `data` words from
//! `unsigned char` arrays via `int` promotion (e.g. `d[3] << 24` becomes
//! negative when `d[3] >= 0x80` and sign-extends into the upper 32 bits).
//!
//! On 64-bit Linux `size_t == uintptr_t == u64`, which the C code assumes; we
//! mirror that here.

use std::ffi::{c_int, c_void};

extern "C" {
    fn printf(format: *const i8, ...) -> i32;
}

#[inline]
fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

/// Faithful translation of `stbds_siphash_bytes`.
///
/// Important: in the original C, the inner expression
/// `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)` has type `int` after
/// integer promotions. When `d[3] >= 0x80`, the resulting `int` is negative;
/// converting it to `size_t` sign-extends, setting the upper 32 bits of the
/// 64-bit `data` value to all-ones. We reproduce that exact behavior by
/// computing the inner expression as `i32` and casting `i32 -> u64` (which is
/// sign-extending in Rust).
fn siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let seed64 = seed as u64;

    let mut v0: u64 = ((0x736f6d65_u64) << 16 << 16).wrapping_add(0x70736575) ^ seed64;
    let mut v1: u64 = ((0x646f7261_u64) << 16 << 16).wrapping_add(0x6e646f6d) ^ !seed64;
    let mut v2: u64 = ((0x6c796765_u64) << 16 << 16).wrapping_add(0x6e657261) ^ seed64;
    let mut v3: u64 = ((0x74656462_u64) << 16 << 16).wrapping_add(0x79746573) ^ !seed64;

    v0 ^= 0x0706050403020100_u64 ^ seed64;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 ^ !seed64;
    v2 ^= 0x0706050403020100_u64 ^ seed64;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 ^ !seed64;

    let word: usize = std::mem::size_of::<usize>(); // 8 on 64-bit

    let mut i: usize = 0;
    while i + word <= len {
        // SAFETY: caller guarantees p[0..len] is a valid readable buffer; we
        // only index within [i, i+word) which the loop guard keeps in bounds.
        let d: &[u8] = unsafe { std::slice::from_raw_parts(p.add(i), 8) };

        // Reproduce the C sign-extension quirk:
        // C: `data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);`
        // The RHS is `int`; when `d[3] >= 0x80` it's negative, and the
        // implicit conversion to `size_t` sign-extends.
        let lo_i32: i32 = (d[0] as i32)
            | (d[1] as i32).wrapping_shl(8)
            | (d[2] as i32).wrapping_shl(16)
            | (d[3] as i32).wrapping_shl(24);
        let mut data: u64 = lo_i32 as u64; // sign-extends i32 -> u64

        // C: `data |= (size_t)(d[4] | (d[5]<<8) | (d[6]<<16) | (d[7]<<24)) << 16 << 16;`
        // The cast happens *before* the shifts, so the sign-extended upper
        // 32 bits get shifted out and only the low 32 bits land in the high
        // half of `data`.
        let hi_i32: i32 = (d[4] as i32)
            | (d[5] as i32).wrapping_shl(8)
            | (d[6] as i32).wrapping_shl(16)
            | (d[7] as i32).wrapping_shl(24);
        let hi: u64 = (hi_i32 as u64).wrapping_shl(16).wrapping_shl(16);
        data |= hi;

        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += word;
    }

    // Tail (the C `switch` block).
    let mut data: u64 = (len as u64) << ((word * 8) - 8); // << 56 on 64-bit

    let rem = len - i;
    // SAFETY: rem == len - i, so [i, i+rem) lies within the original buffer.
    // When rem == 0 we never index into the slice.
    let d: &[u8] = unsafe { std::slice::from_raw_parts(p.add(i), rem) };

    // C switch is fall-through; emulate that with cumulative ifs.
    // case 7: ((size_t)d[6] << 24) << 24  -- pure size_t arithmetic, no sign issue
    if rem >= 7 {
        data |= ((d[6] as u64) << 24) << 24;
    }
    if rem >= 6 {
        data |= ((d[5] as u64) << 20) << 20;
    }
    if rem >= 5 {
        data |= ((d[4] as u64) << 16) << 16;
    }
    if rem >= 4 {
        // C: `data |= (d[3] << 24);` -- `int` shift; sign-extends into size_t
        // when d[3] >= 0x80.
        let v: i32 = (d[3] as i32).wrapping_shl(24);
        data |= v as u64; // sign-extending i32 -> u64
    }
    if rem >= 3 {
        // d[2]<<16 can never set the int sign bit (max 0xFF0000), but use the
        // same pattern for clarity and parity with the C.
        let v: i32 = (d[2] as i32).wrapping_shl(16);
        data |= v as u64;
    }
    if rem >= 2 {
        let v: i32 = (d[1] as i32).wrapping_shl(8);
        data |= v as u64;
    }
    if rem >= 1 {
        data |= d[0] as u64;
    }
    // case 0: nothing.

    v3 ^= data;
    for _ in 0..2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    (v0 ^ v1 ^ v2 ^ v3) as usize
}

/// `size_t stbds_hash_bytes(void *p, size_t len, size_t seed);`
///
/// Public C ABI export.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(p as *const u8, len, seed)
}

/// `void siphash(int init);`
///
/// Public C ABI export. Mirrors the C exactly:
///   * fills a 64-byte buffer where `mem[i] = (unsigned char)(init + i)`
///   * for each length `i` from 0..64, hashes `mem[0..i]` with seed 0
///   * prints the 8 little-endian bytes of the hash to stdout via libc printf
#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    let mut mem: [u8; 64] = [0; 64];
    // C: `int z = init; for (i=0;i<64;++i,z++) mem[i] = z;`
    // Storing an int into unsigned char keeps the low 8 bits; z increments
    // as int (wrapping not actually exercised since 64 fits easily).
    let mut z: c_int = init;
    for i in 0..64usize {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }

    for i in 0..64usize {
        let hash = siphash_bytes(mem.as_ptr(), i, 0);
        // Print "  { " then 8 hex bytes then " },\n", exactly as C printf
        // would. Go through libc printf to guarantee byte-identical output.
        printf(b"  { \0".as_ptr() as *const i8);
        for j in 0..8 {
            let byte = ((hash >> (j * 8)) & 0xff) as u8;
            // C casts the byte back to unsigned char; default argument
            // promotion turns it into int for varargs.
            printf(
                b"0x%02x, \0".as_ptr() as *const i8,
                byte as c_int,
            );
        }
        printf(b" },\n\0".as_ptr() as *const i8);
    }
}
