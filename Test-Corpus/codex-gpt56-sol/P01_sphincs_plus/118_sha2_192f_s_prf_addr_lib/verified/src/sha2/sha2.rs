//! Translation of `lib/sha2/src/sha2.c` (and `lib/sha2/include/sha2.h`).
//!
//! > Based on the public domain implementation in
//! > `crypto_hash/sha512/ref/` from <http://bench.cr.yp.to/supercop.html>
//! > by D. J. Bernstein
//!
//! The C file is written with heavy use of preprocessor macros (`Ch`, `Maj`,
//! `Sigma0`, `Sigma1`, `sigma0`, `sigma1`, `SHR`, `ROTR`, `M`, `EXPAND`, `F`).
//! They are expanded here into `#[inline(always)]` helper functions that
//! operate on the working registers `[a, b, c, d, e, f, g, h]` and the message
//! schedule `[w0 .. w15]`; the round order, the register rotation and every
//! round constant are transcribed verbatim, so the behaviour is
//! byte-identical. All additions are modular (`wrapping_add`), just like the
//! `uint32_t` / `uint64_t` arithmetic in C.

use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::utils::SPX_u32_to_bytes;

// ---------------------------------------------------------------------------
// sha2.h
// ---------------------------------------------------------------------------

/// `#define SPX_SHA256_BLOCK_BYTES 64`
pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
/// `#define SPX_SHA256_OUTPUT_BYTES 32` (this does not necessarily equal `SPX_N`)
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;

/// `#define SPX_SHA512_BLOCK_BYTES 128`
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
/// `#define SPX_SHA512_OUTPUT_BYTES 64`
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;

/// `#define SPX_SHA256_ADDR_BYTES 22`
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// ---------------------------------------------------------------------------
// static load/store helpers
// ---------------------------------------------------------------------------

/// `static uint32_t load_bigendian_32(const uint8_t *x)`
#[inline(always)]
unsafe fn load_bigendian_32(x: *const u8) -> u32 {
    (*x.add(3) as u32)
        | ((*x.add(2) as u32) << 8)
        | ((*x.add(1) as u32) << 16)
        | ((*x.add(0) as u32) << 24)
}

/// `static uint64_t load_bigendian_64(const uint8_t *x)`
#[inline(always)]
unsafe fn load_bigendian_64(x: *const u8) -> u64 {
    (*x.add(7) as u64)
        | ((*x.add(6) as u64) << 8)
        | ((*x.add(5) as u64) << 16)
        | ((*x.add(4) as u64) << 24)
        | ((*x.add(3) as u64) << 32)
        | ((*x.add(2) as u64) << 40)
        | ((*x.add(1) as u64) << 48)
        | ((*x.add(0) as u64) << 56)
}

/// `static void store_bigendian_32(uint8_t *x, uint64_t u)`
#[inline(always)]
unsafe fn store_bigendian_32(x: *mut u8, u: u64) {
    let mut u = u;
    *x.add(3) = u as u8;
    u >>= 8;
    *x.add(2) = u as u8;
    u >>= 8;
    *x.add(1) = u as u8;
    u >>= 8;
    *x.add(0) = u as u8;
}

/// `static void store_bigendian_64(uint8_t *x, uint64_t u)`
#[inline(always)]
unsafe fn store_bigendian_64(x: *mut u8, u: u64) {
    let mut u = u;
    *x.add(7) = u as u8;
    u >>= 8;
    *x.add(6) = u as u8;
    u >>= 8;
    *x.add(5) = u as u8;
    u >>= 8;
    *x.add(4) = u as u8;
    u >>= 8;
    *x.add(3) = u as u8;
    u >>= 8;
    *x.add(2) = u as u8;
    u >>= 8;
    *x.add(1) = u as u8;
    u >>= 8;
    *x.add(0) = u as u8;
}

// ---------------------------------------------------------------------------
// The macro zoo of sha2.c
//
//   #define SHR(x, c)     ((x) >> (c))
//   #define ROTR_32(x, c) (((x) >> (c)) | ((x) << (32 - (c))))
//   #define ROTR_64(x, c) (((x) >> (c)) | ((x) << (64 - (c))))
//   #define Ch(x, y, z)   (((x) & (y)) ^ (~(x) & (z)))
//   #define Maj(x, y, z)  (((x) & (y)) ^ ((x) & (z)) ^ ((y) & (z)))
// ---------------------------------------------------------------------------

#[inline(always)]
fn rotr_32(x: u32, c: u32) -> u32 {
    (x >> c) | (x << (32 - c))
}

#[inline(always)]
fn rotr_64(x: u64, c: u32) -> u64 {
    (x >> c) | (x << (64 - c))
}

#[inline(always)]
fn ch_32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

#[inline(always)]
fn maj_32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

#[inline(always)]
fn ch_64(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (!x & z)
}

#[inline(always)]
fn maj_64(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (x & z) ^ (y & z)
}

/// `#define Sigma0_32(x) (ROTR_32(x, 2) ^ ROTR_32(x,13) ^ ROTR_32(x,22))`
#[inline(always)]
fn big_sigma0_32(x: u32) -> u32 {
    rotr_32(x, 2) ^ rotr_32(x, 13) ^ rotr_32(x, 22)
}

/// `#define Sigma1_32(x) (ROTR_32(x, 6) ^ ROTR_32(x,11) ^ ROTR_32(x,25))`
#[inline(always)]
fn big_sigma1_32(x: u32) -> u32 {
    rotr_32(x, 6) ^ rotr_32(x, 11) ^ rotr_32(x, 25)
}

