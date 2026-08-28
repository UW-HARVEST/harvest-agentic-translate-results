//! Rust translation of `c_src/src/lib.c` (stb_ds style siphash helper +
//! the `siphash` table-dumping routine).
//!
//! The translation is deliberately literal: it reproduces the exact integer
//! promotions / sign-extensions / signed-shift wrap-around behaviour that the C
//! compiler produces on a 64-bit LP64 target, and it prints through libc
//! `printf` with the identical format strings so that the emitted bytes (and
//! stdout buffering behaviour) match the C library exactly.
//!
//! Public ABI (matches `nm -D` of the C shared object):
//!   * `stbds_hash_bytes`
//!   * `siphash`

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    // Variadic libc printf: used so that the produced bytes and the stdout
    // buffering semantics are byte-for-byte identical to the C library.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// `sizeof(size_t) * 8` on the target the C code was compiled for (LP64).
const SIZE_T_BITS: u32 = (core::mem::size_of::<usize>() as u32) * 8;

/// The `stbds_sipround()` macro from stb_ds, expanded exactly as it appears in
/// the C source (`do { ... } while (0)`).
#[inline(always)]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    // v0 += v1;
    *v0 = v0.wrapping_add(*v1);
    // v1 = STBDS_ROTATE_LEFT(v1, 13);
    *v1 = rotate_left(*v1, 13);
    // v1 ^= v0;
    *v1 ^= *v0;
    // v0 = STBDS_ROTATE_LEFT(v0, STBDS_SIZE_T_BITS/2);
    *v0 = rotate_left(*v0, SIZE_T_BITS / 2);

    // v2 += v3;
    *v2 = v2.wrapping_add(*v3);
    // v3 = STBDS_ROTATE_LEFT(v3, 16);
    *v3 = rotate_left(*v3, 16);
    // v3 ^= v2;
    *v3 ^= *v2;

    // v2 += v1;
    *v2 = v2.wrapping_add(*v1);
    // v1 = STBDS_ROTATE_LEFT(v1, 17);
    *v1 = rotate_left(*v1, 17);
    // v1 ^= v2;
    *v1 ^= *v2;
    // v2 = STBDS_ROTATE_LEFT(v2, STBDS_SIZE_T_BITS/2);
    *v2 = rotate_left(*v2, SIZE_T_BITS / 2);

    // v0 += v3;
    *v0 = v0.wrapping_add(*v3);
    // v3 = STBDS_ROTATE_LEFT(v3, 21);
    *v3 = rotate_left(*v3, 21);
    // v3 ^= v0;
    *v3 ^= *v0;
}

/// `(((val) << (n)) | ((val) >> (STBDS_SIZE_T_BITS - (n))))`
#[inline(always)]
fn rotate_left(val: usize, n: u32) -> usize {
    // Both operands are unsigned in C, so this is a plain rotate.  Written with
    // explicit shifts to mirror the macro (n is always in 1..SIZE_T_BITS here).
    (val << n) | (val >> (SIZE_T_BITS - n))
}

