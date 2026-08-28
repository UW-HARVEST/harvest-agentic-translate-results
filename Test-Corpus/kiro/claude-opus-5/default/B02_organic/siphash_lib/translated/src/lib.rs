//! Rust translation of `c_src/src/lib.c` (stb_ds-style siphash test-vector dumper).
//!
//! Behaviour notes -- these mirror the C exactly and are deliberately *not* "fixed":
//!
//! * `size_t` is the target's pointer-sized unsigned integer (`usize`). All the
//!   `SIPROUND` arithmetic in the C is done on `size_t`, and the rotate widths are
//!   spelled `sizeof(size_t)*8`, so on a 64-bit target every rotate is a 64-bit rotate.
//! * In the C, `d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24)` is evaluated in
//!   `int` because `unsigned char` promotes to `int`. When `d[3] >= 0x80` the `int`
//!   result is negative, and the subsequent implicit conversion to `size_t`
//!   *sign-extends*, flooding the upper 32 bits of `data` with ones. The same happens
//!   for `case 4:` of the tail switch (`data |= (d[3] << 24);`). This is reproduced
//!   below via `i32 as u64`.
//! * The high half of the loop body, `(size_t)(d[4] | ... | (d[7] << 24)) << 16 << 16`,
//!   also sign-extends, but the extension bits are then shifted out by `<< 32`, so it
//!   behaves as an unsigned gather.
//! * Output is emitted through C `printf` so that stdio buffering and any interleaving
//!   with a C caller's own output is byte-for-byte identical.

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Width of `size_t` in bits, as the C spells it: `sizeof(size_t) * 8`.
const SIZE_T_BITS: u32 = (size_of::<usize>() as u32) * 8;

/// One `stbds_SIPROUND()` step from the C macro, on `size_t`-wide values.
#[inline]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, SIZE_T_BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, SIZE_T_BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
}

/// `((x) << (n)) | ((x) >> (SIZE_T_BITS - (n)))`; every `n` used by the C is in
/// `1 ..= SIZE_T_BITS - 1`, so this is an ordinary rotate.
#[inline]
fn rotl(x: usize, n: u32) -> usize {
    (x << n) | (x >> (SIZE_T_BITS - n))
}

fn stbds_siphash_bytes(data_in: &[u8], seed: usize) -> usize {
    let len = data_in.len();

    let mut v0: usize = ((0x736f_6d65usize << 16) << 16).wrapping_add(0x7073_6575) ^ seed;
    let mut v1: usize = ((0x646f_7261usize << 16) << 16).wrapping_add(0x6e64_6f6d) ^ !seed;
    let mut v2: usize = ((0x6c79_6765usize << 16) << 16).wrapping_add(0x6e65_7261) ^ seed;
    let mut v3: usize = ((0x7465_6462usize << 16) << 16).wrapping_add(0x7974_6573) ^ !seed;

    // The C xors in the 64-bit literals unconditionally; on a 32-bit `size_t` they
    // would be truncated by the implicit conversion, which `as usize` reproduces.
    v0 ^= (0x0706_0504_0302_0100u64 as usize) ^ seed;
    v1 ^= (0x0f0e_0d0c_0b0a_0908u64 as usize) ^ !seed;
    v2 ^= (0x0706_0504_0302_0100u64 as usize) ^ seed;
    v3 ^= (0x0f0e_0d0c_0b0a_0908u64 as usize) ^ !seed;

    let word = size_of::<usize>();

    // for (i = 0; i + sizeof(size_t) <= len; i += sizeof(size_t), d += sizeof(size_t))
    let mut i: usize = 0;
    while i + word <= len {
        let d = &data_in[i..];

        // Evaluated in `int` by the C, hence the i32 arithmetic and sign-extending cast.
        let lo: i32 = (d[0] as i32)
            | ((d[1] as i32) << 8)
            | ((d[2] as i32) << 16)
            | ((d[3] as i32) << 24);
        let mut data: usize = lo as u64 as usize;

        let hi: i32 = (d[4] as i32)
            | ((d[5] as i32) << 8)
            | ((d[6] as i32) << 16)
            | ((d[7] as i32) << 24);
        data |= ((((hi as u64 as usize) << 16) as usize) << 16) as usize;

        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += word;
    }

    // data = len << (sizeof(size_t)*8 - 8);
    let mut data: usize = len << (SIZE_T_BITS - 8);

    // The switch on `len - i` falls through from the highest matching case down to
    // `case 0`, so every case from 1 up to `len - i` runs. All of them are `|=`, so
    // applying them in ascending order is equivalent.
    let d = &data_in[i..];
    let rem = len - i;
    if rem >= 1 {
        data |= (d[0] as i32) as u64 as usize;
    }
    if rem >= 2 {
        data |= (((d[1] as i32) << 8) as u64) as usize;
    }
    if rem >= 3 {
        data |= (((d[2] as i32) << 16) as u64) as usize;
    }
    if rem >= 4 {
        // `(d[3] << 24)` is a negative `int` when d[3] >= 0x80: sign-extends.
        data |= (((d[3] as i32) << 24) as u64) as usize;
    }
    if rem >= 5 {
        data |= ((d[4] as usize) << 16) << 16;
    }
    if rem >= 6 {
        data |= ((d[5] as usize) << 20) << 20;
    }
    if rem >= 7 {
        data |= ((d[6] as usize) << 24) << 24;
    }

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

/// `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let bytes = if len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(p as *const u8, len) }
    };
    stbds_siphash_bytes(bytes, seed)
}

/// `void siphash(int init)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];

    // int z = init; for (i=0; i<64; ++i,z++) mem[i] = z;
    // `z` is an `int` that wraps on overflow in practice; the store truncates to
    // the low 8 bits.
    let mut z: c_int = init;
    for i in 0..64usize {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }

    for i in 0..64usize {
        let hash = stbds_siphash_bytes(&mem[..i], 0);
        unsafe {
            printf(c"  { ".as_ptr());
            for j in 0..8u32 {
                let byte = ((hash >> (j * 8)) & 255) as u8;
                printf(c"0x%02x, ".as_ptr(), byte as c_int);
            }
            printf(c" },\n".as_ptr());
        }
    }
}