/// `#define sigma0_32(x) (ROTR_32(x, 7) ^ ROTR_32(x,18) ^ SHR(x, 3))`
#[inline(always)]
fn small_sigma0_32(x: u32) -> u32 {
    rotr_32(x, 7) ^ rotr_32(x, 18) ^ (x >> 3)
}

/// `#define sigma1_32(x) (ROTR_32(x,17) ^ ROTR_32(x,19) ^ SHR(x,10))`
#[inline(always)]
fn small_sigma1_32(x: u32) -> u32 {
    rotr_32(x, 17) ^ rotr_32(x, 19) ^ (x >> 10)
}

/// `#define Sigma0_64(x) (ROTR_64(x,28) ^ ROTR_64(x,34) ^ ROTR_64(x,39))`
#[inline(always)]
fn big_sigma0_64(x: u64) -> u64 {
    rotr_64(x, 28) ^ rotr_64(x, 34) ^ rotr_64(x, 39)
}

/// `#define Sigma1_64(x) (ROTR_64(x,14) ^ ROTR_64(x,18) ^ ROTR_64(x,41))`
#[inline(always)]
fn big_sigma1_64(x: u64) -> u64 {
    rotr_64(x, 14) ^ rotr_64(x, 18) ^ rotr_64(x, 41)
}

/// `#define sigma0_64(x) (ROTR_64(x, 1) ^ ROTR_64(x, 8) ^ SHR(x,7))`
#[inline(always)]
fn small_sigma0_64(x: u64) -> u64 {
    rotr_64(x, 1) ^ rotr_64(x, 8) ^ (x >> 7)
}

/// `#define sigma1_64(x) (ROTR_64(x,19) ^ ROTR_64(x,61) ^ SHR(x,6))`
#[inline(always)]
fn small_sigma1_64(x: u64) -> u64 {
    rotr_64(x, 19) ^ rotr_64(x, 61) ^ (x >> 6)
}

/// `#define M_32(w0, w14, w9, w1) w0 = sigma1_32(w14) + (w9) + sigma0_32(w1) + (w0);`
///
/// `w` is `[w0, w1, .., w15]`, the parameters are the indices of the four words.
#[inline(always)]
fn m_32(w: &mut [u32; 16], i0: usize, i14: usize, i9: usize, i1: usize) {
    w[i0] = small_sigma1_32(w[i14])
        .wrapping_add(w[i9])
        .wrapping_add(small_sigma0_32(w[i1]))
        .wrapping_add(w[i0]);
}

/// `#define M_64(w0, w14, w9, w1) w0 = sigma1_64(w14) + (w9) + sigma0_64(w1) + (w0);`
#[inline(always)]
fn m_64(w: &mut [u64; 16], i0: usize, i14: usize, i9: usize, i1: usize) {
    w[i0] = small_sigma1_64(w[i14])
        .wrapping_add(w[i9])
        .wrapping_add(small_sigma0_64(w[i1]))
        .wrapping_add(w[i0]);
}

/// `#define EXPAND_32`
#[inline(always)]
fn expand_32(w: &mut [u32; 16]) {
    m_32(w, 0, 14, 9, 1);
    m_32(w, 1, 15, 10, 2);
    m_32(w, 2, 0, 11, 3);
    m_32(w, 3, 1, 12, 4);
    m_32(w, 4, 2, 13, 5);
    m_32(w, 5, 3, 14, 6);
    m_32(w, 6, 4, 15, 7);
    m_32(w, 7, 5, 0, 8);
    m_32(w, 8, 6, 1, 9);
    m_32(w, 9, 7, 2, 10);
    m_32(w, 10, 8, 3, 11);
    m_32(w, 11, 9, 4, 12);
    m_32(w, 12, 10, 5, 13);
    m_32(w, 13, 11, 6, 14);
    m_32(w, 14, 12, 7, 15);
    m_32(w, 15, 13, 8, 0);
}

/// `#define EXPAND_64`
#[inline(always)]
fn expand_64(w: &mut [u64; 16]) {
    m_64(w, 0, 14, 9, 1);
    m_64(w, 1, 15, 10, 2);
    m_64(w, 2, 0, 11, 3);
    m_64(w, 3, 1, 12, 4);
    m_64(w, 4, 2, 13, 5);
    m_64(w, 5, 3, 14, 6);
    m_64(w, 6, 4, 15, 7);
    m_64(w, 7, 5, 0, 8);
    m_64(w, 8, 6, 1, 9);
    m_64(w, 9, 7, 2, 10);
    m_64(w, 10, 8, 3, 11);
    m_64(w, 11, 9, 4, 12);
    m_64(w, 12, 10, 5, 13);
    m_64(w, 13, 11, 6, 14);
    m_64(w, 14, 12, 7, 15);
    m_64(w, 15, 13, 8, 0);
}

/// ```c
/// #define F_32(w, k)                                   \
///     T1 = h + Sigma1_32(e) + Ch(e, f, g) + (k) + (w); \
///     T2 = Sigma0_32(a) + Maj(a, b, c);                \
///     h = g; g = f; f = e; e = d + T1;                 \
///     d = c; c = b; b = a; a = T1 + T2;
/// ```
///
/// `v` holds the working registers in the order `[a, b, c, d, e, f, g, h]`.
#[inline(always)]
fn f_32(v: &mut [u32; 8], w: u32, k: u32) {
    let t1 = v[7]
        .wrapping_add(big_sigma1_32(v[4]))
        .wrapping_add(ch_32(v[4], v[5], v[6]))
        .wrapping_add(k)
        .wrapping_add(w);
    let t2 = big_sigma0_32(v[0]).wrapping_add(maj_32(v[0], v[1], v[2]));
    v[7] = v[6];
    v[6] = v[5];
    v[5] = v[4];
    v[4] = v[3].wrapping_add(t1);
    v[3] = v[2];
    v[2] = v[1];
    v[1] = v[0];
    v[0] = t1.wrapping_add(t2);
}

