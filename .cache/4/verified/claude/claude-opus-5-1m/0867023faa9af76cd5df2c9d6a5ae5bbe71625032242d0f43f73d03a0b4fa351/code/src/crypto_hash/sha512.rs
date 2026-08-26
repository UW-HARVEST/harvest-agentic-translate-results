//! Translation of `crypto_hash/sha512/hash_sha512.c` and
//! `crypto_hash/sha512/cp/hash_sha512_cp.c`.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};
use core::ptr::addr_of_mut;

use crate::common::{load64_be, memset, rotr64, store64_be};
use crate::sodium::utils::sodium_memzero;

/// `crypto_hash_sha512_BYTES`
pub const crypto_hash_sha512_BYTES: usize = 64;

/// ```c
/// typedef struct crypto_hash_sha512_state {
///     uint64_t state[8];
///     uint64_t count[2];
///     uint8_t  buf[128];
/// } crypto_hash_sha512_state;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

// ---------------------------------------------------------------------------
// hash_sha512_cp.c
// ---------------------------------------------------------------------------

/// `static void be64enc_vect(unsigned char *dst, const uint64_t *src, size_t len)`
unsafe fn be64enc_vect(dst: *mut u8, src: *const u64, len: usize) {
    let mut i: usize = 0;

    while i < len / 8 {
        store64_be(dst.add(i * 8), *src.add(i));
        i += 1;
    }
}

/// `static void be64dec_vect(uint64_t *dst, const unsigned char *src, size_t len)`
unsafe fn be64dec_vect(dst: *mut u64, src: *const u8, len: usize) {
    let mut i: usize = 0;

    while i < len / 8 {
        *dst.add(i) = load64_be(src.add(i * 8));
        i += 1;
    }
}

static Krnd: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

#[inline(always)]
fn Ch(x: u64, y: u64, z: u64) -> u64 {
    (x & (y ^ z)) ^ z
}

#[inline(always)]
fn Maj(x: u64, y: u64, z: u64) -> u64 {
    (x & (y | z)) | (y & z)
}

#[inline(always)]
fn S0(x: u64) -> u64 {
    rotr64(x, 28) ^ rotr64(x, 34) ^ rotr64(x, 39)
}

#[inline(always)]
fn S1(x: u64) -> u64 {
    rotr64(x, 14) ^ rotr64(x, 18) ^ rotr64(x, 41)
}

#[inline(always)]
fn s0(x: u64) -> u64 {
    rotr64(x, 1) ^ rotr64(x, 8) ^ (x >> 7)
}

