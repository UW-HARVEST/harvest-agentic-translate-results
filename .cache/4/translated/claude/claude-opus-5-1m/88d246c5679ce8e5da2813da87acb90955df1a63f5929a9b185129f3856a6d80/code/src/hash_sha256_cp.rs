//! Translation of `crypto_hash/sha256/cp/hash_sha256_cp.c`.
//!
//! The reference build defines neither `__aarch64__` nor `__ARM_FEATURE_SHA2`,
//! so `HAVE_SHA256_ARMCRYPTO` is undefined and the portable (`#else`) branch is
//! the one translated here.  `ACQUIRE_FENCE` expands to `(void) 0` because
//! neither `HAVE_GCC_MEMORY_FENCES` nor `HAVE_C11_MEMORY_FENCES` is defined.

use crate::common::{load32_be, rotr32, store32_be, store64_be};
use core::ffi::{c_int, c_ulonglong, c_void};
use core::ptr::addr_of_mut;

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/* static void be32enc_vect(unsigned char *dst, const uint32_t *src, size_t len) */
unsafe fn be32enc_vect(dst: *mut u8, src: *const u32, len: usize) {
    let mut i: usize = 0;
    while i < len / 4 {
        store32_be(dst.add(i * 4), *src.add(i));
        i += 1;
    }
}

/* static void be32dec_vect(uint32_t *dst, const unsigned char *src, size_t len) */
unsafe fn be32dec_vect(dst: *mut u32, src: *const u8, len: usize) {
    let mut i: usize = 0;
    while i < len / 4 {
        *dst.add(i) = load32_be(src.add(i * 4));
        i += 1;
    }
}

static Krnd: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/* #define Ch(x, y, z) ((x & (y ^ z)) ^ z) */
#[inline(always)]
fn Ch(x: u32, y: u32, z: u32) -> u32 {
    (x & (y ^ z)) ^ z
}

/* #define Maj(x, y, z) ((x & (y | z)) | (y & z)) */
#[inline(always)]
fn Maj(x: u32, y: u32, z: u32) -> u32 {
    (x & (y | z)) | (y & z)
}

/* #define SHR(x, n) (x >> n) */
#[inline(always)]
fn SHR(x: u32, n: i32) -> u32 {
    x >> n
}

/* #define ROTR(x, n) ROTR32(x, n) */
#[inline(always)]
fn ROTR(x: u32, n: i32) -> u32 {
    rotr32(x, n)
}

/* #define S0(x) (ROTR(x, 2) ^ ROTR(x, 13) ^ ROTR(x, 22)) */
#[inline(always)]
fn S0(x: u32) -> u32 {
    ROTR(x, 2) ^ ROTR(x, 13) ^ ROTR(x, 22)
}

/* #define S1(x) (ROTR(x, 6) ^ ROTR(x, 11) ^ ROTR(x, 25)) */
#[inline(always)]
fn S1(x: u32) -> u32 {
    ROTR(x, 6) ^ ROTR(x, 11) ^ ROTR(x, 25)
}

/* #define s0(x) (ROTR(x, 7) ^ ROTR(x, 18) ^ SHR(x, 3)) */
#[inline(always)]
fn s0(x: u32) -> u32 {
    ROTR(x, 7) ^ ROTR(x, 18) ^ SHR(x, 3)
}

/* #define s1(x) (ROTR(x, 17) ^ ROTR(x, 19) ^ SHR(x, 10)) */
#[inline(always)]
fn s1(x: u32) -> u32 {
    ROTR(x, 17) ^ ROTR(x, 19) ^ SHR(x, 10)
}

/*
 * #define RND(a, b, c, d, e, f, g, h, k) \
 *     h += S1(e) + Ch(e, f, g) + k;      \
 *     d += h;                            \
 *     h += S0(a) + Maj(a, b, c);
 *
 * The eight arguments are always the eight distinct elements of S[], so the
 * index-based expansion below is an exact transcription.
 */
