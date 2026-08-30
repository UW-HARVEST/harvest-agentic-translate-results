//! Translation of `lib/sha2/src/sha2.c` and `lib/sha2/include/sha2.h`.
//!
//! Based on the public domain implementation in `crypto_hash/sha512/ref/` from
//! <http://bench.cr.yp.to/supercop.html> by D. J. Bernstein.

use core::ffi::c_ulong;

use crate::context::SpxCtx;
use crate::params::SPX_N;
use crate::sha2::{SPX_SHA512, SpxCtxSha512State};
use crate::utils::u32_to_bytes;

pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
/// This does not necessarily equal `SPX_N`.
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;

pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;

const _: () = assert!(
    SPX_SHA256_OUTPUT_BYTES >= SPX_N,
    "Linking against SHA-256 with N larger than 32 bytes is not supported"
);

pub const SPX_SHA256_ADDR_BYTES: usize = 22;

#[inline(always)]
fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

#[inline(always)]
fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64)
        | ((x[6] as u64) << 8)
        | ((x[5] as u64) << 16)
        | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32)
        | ((x[2] as u64) << 40)
        | ((x[1] as u64) << 48)
        | ((x[0] as u64) << 56)
}

#[inline(always)]
fn store_bigendian_32(x: &mut [u8], u: u64) {
    let mut u = u;
    x[3] = u as u8;
    u >>= 8;
    x[2] = u as u8;
    u >>= 8;
    x[1] = u as u8;
    u >>= 8;
    x[0] = u as u8;
}

#[inline(always)]
fn store_bigendian_64(x: &mut [u8], u: u64) {
    let mut u = u;
    for i in (0..8).rev() {
        x[i] = u as u8;
        u >>= 8;
    }
}

#[inline(always)]
fn ch32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}
#[inline(always)]
fn maj32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}
#[inline(always)]
fn big_sigma0_32(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}
#[inline(always)]
fn big_sigma1_32(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}
#[inline(always)]
fn sigma0_32(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}
#[inline(always)]
fn sigma1_32(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

#[inline(always)]
fn ch64(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (!x & z)
}
#[inline(always)]
fn maj64(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (x & z) ^ (y & z)
}
#[inline(always)]
fn big_sigma0_64(x: u64) -> u64 {
    x.rotate_right(28) ^ x.rotate_right(34) ^ x.rotate_right(39)
}
#[inline(always)]
fn big_sigma1_64(x: u64) -> u64 {
    x.rotate_right(14) ^ x.rotate_right(18) ^ x.rotate_right(41)
}
#[inline(always)]
fn sigma0_64(x: u64) -> u64 {
    x.rotate_right(1) ^ x.rotate_right(8) ^ (x >> 7)
}
#[inline(always)]
fn sigma1_64(x: u64) -> u64 {
    x.rotate_right(19) ^ x.rotate_right(61) ^ (x >> 6)
}

/// The round constants spelled out by the `F_32(w, k)` invocations in `sha2.c`.
#[rustfmt::skip]
static K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The round constants spelled out by the `F_64(w, k)` invocations in `sha2.c`.
#[rustfmt::skip]
static K512: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

fn crypto_hashblocks_sha256(statebytes: &mut [u8], inp: &[u8], inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[4 * i..]);
    }

    let mut inlen = inlen;
    let mut off = 0usize;

    while inlen >= 64 {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = load_bigendian_32(&inp[off + 4 * i..]);
        }
        for i in 16..64 {
            w[i] = sigma1_32(w[i - 2])
                .wrapping_add(w[i - 7])
                .wrapping_add(sigma0_32(w[i - 15]))
                .wrapping_add(w[i - 16]);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for i in 0..64 {
            let t1 = h
                .wrapping_add(big_sigma1_32(e))
                .wrapping_add(ch32(e, f, g))
                .wrapping_add(K256[i])
                .wrapping_add(w[i]);
            let t2 = big_sigma0_32(a).wrapping_add(maj32(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);

        off += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[4 * i..], state[i] as u64);
    }

    inlen
}