/// static size_t stbds_siphash_bytes(void *p, size_t len, size_t seed)
fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8; // unsigned char *d = (unsigned char *) p;
    let mut i: usize;
    let mut j: usize;
    let (mut v0, mut v1, mut v2, mut v3): (usize, usize, usize, usize);
    let mut data: usize;

    // v0 = ((((size_t) 0x736f6d65 << 16) << 16) + 0x70736575) ^ seed;
    v0 = ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    // v1 = ((((size_t) 0x646f7261 << 16) << 16) + 0x6e646f6d) ^ ~seed;
    v1 = ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    // v2 = ((((size_t) 0x6c796765 << 16) << 16) + 0x6e657261) ^ seed;
    v2 = ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    // v3 = ((((size_t) 0x74656462 << 16) << 16) + 0x79746573) ^ ~seed;
    v3 = ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    // The 0x...ull constants below are `unsigned long long`; on this target they
    // fit in size_t and the xor is done in 64 bits, exactly as in C.
    v0 ^= (0x0706050403020100u64 as usize) ^ seed;
    v1 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;
    v2 ^= (0x0706050403020100u64 as usize) ^ seed;
    v3 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;

    let sz = core::mem::size_of::<usize>();

    i = 0;
    while i + sz <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        //
        // NOTE: the sub-expressions are `int` in C.  `d[3] << 24` overflows
        // `int` whenever d[3] >= 0x80, which (as produced by real compilers)
        // wraps to a negative value; the subsequent conversion to `size_t`
        // therefore sign-extends and sets the whole upper half of `data`.
        // This quirk is part of the observable behaviour and is reproduced.
        let lo: i32 = unsafe {
            (*d.add(0) as i32)
                | ((*d.add(1) as i32) << 8)
                | ((*d.add(2) as i32) << 16)
                | ((*d.add(3) as i32) << 24)
        };
        data = lo as usize; // int -> size_t (sign-extending)

        // data |= (size_t) (d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi: i32 = unsafe {
            (*d.add(4) as i32)
                | ((*d.add(5) as i32) << 8)
                | ((*d.add(6) as i32) << 16)
                | ((*d.add(7) as i32) << 24)
        };
        data |= ((hi as usize) << 16) << 16;

        v3 ^= data;
        j = 0;
        while j < 2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            j += 1;
        }
        v0 ^= data;

        i += sz;
        d = unsafe { d.add(sz) };
    }

    // data = len << (STBDS_SIZE_T_BITS - 8);
    data = len << (SIZE_T_BITS - 8);

    // switch (len - i) { ... } -- note the deliberate C fall-through.
    let rem = len.wrapping_sub(i);
    unsafe {
        if rem == 7 {
            data |= ((*d.add(6) as usize) << 24) << 24;
        }
        if rem >= 6 && rem <= 7 {
            data |= ((*d.add(5) as usize) << 20) << 20;
        }
        if rem >= 5 && rem <= 7 {
            data |= ((*d.add(4) as usize) << 16) << 16;
        }
        if rem >= 4 && rem <= 7 {
            // `d[3] << 24` is again `int` arithmetic -> sign-extends on
            // conversion to size_t when d[3] >= 0x80.
            data |= ((*d.add(3) as i32) << 24) as usize;
        }
        if rem >= 3 && rem <= 7 {
            data |= ((*d.add(2) as i32) << 16) as usize;
        }
        if rem >= 2 && rem <= 7 {
            data |= ((*d.add(1) as i32) << 8) as usize;
        }
        if rem >= 1 && rem <= 7 {
            data |= (*d.add(0) as i32) as usize;
        }
        // case 0: break;
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

/// size_t stbds_hash_bytes(void *p, size_t len, size_t seed)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

/// void siphash(int init)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    let mut mem: [u8; 64] = [0; 64];
    let mut z: c_int = init;

    // for (i=0; i < 64; ++i,z++) mem[i] = z;
    let mut i: c_int = 0;
    while i < 64 {
        mem[i as usize] = z as u8; // int -> unsigned char (truncating)
        i += 1;
        z = z.wrapping_add(1);
    }

    // for (i=0; i < 64; ++i) { ... }
    i = 0;
    while i < 64 {
        let hash: usize =
            unsafe { stbds_hash_bytes(mem.as_mut_ptr() as *mut c_void, i as usize, 0) };
        unsafe {
            c_printf(c"  { ".as_ptr());
        }
        let mut j: c_int = 0;
        while j < 8 {
            let byte = ((hash >> (j * 8)) & 255) as u8;
            unsafe {
                c_printf(c"0x%02x, ".as_ptr(), byte as c_int);
            }
            j += 1;
        }
        unsafe {
            c_printf(c" },\n".as_ptr());
        }
        i += 1;
    }
}