/// ```c
/// #define F_64(w,k) \
///     T1 = h + Sigma1_64(e) + Ch(e,f,g) + k + w; \
///     T2 = Sigma0_64(a) + Maj(a,b,c); \
///     h = g; g = f; f = e; e = d + T1; \
///     d = c; c = b; b = a; a = T1 + T2;
/// ```
#[inline(always)]
fn f_64(v: &mut [u64; 8], w: u64, k: u64) {
    let t1 = v[7]
        .wrapping_add(big_sigma1_64(v[4]))
        .wrapping_add(ch_64(v[4], v[5], v[6]))
        .wrapping_add(k)
        .wrapping_add(w);
    let t2 = big_sigma0_64(v[0]).wrapping_add(maj_64(v[0], v[1], v[2]));
    v[7] = v[6];
    v[6] = v[5];
    v[5] = v[4];
    v[4] = v[3].wrapping_add(t1);
    v[3] = v[2];
    v[2] = v[1];
    v[1] = v[0];
    v[0] = t1.wrapping_add(t2);
}

// ---------------------------------------------------------------------------
// static size_t crypto_hashblocks_sha256(uint8_t *statebytes,
//                                        const uint8_t *in, size_t inlen)
// ---------------------------------------------------------------------------

unsafe fn crypto_hashblocks_sha256(statebytes: *mut u8, in_: *const u8, inlen: usize) -> usize {
    let mut state = [0u32; 8];
    /* v = [a, b, c, d, e, f, g, h] */
    let mut v = [0u32; 8];

    let mut in_ = in_;
    let mut inlen = inlen;

    v[0] = load_bigendian_32(statebytes.add(0));
    state[0] = v[0];
    v[1] = load_bigendian_32(statebytes.add(4));
    state[1] = v[1];
    v[2] = load_bigendian_32(statebytes.add(8));
    state[2] = v[2];
    v[3] = load_bigendian_32(statebytes.add(12));
    state[3] = v[3];
    v[4] = load_bigendian_32(statebytes.add(16));
    state[4] = v[4];
    v[5] = load_bigendian_32(statebytes.add(20));
    state[5] = v[5];
    v[6] = load_bigendian_32(statebytes.add(24));
    state[6] = v[6];
    v[7] = load_bigendian_32(statebytes.add(28));
    state[7] = v[7];

    while inlen >= 64 {
        let mut w = [0u32; 16];
        w[0] = load_bigendian_32(in_.add(0));
        w[1] = load_bigendian_32(in_.add(4));
        w[2] = load_bigendian_32(in_.add(8));
        w[3] = load_bigendian_32(in_.add(12));
        w[4] = load_bigendian_32(in_.add(16));
        w[5] = load_bigendian_32(in_.add(20));
        w[6] = load_bigendian_32(in_.add(24));
        w[7] = load_bigendian_32(in_.add(28));
        w[8] = load_bigendian_32(in_.add(32));
        w[9] = load_bigendian_32(in_.add(36));
        w[10] = load_bigendian_32(in_.add(40));
        w[11] = load_bigendian_32(in_.add(44));
        w[12] = load_bigendian_32(in_.add(48));
        w[13] = load_bigendian_32(in_.add(52));
        w[14] = load_bigendian_32(in_.add(56));
        w[15] = load_bigendian_32(in_.add(60));

        f_32(&mut v, w[0], 0x428a2f98);
        f_32(&mut v, w[1], 0x71374491);
        f_32(&mut v, w[2], 0xb5c0fbcf);
        f_32(&mut v, w[3], 0xe9b5dba5);
        f_32(&mut v, w[4], 0x3956c25b);
        f_32(&mut v, w[5], 0x59f111f1);
        f_32(&mut v, w[6], 0x923f82a4);
        f_32(&mut v, w[7], 0xab1c5ed5);
        f_32(&mut v, w[8], 0xd807aa98);
        f_32(&mut v, w[9], 0x12835b01);
        f_32(&mut v, w[10], 0x243185be);
        f_32(&mut v, w[11], 0x550c7dc3);
        f_32(&mut v, w[12], 0x72be5d74);
        f_32(&mut v, w[13], 0x80deb1fe);
        f_32(&mut v, w[14], 0x9bdc06a7);
        f_32(&mut v, w[15], 0xc19bf174);

        expand_32(&mut w);

        f_32(&mut v, w[0], 0xe49b69c1);
        f_32(&mut v, w[1], 0xefbe4786);
        f_32(&mut v, w[2], 0x0fc19dc6);
        f_32(&mut v, w[3], 0x240ca1cc);
        f_32(&mut v, w[4], 0x2de92c6f);
        f_32(&mut v, w[5], 0x4a7484aa);
        f_32(&mut v, w[6], 0x5cb0a9dc);
        f_32(&mut v, w[7], 0x76f988da);
        f_32(&mut v, w[8], 0x983e5152);
        f_32(&mut v, w[9], 0xa831c66d);
        f_32(&mut v, w[10], 0xb00327c8);
        f_32(&mut v, w[11], 0xbf597fc7);
        f_32(&mut v, w[12], 0xc6e00bf3);
        f_32(&mut v, w[13], 0xd5a79147);
        f_32(&mut v, w[14], 0x06ca6351);
        f_32(&mut v, w[15], 0x14292967);

        expand_32(&mut w);

        f_32(&mut v, w[0], 0x27b70a85);
        f_32(&mut v, w[1], 0x2e1b2138);
        f_32(&mut v, w[2], 0x4d2c6dfc);
        f_32(&mut v, w[3], 0x53380d13);
        f_32(&mut v, w[4], 0x650a7354);
        f_32(&mut v, w[5], 0x766a0abb);
        f_32(&mut v, w[6], 0x81c2c92e);
        f_32(&mut v, w[7], 0x92722c85);
        f_32(&mut v, w[8], 0xa2bfe8a1);
        f_32(&mut v, w[9], 0xa81a664b);
        f_32(&mut v, w[10], 0xc24b8b70);
        f_32(&mut v, w[11], 0xc76c51a3);
        f_32(&mut v, w[12], 0xd192e819);
        f_32(&mut v, w[13], 0xd6990624);
        f_32(&mut v, w[14], 0xf40e3585);
        f_32(&mut v, w[15], 0x106aa070);

        expand_32(&mut w);

        f_32(&mut v, w[0], 0x19a4c116);
        f_32(&mut v, w[1], 0x1e376c08);
        f_32(&mut v, w[2], 0x2748774c);
        f_32(&mut v, w[3], 0x34b0bcb5);
        f_32(&mut v, w[4], 0x391c0cb3);
        f_32(&mut v, w[5], 0x4ed8aa4a);
        f_32(&mut v, w[6], 0x5b9cca4f);
        f_32(&mut v, w[7], 0x682e6ff3);
        f_32(&mut v, w[8], 0x748f82ee);
        f_32(&mut v, w[9], 0x78a5636f);
        f_32(&mut v, w[10], 0x84c87814);
        f_32(&mut v, w[11], 0x8cc70208);
        f_32(&mut v, w[12], 0x90befffa);
        f_32(&mut v, w[13], 0xa4506ceb);
        f_32(&mut v, w[14], 0xbef9a3f7);
        f_32(&mut v, w[15], 0xc67178f2);

        v[0] = v[0].wrapping_add(state[0]);
        v[1] = v[1].wrapping_add(state[1]);
        v[2] = v[2].wrapping_add(state[2]);
        v[3] = v[3].wrapping_add(state[3]);
        v[4] = v[4].wrapping_add(state[4]);
        v[5] = v[5].wrapping_add(state[5]);
        v[6] = v[6].wrapping_add(state[6]);
        v[7] = v[7].wrapping_add(state[7]);

        state[0] = v[0];
        state[1] = v[1];
        state[2] = v[2];
        state[3] = v[3];
        state[4] = v[4];
        state[5] = v[5];
        state[6] = v[6];
        state[7] = v[7];

        in_ = in_.add(64);
        inlen -= 64;
    }

    store_bigendian_32(statebytes.add(0), state[0] as u64);
    store_bigendian_32(statebytes.add(4), state[1] as u64);
    store_bigendian_32(statebytes.add(8), state[2] as u64);
    store_bigendian_32(statebytes.add(12), state[3] as u64);
    store_bigendian_32(statebytes.add(16), state[4] as u64);
    store_bigendian_32(statebytes.add(20), state[5] as u64);
    store_bigendian_32(statebytes.add(24), state[6] as u64);
    store_bigendian_32(statebytes.add(28), state[7] as u64);

    inlen
}

