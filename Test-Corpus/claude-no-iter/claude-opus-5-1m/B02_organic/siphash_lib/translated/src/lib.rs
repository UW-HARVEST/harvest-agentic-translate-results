// Rust translation of c_src/src/lib.c. Designed to produce byte-identical
// output to the original C, including reproducing the C-level signed-shift
// sign-extension behavior in `d[3] << 24` and the case-4 trailing-byte path.

use core::ffi::{c_char, c_int};

// We deliberately use libc's `printf` rather than Rust's print! macros so that
// the output (including stdout buffering and the exact byte sequence produced
// by `%02x` formatting) matches the original C library exactly.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

// SipRound macro from the original C, expanded as a function.
// All arithmetic is on size_t (usize) and wraps on overflow.
#[inline(always)]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    const HALF: u32 = (core::mem::size_of::<usize>() as u32) * 8 / 2;
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(HALF);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(HALF);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

// Equivalent of static stbds_siphash_bytes from the C source.
// We faithfully reproduce the C undefined-ish behavior of `d[3] << 24` (an int
// shift that may set the sign bit, resulting in sign-extension when promoted
// to size_t).
fn stbds_siphash_bytes_impl(p: *const u8, len: usize, seed: usize) -> usize {
    // Initial state matches the C bit patterns. Using shift-add (instead of a
    // single 64-bit literal) preserves the original computation pattern; the
    // resulting numeric values are identical on 64-bit platforms.
    let mut v0: usize = (((0x736f_6d65usize) << 16) << 16).wrapping_add(0x7073_6575) ^ seed;
    let mut v1: usize = (((0x646f_7261usize) << 16) << 16).wrapping_add(0x6e64_6f6d) ^ !seed;
    let mut v2: usize = (((0x6c79_6765usize) << 16) << 16).wrapping_add(0x6e65_7261) ^ seed;
    let mut v3: usize = (((0x7465_6462usize) << 16) << 16).wrapping_add(0x7974_6573) ^ !seed;

    v0 ^= 0x0706_0504_0302_0100u64 as usize ^ seed;
    v1 ^= 0x0f0e_0d0c_0b0a_0908u64 as usize ^ !seed;
    v2 ^= 0x0706_0504_0302_0100u64 as usize ^ seed;
    v3 ^= 0x0f0e_0d0c_0b0a_0908u64 as usize ^ !seed;

    let word_size = core::mem::size_of::<usize>(); // 8 on 64-bit
    let mut i: usize = 0;
    let mut d_off: usize = 0;

    while i + word_size <= len {
        // Load 8 bytes the way the C does it, including the (d[3] << 24)
        // signed-int shift that sign-extends into the upper 32 bits of `data`.
        let d0 = unsafe { *p.add(d_off) } as i32;
        let d1 = unsafe { *p.add(d_off + 1) } as i32;
        let d2 = unsafe { *p.add(d_off + 2) } as i32;
        let d3 = unsafe { *p.add(d_off + 3) } as i32;
        let d4 = unsafe { *p.add(d_off + 4) } as i32;
        let d5 = unsafe { *p.add(d_off + 5) } as i32;
        let d6 = unsafe { *p.add(d_off + 6) } as i32;
        let d7 = unsafe { *p.add(d_off + 7) } as i32;

        // First half: int OR with possibly-negative `d3 << 24`, then assign
        // (sign-extend) to size_t.
        let lo_int: i32 = d0 | (d1 << 8) | (d2 << 16) | d3.wrapping_shl(24);
        // Sign-extend i32 -> isize -> usize (matches `size_t data = <int>;`).
        let mut data: usize = lo_int as isize as usize;

        // Second half: same int OR, cast to size_t (sign-extends), then `<< 16
        // << 16` (= << 32). Any high bits set by sign-extension shift out.
        let hi_int: i32 = d4 | (d5 << 8) | (d6 << 16) | d7.wrapping_shl(24);
        let hi: usize = ((hi_int as isize as usize) << 16) << 16;
        data |= hi;

        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += word_size;
        d_off += word_size;
    }

    // Trailing bytes. C uses fall-through switch on (len - i).
    // `data = len << ((sizeof(size_t)*8) - 8)` => len << 56 on 64-bit.
    let mut data: usize = len.wrapping_shl((word_size as u32) * 8 - 8);
    let remaining = len - i;

    // Fall-through cases. Note case 4's `(d[3] << 24)` is an int shift and
    // sign-extends — reproduced via i32 -> isize -> usize.
    if remaining >= 7 {
        let b = unsafe { *p.add(d_off + 6) } as usize;
        data |= (b << 24) << 24;
    }
    if remaining >= 6 {
        let b = unsafe { *p.add(d_off + 5) } as usize;
        data |= (b << 20) << 20;
    }
    if remaining >= 5 {
        let b = unsafe { *p.add(d_off + 4) } as usize;
        data |= (b << 16) << 16;
    }
    if remaining >= 4 {
        let b = unsafe { *p.add(d_off + 3) } as i32;
        let shifted: i32 = b.wrapping_shl(24); // may be negative when b >= 0x80
        data |= shifted as isize as usize; // sign-extend
    }
    if remaining >= 3 {
        let b = unsafe { *p.add(d_off + 2) } as usize;
        data |= b << 16;
    }
    if remaining >= 2 {
        let b = unsafe { *p.add(d_off + 1) } as usize;
        data |= b << 8;
    }
    if remaining >= 1 {
        let b = unsafe { *p.add(d_off) } as usize;
        data |= b;
    }
    // case 0: no-op

    v3 ^= data;
    for _ in 0..2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

// Exposed as a public C symbol because the C source declares it without
// `static`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut core::ffi::c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes_impl(p as *const u8, len, seed)
}

// `void siphash(int init)` — only declared public function in lib.h.
#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem: [u8; 64] = [0; 64];
    // Reproduce: `int z = init; for (i=0; i<64; ++i,z++) mem[i] = z;`
    // The store `mem[i] = z` truncates `int` to `unsigned char`.
    let mut z: i32 = init;
    for i in 0..64 {
        mem[i] = (z as u32) as u8; // narrow int -> unsigned char
        z = z.wrapping_add(1);
    }

    // For each i in 0..64, hash the first i bytes and print the 8 little-endian
    // bytes of the resulting size_t hash, formatted exactly like the original.
    let line_open = b"  { \0".as_ptr() as *const c_char;
    let byte_fmt = b"0x%02x, \0".as_ptr() as *const c_char;
    let line_close = b" },\n\0".as_ptr() as *const c_char;

    for i in 0..64usize {
        let hash = stbds_siphash_bytes_impl(mem.as_ptr(), i, 0);
        unsafe {
            printf(line_open);
        }
        for j in 0..8u32 {
            let byte_val: c_int = (((hash >> (j * 8)) & 255) as u8) as c_int;
            unsafe {
                printf(byte_fmt, byte_val);
            }
        }
        unsafe {
            printf(line_close);
        }
    }
}