#[inline(always)]
#[allow(clippy::too_many_arguments)]
unsafe fn RND(
    S: *mut u32,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    g: usize,
    h: usize,
    k: u32,
) {
    *S.add(h) = (*S.add(h)).wrapping_add(
        S1(*S.add(e))
            .wrapping_add(Ch(*S.add(e), *S.add(f), *S.add(g)))
            .wrapping_add(k),
    );
    *S.add(d) = (*S.add(d)).wrapping_add(*S.add(h));
    *S.add(h) = (*S.add(h)).wrapping_add(S0(*S.add(a)).wrapping_add(Maj(
        *S.add(a),
        *S.add(b),
        *S.add(c),
    )));
}

/*
 * #define RNDr(S, W, i, ii)                                                   \
 *     RND(S[(64 - i) % 8], S[(65 - i) % 8], S[(66 - i) % 8], S[(67 - i) % 8], \
 *         S[(68 - i) % 8], S[(69 - i) % 8], S[(70 - i) % 8], S[(71 - i) % 8], \
 *         W[i + ii] + Krnd[i + ii])
 */
#[inline(always)]
unsafe fn RNDr(S: *mut u32, W: *const u32, i: usize, ii: usize) {
    RND(
        S,
        (64 - i) % 8,
        (65 - i) % 8,
        (66 - i) % 8,
        (67 - i) % 8,
        (68 - i) % 8,
        (69 - i) % 8,
        (70 - i) % 8,
        (71 - i) % 8,
        (*W.add(i + ii)).wrapping_add(Krnd[i + ii]),
    );
}

/*
 * #define MSCH(W, ii, i) \
 *     W[i + ii + 16] =   \
 *         s1(W[i + ii + 14]) + W[i + ii + 9] + s0(W[i + ii + 1]) + W[i + ii]
 */
#[inline(always)]
unsafe fn MSCH(W: *mut u32, ii: usize, i: usize) {
    *W.add(i + ii + 16) = s1(*W.add(i + ii + 14))
        .wrapping_add(*W.add(i + ii + 9))
        .wrapping_add(s0(*W.add(i + ii + 1)))
        .wrapping_add(*W.add(i + ii));
}

/*
 * static void SHA256_Transform(uint32_t state[8], const uint8_t block[64],
 *                              uint32_t W[64], uint32_t S[8])
 */
unsafe fn SHA256_Transform(state: *mut u32, block: *const u8, W: *mut u32, S: *mut u32) {
    let mut i: usize;

    be32dec_vect(W, block, 64);
    core::ptr::copy_nonoverlapping(state as *const u8, S as *mut u8, 32);
    i = 0;
    while i < 64 {
        RNDr(S, W, 0, i);
        RNDr(S, W, 1, i);
        RNDr(S, W, 2, i);
        RNDr(S, W, 3, i);
        RNDr(S, W, 4, i);
        RNDr(S, W, 5, i);
        RNDr(S, W, 6, i);
        RNDr(S, W, 7, i);
        RNDr(S, W, 8, i);
        RNDr(S, W, 9, i);
        RNDr(S, W, 10, i);
        RNDr(S, W, 11, i);
        RNDr(S, W, 12, i);
        RNDr(S, W, 13, i);
        RNDr(S, W, 14, i);
        RNDr(S, W, 15, i);
        if i == 48 {
            break;
        }
        MSCH(W, 0, i);
        MSCH(W, 1, i);
        MSCH(W, 2, i);
        MSCH(W, 3, i);
        MSCH(W, 4, i);
        MSCH(W, 5, i);
        MSCH(W, 6, i);
        MSCH(W, 7, i);
        MSCH(W, 8, i);
        MSCH(W, 9, i);
        MSCH(W, 10, i);
        MSCH(W, 11, i);
        MSCH(W, 12, i);
        MSCH(W, 13, i);
        MSCH(W, 14, i);
        MSCH(W, 15, i);

        i += 16;
    }
    i = 0;
    while i < 8 {
        *state.add(i) = (*state.add(i)).wrapping_add(*S.add(i));
        i += 1;
    }
}

