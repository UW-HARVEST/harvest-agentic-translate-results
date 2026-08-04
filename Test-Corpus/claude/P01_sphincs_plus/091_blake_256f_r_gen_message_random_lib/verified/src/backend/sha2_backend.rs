// SHA2 backend - direct translation from c_src/lib/sha2.

#![allow(non_snake_case)]

use crate::address::addr_to_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::{bytes_to_ull_rs, u32_to_bytes_rs};

const SPX_SHA256_BLOCK_BYTES: usize = 64;
const SPX_SHA256_OUTPUT_BYTES: usize = 32;
const SPX_SHA512_BLOCK_BYTES: usize = 128;
const SPX_SHA512_OUTPUT_BYTES: usize = 64;
const SPX_SHA256_ADDR_BYTES: usize = 22;

// =========================
// Pure-Rust SHA-256 / SHA-512 matching crypto_hashblocks behavior
// =========================

fn load_be32(x: &[u8]) -> u32 {
    ((x[0] as u32) << 24) | ((x[1] as u32) << 16) | ((x[2] as u32) << 8) | (x[3] as u32)
}
fn load_be64(x: &[u8]) -> u64 {
    let mut r: u64 = 0;
    for i in 0..8 {
        r |= (x[i] as u64) << (56 - 8 * i);
    }
    r
}
fn store_be32(x: &mut [u8], v: u32) {
    x[0] = (v >> 24) as u8;
    x[1] = (v >> 16) as u8;
    x[2] = (v >> 8) as u8;
    x[3] = v as u8;
}
fn store_be64(x: &mut [u8], v: u64) {
    for i in 0..8 {
        x[i] = (v >> (56 - 8 * i)) as u8;
    }
}

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const K512: [u64; 80] = [
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

fn crypto_hashblocks_sha256(state: &mut [u8; 32], inp: &[u8]) {
    let mut s: [u32; 8] = [0; 8];
    for i in 0..8 {
        s[i] = load_be32(&state[4 * i..]);
    }
    let mut idx = 0;
    while idx + 64 <= inp.len() {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = load_be32(&inp[idx + 4 * i..]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = s[0];
        let mut b = s[1];
        let mut c = s[2];
        let mut d = s[3];
        let mut e = s[4];
        let mut f = s[5];
        let mut g = s[6];
        let mut h = s[7];
        for i in 0..64 {
            let big_s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(big_s1)
                .wrapping_add(ch)
                .wrapping_add(K256[i])
                .wrapping_add(w[i]);
            let big_s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = big_s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        s[0] = s[0].wrapping_add(a);
        s[1] = s[1].wrapping_add(b);
        s[2] = s[2].wrapping_add(c);
        s[3] = s[3].wrapping_add(d);
        s[4] = s[4].wrapping_add(e);
        s[5] = s[5].wrapping_add(f);
        s[6] = s[6].wrapping_add(g);
        s[7] = s[7].wrapping_add(h);
        idx += 64;
    }
    for i in 0..8 {
        store_be32(&mut state[4 * i..], s[i]);
    }
}

fn crypto_hashblocks_sha512(state: &mut [u8; 64], inp: &[u8]) {
    let mut s: [u64; 8] = [0; 8];
    for i in 0..8 {
        s[i] = load_be64(&state[8 * i..]);
    }
    let mut idx = 0;
    while idx + 128 <= inp.len() {
        let mut w = [0u64; 80];
        for i in 0..16 {
            w[i] = load_be64(&inp[idx + 8 * i..]);
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = s[0];
        let mut b = s[1];
        let mut c = s[2];
        let mut d = s[3];
        let mut e = s[4];
        let mut f = s[5];
        let mut g = s[6];
        let mut h = s[7];
        for i in 0..80 {
            let big_s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(big_s1)
                .wrapping_add(ch)
                .wrapping_add(K512[i])
                .wrapping_add(w[i]);
            let big_s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = big_s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        s[0] = s[0].wrapping_add(a);
        s[1] = s[1].wrapping_add(b);
        s[2] = s[2].wrapping_add(c);
        s[3] = s[3].wrapping_add(d);
        s[4] = s[4].wrapping_add(e);
        s[5] = s[5].wrapping_add(f);
        s[6] = s[6].wrapping_add(g);
        s[7] = s[7].wrapping_add(h);
        idx += 128;
    }
    for i in 0..8 {
        store_be64(&mut state[8 * i..], s[i]);
    }
}

const IV256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

const IV512: [u8; 64] = [
    0x6a, 0x09, 0xe6, 0x67, 0xf3, 0xbc, 0xc9, 0x08, 0xbb, 0x67, 0xae, 0x85, 0x84, 0xca, 0xa7, 0x3b,
    0x3c, 0x6e, 0xf3, 0x72, 0xfe, 0x94, 0xf8, 0x2b, 0xa5, 0x4f, 0xf5, 0x3a, 0x5f, 0x1d, 0x36, 0xf1,
    0x51, 0x0e, 0x52, 0x7f, 0xad, 0xe6, 0x82, 0xd1, 0x9b, 0x05, 0x68, 0x8c, 0x2b, 0x3e, 0x6c, 0x1f,
    0x1f, 0x83, 0xd9, 0xab, 0xfb, 0x41, 0xbd, 0x6b, 0x5b, 0xe0, 0xcd, 0x19, 0x13, 0x7e, 0x21, 0x79,
];

pub fn sha256_inc_init(state: &mut [u8; 40]) {
    state[..32].copy_from_slice(&IV256);
    for i in 32..40 {
        state[i] = 0;
    }
}

pub fn sha512_inc_init(state: &mut [u8; 72]) {
    state[..64].copy_from_slice(&IV512);
    for i in 64..72 {
        state[i] = 0;
    }
}

pub fn sha256_inc_blocks(state: &mut [u8; 40], inp: &[u8], inblocks: usize) {
    let mut bytes = load_be64(&state[32..40]);
    {
        let st32: &mut [u8; 32] = (&mut state[..32]).try_into().unwrap();
        crypto_hashblocks_sha256(st32, &inp[..64 * inblocks]);
    }
    bytes += (64 * inblocks) as u64;
    store_be64(&mut state[32..40], bytes);
}

pub fn sha512_inc_blocks(state: &mut [u8; 72], inp: &[u8], inblocks: usize) {
    let mut bytes = load_be64(&state[64..72]);
    {
        let st64: &mut [u8; 64] = (&mut state[..64]).try_into().unwrap();
        crypto_hashblocks_sha512(st64, &inp[..128 * inblocks]);
    }
    bytes += (128 * inblocks) as u64;
    store_be64(&mut state[64..72], bytes);
}

pub fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8; 40], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_be64(&state[32..40]) + inlen as u64;
    {
        let st32: &mut [u8; 32] = (&mut state[..32]).try_into().unwrap();
        crypto_hashblocks_sha256(st32, &inp[..inlen]);
    }
    let leftover = inlen & 63;
    let inp = &inp[inlen - leftover..];
    let inlen = leftover;

    for i in 0..inlen {
        padded[i] = inp[i];
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
        let st32: &mut [u8; 32] = (&mut state[..32]).try_into().unwrap();
        crypto_hashblocks_sha256(st32, &padded[..64]);
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
        let st32: &mut [u8; 32] = (&mut state[..32]).try_into().unwrap();
        crypto_hashblocks_sha256(st32, &padded[..128]);
    }
    out[..32].copy_from_slice(&state[..32]);
}

pub fn sha512_inc_finalize(out: &mut [u8], state: &mut [u8; 72], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 256];
    let bytes = load_be64(&state[64..72]) + inlen as u64;
    {
        let st64: &mut [u8; 64] = (&mut state[..64]).try_into().unwrap();
        crypto_hashblocks_sha512(st64, &inp[..inlen]);
    }
    let leftover = inlen & 127;
    let inp = &inp[inlen - leftover..];
    let inlen = leftover;
    for i in 0..inlen {
        padded[i] = inp[i];
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
        let st64: &mut [u8; 64] = (&mut state[..64]).try_into().unwrap();
        crypto_hashblocks_sha512(st64, &padded[..128]);
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
        let st64: &mut [u8; 64] = (&mut state[..64]).try_into().unwrap();
        crypto_hashblocks_sha512(st64, &padded[..256]);
    }
    out[..64].copy_from_slice(&state[..64]);
}

pub fn sha256(out: &mut [u8], inp: &[u8]) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, inp, inp.len());
}