// ---------------------------------------------------------------------------
// static int crypto_hashblocks_sha512(unsigned char *statebytes,
//                                     const unsigned char *in,
//                                     unsigned long long inlen)
// ---------------------------------------------------------------------------

unsafe fn crypto_hashblocks_sha512(statebytes: *mut u8, in_: *const u8, inlen: u64) -> i32 {
    let mut state = [0u64; 8];
    /* v = [a, b, c, d, e, f, g, h] */
    let mut v = [0u64; 8];

    let mut in_ = in_;
    let mut inlen = inlen;

    v[0] = load_bigendian_64(statebytes.add(0));
    state[0] = v[0];
    v[1] = load_bigendian_64(statebytes.add(8));
    state[1] = v[1];
    v[2] = load_bigendian_64(statebytes.add(16));
    state[2] = v[2];
    v[3] = load_bigendian_64(statebytes.add(24));
    state[3] = v[3];
    v[4] = load_bigendian_64(statebytes.add(32));
    state[4] = v[4];
    v[5] = load_bigendian_64(statebytes.add(40));
    state[5] = v[5];
    v[6] = load_bigendian_64(statebytes.add(48));
    state[6] = v[6];
    v[7] = load_bigendian_64(statebytes.add(56));
    state[7] = v[7];

    while inlen >= 128 {
        let mut w = [0u64; 16];
        w[0] = load_bigendian_64(in_.add(0));
        w[1] = load_bigendian_64(in_.add(8));
        w[2] = load_bigendian_64(in_.add(16));
        w[3] = load_bigendian_64(in_.add(24));
        w[4] = load_bigendian_64(in_.add(32));
        w[5] = load_bigendian_64(in_.add(40));
        w[6] = load_bigendian_64(in_.add(48));
        w[7] = load_bigendian_64(in_.add(56));
        w[8] = load_bigendian_64(in_.add(64));
        w[9] = load_bigendian_64(in_.add(72));
        w[10] = load_bigendian_64(in_.add(80));
        w[11] = load_bigendian_64(in_.add(88));
        w[12] = load_bigendian_64(in_.add(96));
        w[13] = load_bigendian_64(in_.add(104));
        w[14] = load_bigendian_64(in_.add(112));
        w[15] = load_bigendian_64(in_.add(120));

        f_64(&mut v, w[0], 0x428a2f98d728ae22);
        f_64(&mut v, w[1], 0x7137449123ef65cd);
        f_64(&mut v, w[2], 0xb5c0fbcfec4d3b2f);
        f_64(&mut v, w[3], 0xe9b5dba58189dbbc);
        f_64(&mut v, w[4], 0x3956c25bf348b538);
        f_64(&mut v, w[5], 0x59f111f1b605d019);
        f_64(&mut v, w[6], 0x923f82a4af194f9b);
        f_64(&mut v, w[7], 0xab1c5ed5da6d8118);
        f_64(&mut v, w[8], 0xd807aa98a3030242);
        f_64(&mut v, w[9], 0x12835b0145706fbe);
        f_64(&mut v, w[10], 0x243185be4ee4b28c);
        f_64(&mut v, w[11], 0x550c7dc3d5ffb4e2);
        f_64(&mut v, w[12], 0x72be5d74f27b896f);
        f_64(&mut v, w[13], 0x80deb1fe3b1696b1);
        f_64(&mut v, w[14], 0x9bdc06a725c71235);
        f_64(&mut v, w[15], 0xc19bf174cf692694);

        expand_64(&mut w);

        f_64(&mut v, w[0], 0xe49b69c19ef14ad2);
        f_64(&mut v, w[1], 0xefbe4786384f25e3);
        f_64(&mut v, w[2], 0x0fc19dc68b8cd5b5);
        f_64(&mut v, w[3], 0x240ca1cc77ac9c65);
        f_64(&mut v, w[4], 0x2de92c6f592b0275);
        f_64(&mut v, w[5], 0x4a7484aa6ea6e483);
        f_64(&mut v, w[6], 0x5cb0a9dcbd41fbd4);
        f_64(&mut v, w[7], 0x76f988da831153b5);
        f_64(&mut v, w[8], 0x983e5152ee66dfab);
        f_64(&mut v, w[9], 0xa831c66d2db43210);
        f_64(&mut v, w[10], 0xb00327c898fb213f);
        f_64(&mut v, w[11], 0xbf597fc7beef0ee4);
        f_64(&mut v, w[12], 0xc6e00bf33da88fc2);
        f_64(&mut v, w[13], 0xd5a79147930aa725);
        f_64(&mut v, w[14], 0x06ca6351e003826f);
        f_64(&mut v, w[15], 0x142929670a0e6e70);

        expand_64(&mut w);

        f_64(&mut v, w[0], 0x27b70a8546d22ffc);
        f_64(&mut v, w[1], 0x2e1b21385c26c926);
        f_64(&mut v, w[2], 0x4d2c6dfc5ac42aed);
        f_64(&mut v, w[3], 0x53380d139d95b3df);
        f_64(&mut v, w[4], 0x650a73548baf63de);
        f_64(&mut v, w[5], 0x766a0abb3c77b2a8);
        f_64(&mut v, w[6], 0x81c2c92e47edaee6);
        f_64(&mut v, w[7], 0x92722c851482353b);
        f_64(&mut v, w[8], 0xa2bfe8a14cf10364);
        f_64(&mut v, w[9], 0xa81a664bbc423001);
        f_64(&mut v, w[10], 0xc24b8b70d0f89791);
        f_64(&mut v, w[11], 0xc76c51a30654be30);
        f_64(&mut v, w[12], 0xd192e819d6ef5218);
        f_64(&mut v, w[13], 0xd69906245565a910);
        f_64(&mut v, w[14], 0xf40e35855771202a);
        f_64(&mut v, w[15], 0x106aa07032bbd1b8);

        expand_64(&mut w);

        f_64(&mut v, w[0], 0x19a4c116b8d2d0c8);
        f_64(&mut v, w[1], 0x1e376c085141ab53);
        f_64(&mut v, w[2], 0x2748774cdf8eeb99);
        f_64(&mut v, w[3], 0x34b0bcb5e19b48a8);
        f_64(&mut v, w[4], 0x391c0cb3c5c95a63);
        f_64(&mut v, w[5], 0x4ed8aa4ae3418acb);
        f_64(&mut v, w[6], 0x5b9cca4f7763e373);
        f_64(&mut v, w[7], 0x682e6ff3d6b2b8a3);
        f_64(&mut v, w[8], 0x748f82ee5defb2fc);
        f_64(&mut v, w[9], 0x78a5636f43172f60);
        f_64(&mut v, w[10], 0x84c87814a1f0ab72);
        f_64(&mut v, w[11], 0x8cc702081a6439ec);
        f_64(&mut v, w[12], 0x90befffa23631e28);
        f_64(&mut v, w[13], 0xa4506cebde82bde9);
        f_64(&mut v, w[14], 0xbef9a3f7b2c67915);
        f_64(&mut v, w[15], 0xc67178f2e372532b);

        expand_64(&mut w);

        f_64(&mut v, w[0], 0xca273eceea26619c);
        f_64(&mut v, w[1], 0xd186b8c721c0c207);
        f_64(&mut v, w[2], 0xeada7dd6cde0eb1e);
        f_64(&mut v, w[3], 0xf57d4f7fee6ed178);
        f_64(&mut v, w[4], 0x06f067aa72176fba);
        f_64(&mut v, w[5], 0x0a637dc5a2c898a6);
        f_64(&mut v, w[6], 0x113f9804bef90dae);
        f_64(&mut v, w[7], 0x1b710b35131c471b);
        f_64(&mut v, w[8], 0x28db77f523047d84);
        f_64(&mut v, w[9], 0x32caab7b40c72493);
        f_64(&mut v, w[10], 0x3c9ebe0a15c9bebc);
        f_64(&mut v, w[11], 0x431d67c49c100d4c);
        f_64(&mut v, w[12], 0x4cc5d4becb3e42b6);
        f_64(&mut v, w[13], 0x597f299cfc657e2a);
        f_64(&mut v, w[14], 0x5fcb6fab3ad6faec);
        f_64(&mut v, w[15], 0x6c44198c4a475817);

        v[0] = v[0].wrapping_add(state[0]);
        v[1] = v[1].wrapping_add(state[1]);
        v[2] = v[2].wrapping_add(state[2]);
        v[3] = v[3].wrapping_add(state[3]);
        v[4] = v[4].wrapping_add(state[4]);
        v[5] = v[5].wrapping_add(state[5]);
        v[6] = v[6].wrapping_add(state[6]);
        v[7] = v[7].wrapping_add(state[7]);

        state[0] = v[0];
        state[1] = v[1];
        state[2] = v[2];
        state[3] = v[3];
        state[4] = v[4];
        state[5] = v[5];
        state[6] = v[6];
        state[7] = v[7];

        in_ = in_.add(128);
        inlen -= 128;
    }

    store_bigendian_64(statebytes.add(0), state[0]);
    store_bigendian_64(statebytes.add(8), state[1]);
    store_bigendian_64(statebytes.add(16), state[2]);
    store_bigendian_64(statebytes.add(24), state[3]);
    store_bigendian_64(statebytes.add(32), state[4]);
    store_bigendian_64(statebytes.add(40), state[5]);
    store_bigendian_64(statebytes.add(48), state[6]);
    store_bigendian_64(statebytes.add(56), state[7]);

    /* `return inlen;` — truncated to `int` by the C prototype; the value is
     * always < 128 here and no caller uses it. */
    inlen as i32
}

