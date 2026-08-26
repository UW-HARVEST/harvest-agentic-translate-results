//! Translation of `crypto_hash/sha256/hash_sha256.c` and
//! `crypto_hash/sha256/cp/hash_sha256_cp.c`.
//!
//! The reference build defines neither `__aarch64__`/`__ARM_FEATURE_SHA2` nor
//! `NATIVE_LITTLE_ENDIAN`, so the portable (`be32*_vect` + `SHA256_Transform`)
//! branch is the one translated here.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};
use core::ptr::addr_of_mut;

use crate::common::{load32_be, memset, rotr32, store32_be, store64_be};
use crate::sodium::utils::sodium_memzero;

/// `crypto_hash_sha256_BYTES`
pub const crypto_hash_sha256_BYTES: usize = 32;

/// ```c
/// typedef struct crypto_hash_sha256_state {
///     uint32_t state[8];
///     uint64_t count;
///     uint8_t  buf[64];
/// } crypto_hash_sha256_state;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

// ---------------------------------------------------------------------------
// hash_sha256_cp.c
// ---------------------------------------------------------------------------

/// `static void be32enc_vect(unsigned char *dst, const uint32_t *src, size_t len)`
unsafe fn be32enc_vect(dst: *mut u8, src: *const u32, len: usize) {
    let mut i: usize = 0;

    while i < len / 4 {
        store32_be(dst.add(i * 4), *src.add(i));
        i += 1;
    }
}

/// `static void be32dec_vect(uint32_t *dst, const unsigned char *src, size_t len)`
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

#[inline(always)]
fn Ch(x: u32, y: u32, z: u32) -> u32 {
    (x & (y ^ z)) ^ z
}

#[inline(always)]
fn Maj(x: u32, y: u32, z: u32) -> u32 {
    (x & (y | z)) | (y & z)
}

#[inline(always)]
fn S0(x: u32) -> u32 {
    rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22)
}

#[inline(always)]
fn S1(x: u32) -> u32 {
    rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25)
}

#[inline(always)]
fn s0(x: u32) -> u32 {
    rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3)
}

#[inline(always)]
fn s1(x: u32) -> u32 {
    rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10)
}

/// ```c
/// #define RND(a, b, c, d, e, f, g, h, k) \
///     h += S1(e) + Ch(e, f, g) + k;      \
///     d += h;                            \
///     h += S0(a) + Maj(a, b, c);
/// ```
///
/// The macro is always instantiated with distinct elements of `S`, so the
/// operands are passed here as indices into `S`.
#[inline(always)]
fn RND(
    S: &mut [u32; 8],
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
    let hv = S[h]
        .wrapping_add(S1(S[e]))
        .wrapping_add(Ch(S[e], S[f], S[g]))
        .wrapping_add(k);
    S[h] = hv;
    S[d] = S[d].wrapping_add(hv);
    S[h] = S[h]
        .wrapping_add(S0(S[a]))
        .wrapping_add(Maj(S[a], S[b], S[c]));
}

/// `static void SHA256_Transform(uint32_t state[8], const uint8_t block[64],
///                               uint32_t W[64], uint32_t S[8])`
unsafe fn SHA256_Transform(state: *mut u32, block: *const u8, W: &mut [u32; 64], S: &mut [u32; 8]) {
    let mut i: usize;

    be32dec_vect(W.as_mut_ptr(), block, 64);
    /* memcpy(S, state, 32) */
    for j in 0..8 {
        S[j] = *state.add(j);
    }
    i = 0;
    while i < 64 {
        /* RNDr(S, W, 0..15, i) */
        for ii in 0..16usize {
            RND(
                S,
                (64 - ii) % 8,
                (65 - ii) % 8,
                (66 - ii) % 8,
                (67 - ii) % 8,
                (68 - ii) % 8,
                (69 - ii) % 8,
                (70 - ii) % 8,
                (71 - ii) % 8,
                W[ii + i].wrapping_add(Krnd[ii + i]),
            );
        }
        if i == 48 {
            break;
        }
        /* MSCH(W, 0..15, i) */
        for ii in 0..16usize {
            W[i + ii + 16] = s1(W[i + ii + 14])
                .wrapping_add(W[i + ii + 9])
                .wrapping_add(s0(W[i + ii + 1]))
                .wrapping_add(W[i + ii]);
        }
        i += 16;
    }
    for j in 0..8 {
        *state.add(j) = (*state.add(j)).wrapping_add(S[j]);
    }
}