fn crypto_hashblocks_sha512(statebytes: &mut [u8], inp: &[u8], inlen: usize) -> usize {
    let mut state = [0u64; 8];
    for i in 0..8 {
        state[i] = load_bigendian_64(&statebytes[8 * i..]);
    }

    let mut inlen = inlen;
    let mut off = 0usize;

    while inlen >= 128 {
        let mut w = [0u64; 80];
        for i in 0..16 {
            w[i] = load_bigendian_64(&inp[off + 8 * i..]);
        }
        for i in 16..80 {
            w[i] = sigma1_64(w[i - 2])
                .wrapping_add(w[i - 7])
                .wrapping_add(sigma0_64(w[i - 15]))
                .wrapping_add(w[i - 16]);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for i in 0..80 {
            let t1 = h
                .wrapping_add(big_sigma1_64(e))
                .wrapping_add(ch64(e, f, g))
                .wrapping_add(K512[i])
                .wrapping_add(w[i]);
            let t2 = big_sigma0_64(a).wrapping_add(maj64(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);

        off += 128;
        inlen -= 128;
    }

    for i in 0..8 {
        store_bigendian_64(&mut statebytes[8 * i..], state[i]);
    }

    inlen
}

#[rustfmt::skip]
static IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85,
    0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c,
    0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

#[rustfmt::skip]
static IV_512: [u8; 64] = [
    0x6a, 0x09, 0xe6, 0x67, 0xf3, 0xbc, 0xc9, 0x08, 0xbb, 0x67, 0xae,
    0x85, 0x84, 0xca, 0xa7, 0x3b, 0x3c, 0x6e, 0xf3, 0x72, 0xfe, 0x94,
    0xf8, 0x2b, 0xa5, 0x4f, 0xf5, 0x3a, 0x5f, 0x1d, 0x36, 0xf1, 0x51,
    0x0e, 0x52, 0x7f, 0xad, 0xe6, 0x82, 0xd1, 0x9b, 0x05, 0x68, 0x8c,
    0x2b, 0x3e, 0x6c, 0x1f, 0x1f, 0x83, 0xd9, 0xab, 0xfb, 0x41, 0xbd,
    0x6b, 0x5b, 0xe0, 0xcd, 0x19, 0x13, 0x7e, 0x21, 0x79,
];

pub fn sha256_inc_init(state: &mut [u8]) {
    for i in 0..32 {
        state[i] = IV_256[i];
    }
    for i in 32..40 {
        state[i] = 0;
    }
}

pub fn sha512_inc_init(state: &mut [u8]) {
    for i in 0..64 {
        state[i] = IV_512[i];
    }
    for i in 64..72 {
        state[i] = 0;
    }
}

pub fn sha256_inc_blocks(state: &mut [u8], inp: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);

    crypto_hashblocks_sha256(state, inp, 64 * inblocks);
    bytes += (64 * inblocks) as u64;

    store_bigendian_64(&mut state[32..], bytes);
}

pub fn sha512_inc_blocks(state: &mut [u8], inp: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[64..]);

    crypto_hashblocks_sha512(state, inp, 128 * inblocks);
    bytes += (128 * inblocks) as u64;

    store_bigendian_64(&mut state[64..], bytes);
}

pub fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]).wrapping_add(inlen as u64);

    crypto_hashblocks_sha256(state, inp, inlen);
    // `in += inlen; inlen &= 63; in -= inlen;`
    let tail = inlen & 63;
    let tail_off = inlen - tail;
    let inlen = tail;

    for i in 0..inlen {
        padded[i] = inp[tail_off + i];
    }
    padded[inlen] = 0x80;

    if inlen < 56 {
        for i in inlen + 1..56 {
            padded[i] = 0;
        }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 64);
    } else {
        for i in inlen + 1..120 {
            padded[i] = 0;
        }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 128);
    }

    out[..32].copy_from_slice(&state[..32]);
}

pub fn sha512_inc_finalize(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 256];
    let bytes = load_bigendian_64(&state[64..]).wrapping_add(inlen as u64);

    crypto_hashblocks_sha512(state, inp, inlen);
    let tail = inlen & 127;
    let tail_off = inlen - tail;
    let inlen = tail;

    for i in 0..inlen {
        padded[i] = inp[tail_off + i];
    }
    padded[inlen] = 0x80;

    if inlen < 112 {
        for i in inlen + 1..119 {
            padded[i] = 0;
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
        crypto_hashblocks_sha512(state, &padded, 128);
    } else {
        for i in inlen + 1..247 {
            padded[i] = 0;
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
        crypto_hashblocks_sha512(state, &padded, 256);
    }

    out[..64].copy_from_slice(&state[..64]);
}

pub fn sha256(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut state = [0u8; 40];

    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, inp, inlen);
}

pub fn sha512(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut state = [0u8; 72];

    sha512_inc_init(&mut state);
    sha512_inc_finalize(out, &mut state, inp, inlen);
}

/// Upper bound for the `inlen + 4` VLA in the MGF1 routines: the largest `in`
/// they are called with is either `SPX_N + SPX_SHA256_ADDR_BYTES` (thash) or
/// `2 * SPX_N + SPX_SHAX_OUTPUT_BYTES` (hash_message).
const MGF1_INBUF_MAX: usize = {
    let a = SPX_N + SPX_SHA256_ADDR_BYTES;
    let b = 2 * SPX_N + SPX_SHA512_OUTPUT_BYTES;
    (if a > b { a } else { b }) + 4
};

/// MGF1 based on the SHA-256 hash function.
///
/// Note that `inlen` should be sufficiently small that it still allows for an
/// array to be allocated on the stack.  Typically `inp` is merely a seed.
pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut vla = crate::vla::Vla::<{ MGF1_INBUF_MAX }>::new(inlen + 4);
    let inbuf = vla.as_mut_slice();
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    /* While we can fit in at least another full block of SHA256 output.. */
    let mut i: usize = 0;
    let mut off: usize = 0;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        let mut tmp = [0u8; SPX_SHA256_OUTPUT_BYTES];
        sha256(&mut tmp, &inbuf[..inlen + 4], inlen + 4);
        out[off..off + SPX_SHA256_OUTPUT_BYTES].copy_from_slice(&tmp);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        sha256(&mut outbuf, &inbuf[..inlen + 4], inlen + 4);
        let rem = outlen - i * SPX_SHA256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