// ---------------------------------------------------------------------------
// initial values
// ---------------------------------------------------------------------------

/// `static const uint8_t iv_256[32]`
static iv_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

/// `static const uint8_t iv_512[64]`
static iv_512: [u8; 64] = [
    0x6a, 0x09, 0xe6, 0x67, 0xf3, 0xbc, 0xc9, 0x08, 0xbb, 0x67, 0xae, 0x85, 0x84, 0xca, 0xa7, 0x3b,
    0x3c, 0x6e, 0xf3, 0x72, 0xfe, 0x94, 0xf8, 0x2b, 0xa5, 0x4f, 0xf5, 0x3a, 0x5f, 0x1d, 0x36, 0xf1,
    0x51, 0x0e, 0x52, 0x7f, 0xad, 0xe6, 0x82, 0xd1, 0x9b, 0x05, 0x68, 0x8c, 0x2b, 0x3e, 0x6c, 0x1f,
    0x1f, 0x83, 0xd9, 0xab, 0xfb, 0x41, 0xbd, 0x6b, 0x5b, 0xe0, 0xcd, 0x19, 0x13, 0x7e, 0x21, 0x79,
];

// ---------------------------------------------------------------------------
// public API
// ---------------------------------------------------------------------------

/// `void sha256_inc_init(uint8_t *state)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_init(state: *mut u8) {
    let mut i: usize = 0;
    while i < 32 {
        *state.add(i) = iv_256[i];
        i += 1;
    }
    let mut i: usize = 32;
    while i < 40 {
        *state.add(i) = 0;
        i += 1;
    }
}