pub fn sha512(out: &mut [u8], inp: &[u8]) {
    let mut state = [0u8; 72];
    sha512_inc_init(&mut state);
    sha512_inc_finalize(out, &mut state, inp, inp.len());
}

pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8]) {
    let inlen = inp.len();
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(inp);
    let mut i: usize = 0;
    let mut written = 0;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        sha256(&mut out[written..written + 32], &inbuf);
        written += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        sha256(&mut outbuf, &inbuf);
        let rem = outlen - i * SPX_SHA256_OUTPUT_BYTES;
        out[written..written + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub fn mgf1_512(out: &mut [u8], outlen: usize, inp: &[u8]) {
    let inlen = inp.len();
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(inp);
    let mut i: usize = 0;
    let mut written = 0;
    while (i + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        sha512(&mut out[written..written + 64], &inbuf);
        written += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA512_OUTPUT_BYTES {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        sha512(&mut outbuf, &inbuf);
        let rem = outlen - i * SPX_SHA512_OUTPUT_BYTES;
        out[written..written + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// =========================
// SPHINCS+ hash-related functions
// =========================

pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        sha512_inc_init(&mut ctx.state_seeded_512);
        sha512_inc_blocks(&mut ctx.state_seeded_512, &block, 1);
    }
}

pub fn initialize_hash_function_impl(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr_impl(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = ctx.state_seeded;
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let addr_bytes = addr_to_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf.len());
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;

pub fn gen_message_random_impl(
    R: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut buf = vec![0u8; SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES];

    // HMAC-SHA outer key prep
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    let mut state = vec![0u8; 8 + SHAX_OUTPUT_BYTES];
    init_shaX(&mut state);
    shaX_inc_blocks_state(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    let mlen = m.len();
    if SPX_N + mlen < SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(m);
        let (head, tail) = buf.split_at_mut(SHAX_BLOCK_BYTES);
        // tail starts at SHAX_BLOCK_BYTES, length SHAX_OUTPUT_BYTES
        shaX_inc_finalize_state(tail, &mut state, &head[..SPX_N + mlen], SPX_N + mlen);
    } else {
        let block_left = SHAX_BLOCK_BYTES - SPX_N;
        buf[SPX_N..SPX_N + block_left].copy_from_slice(&m[..block_left]);
        // Use a separate copy of buf for the absorb
        let mut input_block = vec![0u8; SHAX_BLOCK_BYTES];
        input_block.copy_from_slice(&buf[..SHAX_BLOCK_BYTES]);
        shaX_inc_blocks_state(&mut state, &input_block, 1);

        let m_rest = &m[block_left..];
        let mlen_rest = mlen - block_left;
        let (_head, tail) = buf.split_at_mut(SHAX_BLOCK_BYTES);
        shaX_inc_finalize_state(tail, &mut state, m_rest, mlen_rest);
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }
    let mut tmp = vec![0u8; SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES];
    tmp.copy_from_slice(&buf);
    shaX(&mut buf, &tmp);
    R[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

fn init_shaX(state: &mut [u8]) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let st: &mut [u8; 72] = (&mut state[..72]).try_into().unwrap();
        sha512_inc_init(st);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let st: &mut [u8; 40] = (&mut state[..40]).try_into().unwrap();
        sha256_inc_init(st);
    }
}

fn shaX_inc_blocks_state(state: &mut [u8], inp: &[u8], inblocks: usize) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let st: &mut [u8; 72] = (&mut state[..72]).try_into().unwrap();
        sha512_inc_blocks(st, inp, inblocks);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let st: &mut [u8; 40] = (&mut state[..40]).try_into().unwrap();
        sha256_inc_blocks(st, inp, inblocks);
    }
}

fn shaX_inc_finalize_state(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let st: &mut [u8; 72] = (&mut state[..72]).try_into().unwrap();
        sha512_inc_finalize(out, st, inp, inlen);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let st: &mut [u8; 40] = (&mut state[..40]).try_into().unwrap();
        sha256_inc_finalize(out, st, inp, inlen);
    }
}

fn shaX(out: &mut [u8], inp: &[u8]) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    sha512(out, inp);
    #[cfg(any(feature = "128s", feature = "128f"))]
    sha256(out, inp);
}

pub fn hash_message_impl(
    digest: &mut [u8],
    R: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) -> (u64, u32) {
    let spx_tree_bits = SPX_TREE_HEIGHT * (SPX_D - 1);
    let spx_tree_bytes = (spx_tree_bits + 7) / 8;
    let spx_leaf_bits = SPX_TREE_HEIGHT;
    let spx_leaf_bytes = (spx_leaf_bits + 7) / 8;
    let spx_dgst_bytes = SPX_FORS_MSG_BYTES + spx_tree_bytes + spx_leaf_bytes;

    let mut seed = vec![0u8; 2 * SPX_N + SHAX_OUTPUT_BYTES];

    let inblocks = (SPX_N + SPX_PK_BYTES + SHAX_BLOCK_BYTES - 1) / SHAX_BLOCK_BYTES;
    let mut inbuf = vec![0u8; inblocks * SHAX_BLOCK_BYTES];

    let mut buf = vec![0u8; spx_dgst_bytes];

    let mut state = vec![0u8; 8 + SHAX_OUTPUT_BYTES];
    init_shaX(&mut state);

    inbuf[..SPX_N].copy_from_slice(R);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(pk);

    let mlen = m.len();
    if SPX_N + SPX_PK_BYTES + mlen < inblocks * SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(m);
        shaX_inc_finalize_state(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let needed = inblocks * SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + needed].copy_from_slice(&m[..needed]);
        shaX_inc_blocks_state(&mut state, &inbuf, inblocks);

        let m_rest = &m[needed..];
        let mlen_rest = mlen - needed;
        shaX_inc_finalize_state(&mut seed[2 * SPX_N..], &mut state, m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(R);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    mgf1_512(&mut buf, spx_dgst_bytes, &seed);
    #[cfg(any(feature = "128s", feature = "128f"))]
    mgf1_256(&mut buf, spx_dgst_bytes, &seed);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree = if SPX_D == 1 {
        0u64
    } else {
        let mut t = bytes_to_ull_rs(&buf[bufp..bufp + spx_tree_bytes]);
        t &= !0u64 >> (64 - spx_tree_bits);
        t
    };
    bufp += spx_tree_bytes;

    let mut leaf_idx = bytes_to_ull_rs(&buf[bufp..bufp + spx_leaf_bytes]) as u32;
    leaf_idx &= !0u32 >> (32 - spx_leaf_bits);

    (tree, leaf_idx)
}

pub fn thash_impl(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(feature = "robust")]
    {
        thash_robust(out, inp, inblocks, ctx, addr);
    }
    #[cfg(feature = "simple")]
    {
        thash_simple(out, inp, inblocks, ctx, addr);
    }
}

#[cfg(feature = "robust")]
fn thash_robust(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_robust_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut bitmask = vec![0u8; inblocks_us * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks_us * SPX_N];
    let mut sha2_state = ctx.state_seeded;

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    let buf_len_for_mgf = SPX_N + SPX_SHA256_ADDR_BYTES;
    let buf_clone: Vec<u8> = buf[..buf_len_for_mgf].to_vec();
    mgf1_256(&mut bitmask, inblocks_us * SPX_N, &buf_clone);

    for i in 0..inblocks_us * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let len = SPX_SHA256_ADDR_BYTES + inblocks_us * SPX_N;
    let inp_for_finalize: Vec<u8> = buf[SPX_N..SPX_N + len].to_vec();
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &inp_for_finalize, len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "robust", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_robust_512(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut bitmask = vec![0u8; inblocks_us * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks_us * SPX_N];
    let mut sha2_state = ctx.state_seeded_512;

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    let buf_clone: Vec<u8> = buf[..SPX_N + SPX_SHA256_ADDR_BYTES].to_vec();
    mgf1_512(&mut bitmask, inblocks_us * SPX_N, &buf_clone);

    for i in 0..inblocks_us * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let len = SPX_SHA256_ADDR_BYTES + inblocks_us * SPX_N;
    let inp_for_finalize: Vec<u8> = buf[SPX_N..SPX_N + len].to_vec();
    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &inp_for_finalize, len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "simple")]
fn thash_simple(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_simple_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut sha2_state = ctx.state_seeded;
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks_us * SPX_N];
    let addr_bytes = addr_to_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&inp[..inblocks_us * SPX_N]);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let len = buf.len();
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "simple", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_simple_512(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut sha2_state = ctx.state_seeded_512;
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks_us * SPX_N];
    let addr_bytes = addr_to_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&inp[..inblocks_us * SPX_N]);
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let len = buf.len();
    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// =========================