#[inline(always)]
fn s1(x: u64) -> u64 {
    rotr64(x, 19) ^ rotr64(x, 61) ^ (x >> 6)
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
    S: &mut [u64; 8],
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    g: usize,
    h: usize,
    k: u64,
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

/// `static void SHA512_Transform(uint64_t *state, const uint8_t block[128],
///                               uint64_t W[80], uint64_t S[8])`
unsafe fn SHA512_Transform(state: *mut u64, block: *const u8, W: &mut [u64; 80], S: &mut [u64; 8]) {
    let mut i: usize;

    be64dec_vect(W.as_mut_ptr(), block, 128);
    /* memcpy(S, state, 64) */
    for j in 0..8 {
        S[j] = *state.add(j);
    }
    i = 0;
    while i < 80 {
        /* RNDr(S, W, 0..15, i) */
        for ii in 0..16usize {
            RND(
                S,
                (80 - ii) % 8,
                (81 - ii) % 8,
                (82 - ii) % 8,
                (83 - ii) % 8,
                (84 - ii) % 8,
                (85 - ii) % 8,
                (86 - ii) % 8,
                (87 - ii) % 8,
                W[ii + i].wrapping_add(Krnd[ii + i]),
            );
        }
        if i == 64 {
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

/// `static const uint8_t PAD[128] = { 0x80, 0, 0, ... };`
static PAD: [u8; 128] = {
    let mut pad = [0u8; 128];
    pad[0] = 0x80;
    pad
};

/// `static void SHA512_Pad(crypto_hash_sha512_state *state, uint64_t tmp64[80 + 8])`
unsafe fn SHA512_Pad(state: *mut crypto_hash_sha512_state, W: &mut [u64; 80], S: &mut [u64; 8]) {
    let r: u32;
    let mut i: u32;

    let statep = addr_of_mut!((*state).state) as *mut u64;
    let countp = addr_of_mut!((*state).count) as *mut u64;
    let bufp = addr_of_mut!((*state).buf) as *mut u8;

    /* ACQUIRE_FENCE is a no-op in this configuration */
    r = ((*countp.add(1) >> 3) & 0x7f) as u32;
    if r < 112 {
        i = 0;
        while i < 112 - r {
            *bufp.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
    } else {
        i = 0;
        while i < 128 - r {
            *bufp.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
        SHA512_Transform(statep, bufp, W, S);
        memset(bufp, 0, 112);
    }
    be64enc_vect(bufp.add(112), countp as *const u64, 16);
    SHA512_Transform(statep, bufp, W, S);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int {
    static sha512_initial_state: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];

    let statep = addr_of_mut!((*state).state) as *mut u64;
    let countp = addr_of_mut!((*state).count) as *mut u64;

    *countp.add(1) = 0u64;
    *countp.add(0) = *countp.add(1);
    /* memcpy(state->state, sha512_initial_state, sizeof sha512_initial_state) */
    for i in 0..8 {
        *statep.add(i) = sha512_initial_state[i];
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_update(
    state: *mut crypto_hash_sha512_state,
    mut in_: *const u8,
    mut inlen: u64,
) -> c_int {
    let mut tmp64_W = [0u64; 80];
    let mut tmp64_S = [0u64; 8];
    let mut bitlen = [0u64; 2];
    let mut i: u64;
    let r: u64;

    if inlen == 0 {
        return 0;
    }
    /* ACQUIRE_FENCE is a no-op in this configuration */
    let statep = addr_of_mut!((*state).state) as *mut u64;
    let countp = addr_of_mut!((*state).count) as *mut u64;
    let bufp = addr_of_mut!((*state).buf) as *mut u8;

    r = (*countp.add(1) >> 3) & 0x7f;

    bitlen[1] = inlen << 3;
    bitlen[0] = inlen >> 61;
    /* LCOV_EXCL_START */
    *countp.add(1) = (*countp.add(1)).wrapping_add(bitlen[1]);
    if *countp.add(1) < bitlen[1] {
        *countp.add(0) = (*countp.add(0)).wrapping_add(1);
    }
    /* LCOV_EXCL_STOP */
    *countp.add(0) = (*countp.add(0)).wrapping_add(bitlen[0]);
    if inlen < 128 - r {
        i = 0;
        while i < inlen {
            *bufp.add((r + i) as usize) = *in_.add(i as usize);
            i += 1;
        }
        return 0;
    }
    i = 0;
    while i < 128 - r {
        *bufp.add((r + i) as usize) = *in_.add(i as usize);
        i += 1;
    }
    SHA512_Transform(statep, bufp, &mut tmp64_W, &mut tmp64_S);
    in_ = in_.add((128 - r) as usize);
    inlen = inlen.wrapping_sub(128 - r);

    while inlen >= 128 {
        SHA512_Transform(statep, in_, &mut tmp64_W, &mut tmp64_S);
        in_ = in_.add(128);
        inlen -= 128;
    }
    inlen &= 127;
    i = 0;
    while i < inlen {
        *bufp.add(i as usize) = *in_.add(i as usize);
        i += 1;
    }
    sodium_memzero(tmp64_W.as_mut_ptr() as *mut c_void, 80 * 8);
    sodium_memzero(tmp64_S.as_mut_ptr() as *mut c_void, 8 * 8);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_final(
    state: *mut crypto_hash_sha512_state,
    out: *mut u8,
) -> c_int {
    let mut tmp64_W = [0u64; 80];
    let mut tmp64_S = [0u64; 8];

    SHA512_Pad(state, &mut tmp64_W, &mut tmp64_S);
    be64enc_vect(out, addr_of_mut!((*state).state) as *const u64, 64);
    sodium_memzero(tmp64_W.as_mut_ptr() as *mut c_void, 80 * 8);
    sodium_memzero(tmp64_S.as_mut_ptr() as *mut c_void, 8 * 8);
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_hash_sha512_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int {
    let mut state = crypto_hash_sha512_state {
        state: [0u64; 8],
        count: [0u64; 2],
        buf: [0u8; 128],
    };

    crypto_hash_sha512_init(&mut state);
    crypto_hash_sha512_update(&mut state, in_, inlen);
    crypto_hash_sha512_final(&mut state, out);

    0
}

// ---------------------------------------------------------------------------
// hash_sha512.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_bytes() -> usize {
    crypto_hash_sha512_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha512_state>()
}