/// `void sha512_inc_init(uint8_t *state)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_init(state: *mut u8) {
    let mut i: usize = 0;
    while i < 64 {
        *state.add(i) = iv_512[i];
        i += 1;
    }
    let mut i: usize = 64;
    while i < 72 {
        *state.add(i) = 0;
        i += 1;
    }
}

/// `void sha256_inc_blocks(uint8_t *state, const uint8_t *in, size_t inblocks)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_blocks(state: *mut u8, in_: *const u8, inblocks: usize) {
    let mut bytes: u64 = load_bigendian_64(state.add(32));

    crypto_hashblocks_sha256(state, in_, 64usize.wrapping_mul(inblocks));
    bytes = bytes.wrapping_add(64u64.wrapping_mul(inblocks as u64));

    store_bigendian_64(state.add(32), bytes);
}

/// `void sha512_inc_blocks(uint8_t *state, const uint8_t *in, size_t inblocks)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_blocks(state: *mut u8, in_: *const u8, inblocks: usize) {
    let mut bytes: u64 = load_bigendian_64(state.add(64));

    crypto_hashblocks_sha512(state, in_, 128u64.wrapping_mul(inblocks as u64));
    bytes = bytes.wrapping_add(128u64.wrapping_mul(inblocks as u64));

    store_bigendian_64(state.add(64), bytes);
}

/// `void sha256_inc_finalize(uint8_t *out, uint8_t *state, const uint8_t *in, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    in_: *const u8,
    inlen: usize,
) {
    let mut padded = [0u8; 128];
    let bytes: u64 = load_bigendian_64(state.add(32)).wrapping_add(inlen as u64);

    let mut in_ = in_;
    let mut inlen = inlen;

    crypto_hashblocks_sha256(state, in_, inlen);
    in_ = in_.add(inlen);
    inlen &= 63;
    in_ = in_.sub(inlen);

    let mut i: usize = 0;
    while i < inlen {
        padded[i] = *in_.add(i);
        i += 1;
    }
    padded[inlen] = 0x80;

    if inlen < 56 {
        let mut i: usize = inlen + 1;
        while i < 56 {
            padded[i] = 0;
            i += 1;
        }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, padded.as_ptr(), 64);
    } else {
        let mut i: usize = inlen + 1;
        while i < 120 {
            padded[i] = 0;
            i += 1;
        }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, padded.as_ptr(), 128);
    }

    let mut i: usize = 0;
    while i < 32 {
        *out.add(i) = *state.add(i);
        i += 1;
    }
}