// FFI exports for the SHA2 functions used by the driver
// =========================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_init_c(state: *mut u8) {
    let s = unsafe { &mut *(state as *mut [u8; 40]) };
    sha256_inc_init(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_blocks_c(state: *mut u8, inp: *const u8, inblocks: usize) {
    let s = unsafe { &mut *(state as *mut [u8; 40]) };
    let in_slice = unsafe { core::slice::from_raw_parts(inp, 64 * inblocks) };
    sha256_inc_blocks(s, in_slice, inblocks);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_finalize_c(
    out: *mut u8,
    state: *mut u8,
    inp: *const u8,
    inlen: usize,
) {
    let s = unsafe { &mut *(state as *mut [u8; 40]) };
    let in_slice = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, 32) };
    sha256_inc_finalize(out_slice, s, in_slice, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_init_c(state: *mut u8) {
    let s = unsafe { &mut *(state as *mut [u8; 72]) };
    sha512_inc_init(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_blocks_c(state: *mut u8, inp: *const u8, inblocks: usize) {
    let s = unsafe { &mut *(state as *mut [u8; 72]) };
    let in_slice = unsafe { core::slice::from_raw_parts(inp, 128 * inblocks) };
    sha512_inc_blocks(s, in_slice, inblocks);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_finalize_c(
    out: *mut u8,
    state: *mut u8,
    inp: *const u8,
    inlen: usize,
) {
    let s = unsafe { &mut *(state as *mut [u8; 72]) };
    let in_slice = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, 64) };
    sha512_inc_finalize(out_slice, s, in_slice, inlen);
}

// We don't need to export sha256/sha512/mgf1 through C for the driver.
