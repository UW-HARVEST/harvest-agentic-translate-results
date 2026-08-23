//! Translation of `crypto_generichash/blake2b/ref/blake2b-compress-ref.c`.
//!
//! Exports (after the `private/quirks.h` renaming):
//!   * `_sodium_blake2b_compress_ref`

use crate::common::*;
use core::ffi::c_int;

/* enum blake2b_constant (blake2.h) */
const BLAKE2B_BLOCKBYTES: usize = 128;

/* `typedef struct blake2b_state` from blake2.h.  The whole declaration sits
 * inside `#pragma pack(push, 1)`, hence `packed`.  In C: sizeof == 361,
 * _Alignof == 1, field offsets h=0 t=64 f=80 buf=96 buflen=352 last_node=360. */
#[repr(C, packed)]
pub struct blake2b_state {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; 2 * 128],
    pub buflen: usize,
    pub last_node: u8,
}

/* CRYPTO_ALIGN(64) static const uint64_t blake2b_IV[8] */
#[repr(C, align(64))]
struct AlignedIV([u64; 8]);

static blake2b_IV: AlignedIV = AlignedIV([
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
]);

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

/* #define G(r, i, a, b, c, d) */
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
    v[a] = v[a].wrapping_add(v[b].wrapping_add(m[blake2b_sigma[r][2 * i + 0] as usize]));
    v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = v[a].wrapping_add(v[b].wrapping_add(m[blake2b_sigma[r][2 * i + 1] as usize]));
    v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 63);
}

/* #define ROUND(r) */
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
    let mut m: [u64; 16] = [0; 16];
    let mut v: [u64; 16] = [0; 16];

    let _ = BLAKE2B_BLOCKBYTES;

    let mut i: usize = 0;
    while i < 16 {
        m[i] = load64_le(block.add(i * core::mem::size_of::<u64>()));
        i += 1;
    }
    i = 0;
    while i < 8 {
        v[i] = (*S).h[i];
        i += 1;
    }
    v[8] = blake2b_IV.0[0];
    v[9] = blake2b_IV.0[1];
    v[10] = blake2b_IV.0[2];
    v[11] = blake2b_IV.0[3];
    v[12] = (*S).t[0] ^ blake2b_IV.0[4];
    v[13] = (*S).t[1] ^ blake2b_IV.0[5];
    v[14] = (*S).f[0] ^ blake2b_IV.0[6];
    v[15] = (*S).f[1] ^ blake2b_IV.0[7];

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

    i = 0;
    while i < 8 {
        (*S).h[i] = (*S).h[i] ^ v[i] ^ v[i + 8];
        i += 1;
    }

    0
}