/// `void sha512_inc_finalize(uint8_t *out, uint8_t *state, const uint8_t *in, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    in_: *const u8,
    inlen: usize,
) {
    let mut padded = [0u8; 256];
    let bytes: u64 = load_bigendian_64(state.add(64)).wrapping_add(inlen as u64);

    let mut in_ = in_;
    let mut inlen = inlen;

    crypto_hashblocks_sha512(state, in_, inlen as u64);
    in_ = in_.add(inlen);
    inlen &= 127;
    in_ = in_.sub(inlen);

    let mut i: usize = 0;
    while i < inlen {
        padded[i] = *in_.add(i);
        i += 1;
    }
    padded[inlen] = 0x80;

    if inlen < 112 {
        let mut i: usize = inlen + 1;
        while i < 119 {
            padded[i] = 0;
            i += 1;
        }
        padded[119] = (bytes >> 61) as u8;
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(state, padded.as_ptr(), 128);
    } else {
        let mut i: usize = inlen + 1;
        while i < 247 {
            padded[i] = 0;
            i += 1;
        }
        padded[247] = (bytes >> 61) as u8;
        padded[248] = (bytes >> 53) as u8;
        padded[249] = (bytes >> 45) as u8;
        padded[250] = (bytes >> 37) as u8;
        padded[251] = (bytes >> 29) as u8;
        padded[252] = (bytes >> 21) as u8;
        padded[253] = (bytes >> 13) as u8;
        padded[254] = (bytes >> 5) as u8;
        padded[255] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(state, padded.as_ptr(), 256);
    }

    let mut i: usize = 0;
    while i < 64 {
        *out.add(i) = *state.add(i);
        i += 1;
    }
}

/// `void sha256(uint8_t *out, const uint8_t *in, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256(out: *mut u8, in_: *const u8, inlen: usize) {
    let mut state = [0u8; 40];

    sha256_inc_init(state.as_mut_ptr());
    sha256_inc_finalize(out, state.as_mut_ptr(), in_, inlen);
}

/// `void sha512(uint8_t *out, const uint8_t *in, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512(out: *mut u8, in_: *const u8, inlen: usize) {
    let mut state = [0u8; 72];

    sha512_inc_init(state.as_mut_ptr());
    sha512_inc_finalize(out, state.as_mut_ptr(), in_, inlen);
}

/// mgf1 function based on the SHA-256 hash function
///
/// Note that inlen should be sufficiently small that it still allows for
/// an array to be allocated on the stack. Typically 'in' is merely a seed.
/// Outputs outlen number of bytes
///
/// ```c
/// void mgf1_256(unsigned char *out, unsigned long outlen,
///           const unsigned char *in, unsigned long inlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_256(out: *mut u8, outlen: u64, in_: *const u8, inlen: u64) {
    /* SPX_VLA(uint8_t, inbuf, inlen+4); */
    let mut inbuf = vec![0u8; (inlen as usize).wrapping_add(4)];
    let inbuf_ptr = inbuf.as_mut_ptr();
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    let mut out = out;

    core::ptr::copy_nonoverlapping(in_, inbuf_ptr, inlen as usize);

    /* While we can fit in at least another full block of SHA256 output.. */
    let mut i: u64 = 0;
    while i
        .wrapping_add(1)
        .wrapping_mul(SPX_SHA256_OUTPUT_BYTES as u64)
        <= outlen
    {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        sha256(
            out,
            inbuf_ptr as *const u8,
            (inlen as usize).wrapping_add(4),
        );
        out = out.add(SPX_SHA256_OUTPUT_BYTES);
        i = i.wrapping_add(1);
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i.wrapping_mul(SPX_SHA256_OUTPUT_BYTES as u64) {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        sha256(
            outbuf.as_mut_ptr(),
            inbuf_ptr as *const u8,
            (inlen as usize).wrapping_add(4),
        );
        core::ptr::copy_nonoverlapping(
            outbuf.as_ptr(),
            out,
            outlen.wrapping_sub(i.wrapping_mul(SPX_SHA256_OUTPUT_BYTES as u64)) as usize,
        );
    }
}

/// mgf1 function based on the SHA-512 hash function
///
/// ```c
/// void mgf1_512(unsigned char *out, unsigned long outlen,
///           const unsigned char *in, unsigned long inlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_512(out: *mut u8, outlen: u64, in_: *const u8, inlen: u64) {
    /* SPX_VLA(uint8_t, inbuf, inlen+4); */
    let mut inbuf = vec![0u8; (inlen as usize).wrapping_add(4)];
    let inbuf_ptr = inbuf.as_mut_ptr();
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];

    let mut out = out;

    core::ptr::copy_nonoverlapping(in_, inbuf_ptr, inlen as usize);

    /* While we can fit in at least another full block of SHA512 output.. */
    let mut i: u64 = 0;
    while i
        .wrapping_add(1)
        .wrapping_mul(SPX_SHA512_OUTPUT_BYTES as u64)
        <= outlen
    {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        sha512(
            out,
            inbuf_ptr as *const u8,
            (inlen as usize).wrapping_add(4),
        );
        out = out.add(SPX_SHA512_OUTPUT_BYTES);
        i = i.wrapping_add(1);
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i.wrapping_mul(SPX_SHA512_OUTPUT_BYTES as u64) {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        sha512(
            outbuf.as_mut_ptr(),
            inbuf_ptr as *const u8,
            (inlen as usize).wrapping_add(4),
        );
        core::ptr::copy_nonoverlapping(
            outbuf.as_ptr(),
            out,
            outlen.wrapping_sub(i.wrapping_mul(SPX_SHA512_OUTPUT_BYTES as u64)) as usize,
        );
    }
}