/// `static const uint8_t PAD[64] = { 0x80, 0, 0, ... };`
static PAD: [u8; 64] = {
    let mut pad = [0u8; 64];
    pad[0] = 0x80;
    pad
};

/// `static void SHA256_Pad(crypto_hash_sha256_state *state, uint32_t tmp32[64 + 8])`
unsafe fn SHA256_Pad(
    state: *mut crypto_hash_sha256_state,
    W: &mut [u32; 64],
    S: &mut [u32; 8],
) {
    let r: u32;
    let mut i: u32;

    let statep = addr_of_mut!((*state).state) as *mut u32;
    let countp = addr_of_mut!((*state).count);
    let bufp = addr_of_mut!((*state).buf) as *mut u8;

    /* ACQUIRE_FENCE is a no-op in this configuration */
    r = ((*countp >> 3) & 0x3f) as u32;
    if r < 56 {
        i = 0;
        while i < 56 - r {
            *bufp.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
    } else {
        i = 0;
        while i < 64 - r {
            *bufp.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
        SHA256_Transform(statep, bufp, W, S);
        memset(bufp, 0, 56);
    }
    store64_be(bufp.add(56), *countp);
    SHA256_Transform(statep, bufp, W, S);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int {
    static sha256_initial_state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    *addr_of_mut!((*state).count) = 0u64;
    /* memcpy(state->state, sha256_initial_state, sizeof sha256_initial_state) */
    let statep = addr_of_mut!((*state).state) as *mut u32;
    for i in 0..8 {
        *statep.add(i) = sha256_initial_state[i];
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_update(
    state: *mut crypto_hash_sha256_state,
    mut in_: *const u8,
    mut inlen: u64,
) -> c_int {
    let mut tmp32_W = [0u32; 64];
    let mut tmp32_S = [0u32; 8];
    let mut i: u64;
    let r: u64;

    if inlen == 0 {
        return 0;
    }
    /* ACQUIRE_FENCE is a no-op in this configuration */
    let statep = addr_of_mut!((*state).state) as *mut u32;
    let countp = addr_of_mut!((*state).count);
    let bufp = addr_of_mut!((*state).buf) as *mut u8;

    r = (*countp >> 3) & 0x3f;

    *countp = (*countp).wrapping_add(inlen << 3);
    if inlen < 64 - r {
        i = 0;
        while i < inlen {
            *bufp.add((r + i) as usize) = *in_.add(i as usize);
            i += 1;
        }
        return 0;
    }
    i = 0;
    while i < 64 - r {
        *bufp.add((r + i) as usize) = *in_.add(i as usize);
        i += 1;
    }
    SHA256_Transform(statep, bufp, &mut tmp32_W, &mut tmp32_S);
    in_ = in_.add((64 - r) as usize);
    inlen = inlen.wrapping_sub(64 - r);

    while inlen >= 64 {
        SHA256_Transform(statep, in_, &mut tmp32_W, &mut tmp32_S);
        in_ = in_.add(64);
        inlen -= 64;
    }
    inlen &= 63;
    i = 0;
    while i < inlen {
        *bufp.add(i as usize) = *in_.add(i as usize);
        i += 1;
    }
    sodium_memzero(tmp32_W.as_mut_ptr() as *mut c_void, 64 * 4);
    sodium_memzero(tmp32_S.as_mut_ptr() as *mut c_void, 8 * 4);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_final(
    state: *mut crypto_hash_sha256_state,
    out: *mut u8,
) -> c_int {
    let mut tmp32_W = [0u32; 64];
    let mut tmp32_S = [0u32; 8];

    SHA256_Pad(state, &mut tmp32_W, &mut tmp32_S);
    be32enc_vect(out, addr_of_mut!((*state).state) as *const u32, 32);
    sodium_memzero(tmp32_W.as_mut_ptr() as *mut c_void, 64 * 4);
    sodium_memzero(tmp32_S.as_mut_ptr() as *mut c_void, 8 * 4);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_hash_sha256_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256(out: *mut u8, in_: *const u8, inlen: u64) -> c_int {
    let mut state = crypto_hash_sha256_state {
        state: [0u32; 8],
        count: 0,
        buf: [0u8; 64],
    };

    crypto_hash_sha256_init(&mut state);
    crypto_hash_sha256_update(&mut state, in_, inlen);
    crypto_hash_sha256_final(&mut state, out);

    0
}

// ---------------------------------------------------------------------------
// hash_sha256.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_bytes() -> usize {
    crypto_hash_sha256_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha256_state>()
}