/// MGF1 based on the SHA-512 hash function.
pub fn mgf1_512(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut vla = crate::vla::Vla::<{ MGF1_INBUF_MAX }>::new(inlen + 4);
    let inbuf = vla.as_mut_slice();
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut off: usize = 0;
    while (i + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        let mut tmp = [0u8; SPX_SHA512_OUTPUT_BYTES];
        sha512(&mut tmp, &inbuf[..inlen + 4], inlen + 4);
        out[off..off + SPX_SHA512_OUTPUT_BYTES].copy_from_slice(&tmp);
        off += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA512_OUTPUT_BYTES {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        sha512(&mut outbuf, &inbuf[..inlen + 4], inlen + 4);
        let rem = outlen - i * SPX_SHA512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

/// Absorb the constant `pub_seed` using one round of the compression function.
///
/// This initializes `state_seeded` and `state_seeded_512`, which can then be
/// reused in `thash`.
pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];

    for i in 0..SPX_N {
        block[i] = ctx.pub_seed[i];
    }
    for i in SPX_N..SPX_SHA512_BLOCK_BYTES {
        block[i] = 0;
    }
    /* block has been properly initialized for both SHA-256 and SHA-512 */

    sha256_inc_init(&mut ctx.backend.state_seeded);
    sha256_inc_blocks(&mut ctx.backend.state_seeded, &block, 1);
    if SPX_SHA512 {
        let s: &mut SpxCtxSha512State = &mut ctx.backend.state_seeded_512;
        sha512_inc_init(s);
        sha512_inc_blocks(s, &block, 1);
    }
}

// ---------------------------------------------------------------------------
// C ABI.  Only `mgf1_*` and `seed_state` are renamed by `sha2.h`; the raw hash
// primitives keep their plain names, so those wrappers live in a submodule to
// avoid clashing with the safe functions above.
// ---------------------------------------------------------------------------

pub mod abi {

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha256_inc_init(state: *mut u8) {
        unsafe { super::sha256_inc_init(core::slice::from_raw_parts_mut(state, 40)) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha256_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
        unsafe {
            super::sha256_inc_blocks(
                core::slice::from_raw_parts_mut(state, 40),
                core::slice::from_raw_parts(inp, 64 * inblocks),
                inblocks,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha256_inc_finalize(
        out: *mut u8,
        state: *mut u8,
        inp: *const u8,
        inlen: usize,
    ) {
        unsafe {
            super::sha256_inc_finalize(
                core::slice::from_raw_parts_mut(out, 32),
                core::slice::from_raw_parts_mut(state, 40),
                core::slice::from_raw_parts(inp, inlen),
                inlen,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha256(out: *mut u8, inp: *const u8, inlen: usize) {
        unsafe {
            super::sha256(
                core::slice::from_raw_parts_mut(out, 32),
                core::slice::from_raw_parts(inp, inlen),
                inlen,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha512_inc_init(state: *mut u8) {
        unsafe { super::sha512_inc_init(core::slice::from_raw_parts_mut(state, 72)) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha512_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
        unsafe {
            super::sha512_inc_blocks(
                core::slice::from_raw_parts_mut(state, 72),
                core::slice::from_raw_parts(inp, 128 * inblocks),
                inblocks,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha512_inc_finalize(
        out: *mut u8,
        state: *mut u8,
        inp: *const u8,
        inlen: usize,
    ) {
        unsafe {
            super::sha512_inc_finalize(
                core::slice::from_raw_parts_mut(out, 64),
                core::slice::from_raw_parts_mut(state, 72),
                core::slice::from_raw_parts(inp, inlen),
                inlen,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn sha512(out: *mut u8, inp: *const u8, inlen: usize) {
        unsafe {
            super::sha512(
                core::slice::from_raw_parts_mut(out, 64),
                core::slice::from_raw_parts(inp, inlen),
                inlen,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_256(
    out: *mut u8,
    outlen: c_ulong,
    inp: *const u8,
    inlen: c_ulong,
) {
    unsafe {
        mgf1_256(
            core::slice::from_raw_parts_mut(out, outlen as usize),
            outlen as usize,
            core::slice::from_raw_parts(inp, inlen as usize),
            inlen as usize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_512(
    out: *mut u8,
    outlen: c_ulong,
    inp: *const u8,
    inlen: c_ulong,
) {
    unsafe {
        mgf1_512(
            core::slice::from_raw_parts_mut(out, outlen as usize),
            outlen as usize,
            core::slice::from_raw_parts(inp, inlen as usize),
            inlen as usize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    unsafe { seed_state(&mut *ctx) }
}
