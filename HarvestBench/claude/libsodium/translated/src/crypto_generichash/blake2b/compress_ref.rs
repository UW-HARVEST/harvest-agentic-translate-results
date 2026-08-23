//! Translation of `crypto_generichash/blake2b/ref/blake2b-compress-ref.c`.
//!
//! The reference build defines none of `HAVE_EMMINTRIN_H` / `__SSE2__` /
//! `SSSE3` / `SSE41` / `AVX2`, so this portable implementation is the only
//! compress routine actually compiled in.

use core::ffi::c_int;

use crate::common::{load64_le, rotr64};

use super::blake2b_ref::blake2b_state;

/// `CRYPTO_ALIGN(64) static const uint64_t blake2b_IV[8]`
///
/// (The alignment attribute has no observable effect for the reference
/// implementation, which only performs scalar loads.)
static blake2b_IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

static blake2b_sigma: [[u8; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

/// The `G` macro of the C source.
#[inline(always)]
fn g(
    v: &mut [u64; 16],
    m: &[u64; 16],
    r: usize,
    i: usize,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
) {
    v[a] = v[a]
        .wrapping_add(v[b])
        .wrapping_add(m[blake2b_sigma[r][2 * i + 0] as usize]);
    v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = v[a]
        .wrapping_add(v[b])
        .wrapping_add(m[blake2b_sigma[r][2 * i + 1] as usize]);
    v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 63);
}

/// The `ROUND` macro of the C source.
#[inline(always)]
fn round(v: &mut [u64; 16], m: &[u64; 16], r: usize) {
    g(v, m, r, 0, 0, 4, 8, 12);
    g(v, m, r, 1, 1, 5, 9, 13);
    g(v, m, r, 2, 2, 6, 10, 14);
    g(v, m, r, 3, 3, 7, 11, 15);
    g(v, m, r, 4, 0, 5, 10, 15);
    g(v, m, r, 5, 1, 6, 11, 12);
    g(v, m, r, 6, 2, 7, 8, 13);
    g(v, m, r, 7, 3, 4, 9, 14);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_compress_ref(
    S: *mut blake2b_state,
    block: *const u8,
) -> c_int {
    let mut m = [0u64; 16];
    let mut v = [0u64; 16];

    for i in 0..16usize {
        m[i] = unsafe { load64_le(block.add(i * core::mem::size_of::<u64>())) };
    }
    for i in 0..8usize {
        v[i] = unsafe { (*S).h[i] };
    }
    v[8] = blake2b_IV[0];
    v[9] = blake2b_IV[1];
    v[10] = blake2b_IV[2];
    v[11] = blake2b_IV[3];
    v[12] = unsafe { (*S).t[0] } ^ blake2b_IV[4];
    v[13] = unsafe { (*S).t[1] } ^ blake2b_IV[5];
    v[14] = unsafe { (*S).f[0] } ^ blake2b_IV[6];
    v[15] = unsafe { (*S).f[1] } ^ blake2b_IV[7];

    round(&mut v, &m, 0);
    round(&mut v, &m, 1);
    round(&mut v, &m, 2);
    round(&mut v, &m, 3);
    round(&mut v, &m, 4);
    round(&mut v, &m, 5);
    round(&mut v, &m, 6);
    round(&mut v, &m, 7);
    round(&mut v, &m, 8);
    round(&mut v, &m, 9);
    round(&mut v, &m, 10);
    round(&mut v, &m, 11);

    for i in 0..8usize {
        unsafe { (*S).h[i] = (*S).h[i] ^ v[i] ^ v[i + 8] };
    }

    0
}