/// Absorb the constant pub_seed using one round of the compression function
/// This initializes state_seeded and state_seeded_512, which can then be
/// reused in thash
///
/// `void seed_state(spx_ctx *ctx)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    let mut i: usize;

    i = 0;
    while i < SPX_N {
        block[i] = (*ctx).pub_seed[i];
        i += 1;
    }
    i = SPX_N;
    while i < SPX_SHA512_BLOCK_BYTES {
        block[i] = 0;
        i += 1;
    }
    /* block has been properly initialized for both SHA-256 and SHA-512 */

    sha256_inc_init((*ctx).state_seeded.as_mut_ptr());
    sha256_inc_blocks((*ctx).state_seeded.as_mut_ptr(), block.as_ptr(), 1);
    /* `#if SPX_SHA512` */
    if crate::params::SPX_SHA512 {
        sha512_inc_init((*ctx).state_seeded_512.as_mut_ptr());
        sha512_inc_blocks((*ctx).state_seeded_512.as_mut_ptr(), block.as_ptr(), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        let mut s = String::new();
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    #[test]
    fn sha256_abc() {
        let msg = b"abc";
        let mut out = [0u8; 32];
        unsafe { sha256(out.as_mut_ptr(), msg.as_ptr(), msg.len()) };
        assert_eq!(
            hex(&out),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha512_abc() {
        let msg = b"abc";
        let mut out = [0u8; 64];
        unsafe { sha512(out.as_mut_ptr(), msg.as_ptr(), msg.len()) };
        assert_eq!(
            hex(&out),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    /// Empty input and a multi-block input (exercises both padding branches
    /// and the incremental block counter).
    #[test]
    fn sha256_empty_and_long() {
        let mut out = [0u8; 32];
        unsafe { sha256(out.as_mut_ptr(), [].as_ptr(), 0) };
        assert_eq!(
            hex(&out),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        /* 1_000_000 x 'a' is expensive; use the 112-byte "abcdbcde..." vector
         * from FIPS 180-2 instead, which needs the second padding branch. */
        let msg: &[u8] = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        unsafe { sha256(out.as_mut_ptr(), msg.as_ptr(), msg.len()) };
        assert_eq!(
            hex(&out),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha512_empty_and_long() {
        let mut out = [0u8; 64];
        unsafe { sha512(out.as_mut_ptr(), [].as_ptr(), 0) };
        assert_eq!(
            hex(&out),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce\
             47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );

        let msg: &[u8] = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn\
hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
        unsafe { sha512(out.as_mut_ptr(), msg.as_ptr(), msg.len()) };
        assert_eq!(
            hex(&out),
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018\
             501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
        );
    }

    /// Incremental API: `inc_blocks` + `inc_finalize` must equal one-shot.
    #[test]
    fn sha256_incremental_matches_oneshot() {
        let msg: [u8; 200] = core::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(1));

        let mut one = [0u8; 32];
        unsafe { sha256(one.as_mut_ptr(), msg.as_ptr(), msg.len()) };

        let mut state = [0u8; 40];
        let mut inc = [0u8; 32];
        unsafe {
            sha256_inc_init(state.as_mut_ptr());
            sha256_inc_blocks(state.as_mut_ptr(), msg.as_ptr(), 2); /* 128 bytes */
            sha256_inc_finalize(
                inc.as_mut_ptr(),
                state.as_mut_ptr(),
                msg.as_ptr().add(128),
                72,
            );
        }
        assert_eq!(hex(&one), hex(&inc));
    }

    #[test]
    fn sha512_incremental_matches_oneshot() {
        let msg: [u8; 400] = core::array::from_fn(|i| (i as u8).wrapping_mul(11).wrapping_add(3));

        let mut one = [0u8; 64];
        unsafe { sha512(one.as_mut_ptr(), msg.as_ptr(), msg.len()) };

        let mut state = [0u8; 72];
        let mut inc = [0u8; 64];
        unsafe {
            sha512_inc_init(state.as_mut_ptr());
            sha512_inc_blocks(state.as_mut_ptr(), msg.as_ptr(), 2); /* 256 bytes */
            sha512_inc_finalize(
                inc.as_mut_ptr(),
                state.as_mut_ptr(),
                msg.as_ptr().add(256),
                144,
            );
        }
        assert_eq!(hex(&one), hex(&inc));
    }

    /// mgf1_256 against the reference behaviour (concatenated sha256 of
    /// seed || counter), including a non-multiple-of-32 output length.
    #[test]
    fn mgf1_256_matches_manual() {
        let seed: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut out = [0u8; 70];
        unsafe { SPX_mgf1_256(out.as_mut_ptr(), 70, seed.as_ptr(), 8) };

        let mut expected = Vec::new();
        for ctr in 0u32..3 {
            let mut buf = [0u8; 12];
            buf[..8].copy_from_slice(&seed);
            buf[8..].copy_from_slice(&ctr.to_be_bytes());
            let mut h = [0u8; 32];
            unsafe { sha256(h.as_mut_ptr(), buf.as_ptr(), 12) };
            expected.extend_from_slice(&h);
        }
        assert_eq!(hex(&out), hex(&expected[..70]));
    }

    #[test]
    fn mgf1_512_matches_manual() {
        let seed: [u8; 16] = core::array::from_fn(|i| i as u8);
        let mut out = [0u8; 100];
        unsafe { SPX_mgf1_512(out.as_mut_ptr(), 100, seed.as_ptr(), 16) };

        let mut expected = Vec::new();
        for ctr in 0u32..2 {
            let mut buf = [0u8; 20];
            buf[..16].copy_from_slice(&seed);
            buf[16..].copy_from_slice(&ctr.to_be_bytes());
            let mut h = [0u8; 64];
            unsafe { sha512(h.as_mut_ptr(), buf.as_ptr(), 20) };
            expected.extend_from_slice(&h);
        }
        assert_eq!(hex(&out), hex(&expected[..100]));
    }
}