static PAD: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

/* static void SHA256_Pad(crypto_hash_sha256_state *state, uint32_t tmp32[64 + 8]) */
unsafe fn SHA256_Pad(state: *mut crypto_hash_sha256_state, tmp32: *mut u32) {
    let r: u32;
    let mut i: u32;

    /* ACQUIRE_FENCE == (void) 0 */
    let st = addr_of_mut!((*state).state) as *mut u32;
    let bf = addr_of_mut!((*state).buf) as *mut u8;

    r = (((*state).count >> 3) & 0x3f) as u32;
    if r < 56 {
        i = 0;
        while i < 56 - r {
            *bf.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
    } else {
        i = 0;
        while i < 64 - r {
            *bf.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
        SHA256_Transform(st, bf, tmp32, tmp32.add(64));
        core::ptr::write_bytes(bf, 0, 56);
    }
    store64_be(bf.add(56), (*state).count);
    SHA256_Transform(st, bf, tmp32, tmp32.add(64));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_init(
    state: *mut crypto_hash_sha256_state,
) -> c_int {
    static sha256_initial_state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    (*state).count = 0u64;
    core::ptr::copy_nonoverlapping(
        sha256_initial_state.as_ptr() as *const u8,
        addr_of_mut!((*state).state) as *mut u8,
        core::mem::size_of_val(&sha256_initial_state),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_update(
    state: *mut crypto_hash_sha256_state,
    mut in_: *const u8,
    mut inlen: c_ulonglong,
) -> c_int {
    let mut tmp32: [u32; 64 + 8] = [0u32; 64 + 8];
    let mut i: c_ulonglong;
    let r: c_ulonglong;

    if inlen == 0 {
        return 0;
    }
    /* ACQUIRE_FENCE == (void) 0 */
    let st = addr_of_mut!((*state).state) as *mut u32;
    let bf = addr_of_mut!((*state).buf) as *mut u8;
    let w = tmp32.as_mut_ptr();

    r = ((*state).count >> 3) & 0x3f;

    (*state).count = (*state).count.wrapping_add((inlen as u64) << 3);
    if inlen < 64 - r {
        i = 0;
        while i < inlen {
            *bf.add((r + i) as usize) = *in_.add(i as usize);
            i += 1;
        }
        return 0;
    }
    i = 0;
    while i < 64 - r {
        *bf.add((r + i) as usize) = *in_.add(i as usize);
        i += 1;
    }
    SHA256_Transform(st, bf, w, w.add(64));
    in_ = in_.add((64 - r) as usize);
    inlen -= 64 - r;

    while inlen >= 64 {
        SHA256_Transform(st, in_, w, w.add(64));
        in_ = in_.add(64);
        inlen -= 64;
    }
    inlen &= 63;
    i = 0;
    while i < inlen {
        *bf.add(i as usize) = *in_.add(i as usize);
        i += 1;
    }
    sodium_memzero(w as *mut c_void, core::mem::size_of_val(&tmp32));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_final(
    state: *mut crypto_hash_sha256_state,
    out: *mut u8,
) -> c_int {
    let mut tmp32: [u32; 64 + 8] = [0u32; 64 + 8];

    SHA256_Pad(state, tmp32.as_mut_ptr());
    be32enc_vect(out, addr_of_mut!((*state).state) as *const u32, 32);
    sodium_memzero(tmp32.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&tmp32));
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_hash_sha256_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    let mut state = crypto_hash_sha256_state {
        state: [0u32; 8],
        count: 0u64,
        buf: [0u8; 64],
    };

    crypto_hash_sha256_init(&mut state);
    crypto_hash_sha256_update(&mut state, in_, inlen);
    crypto_hash_sha256_final(&mut state, out);

    0
}
