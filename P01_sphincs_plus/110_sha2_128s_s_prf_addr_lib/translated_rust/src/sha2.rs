use crate::params::*;
use crate::address::*;
use crate::context::*;

fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

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

fn store_bigendian_32(x: &mut [u8], mut u: u64) {
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn store_bigendian_64(x: &mut [u8], mut u: u64) {
    x[7] = u as u8; u >>= 8;
    x[6] = u as u8; u >>= 8;
    x[5] = u as u8; u >>= 8;
    x[4] = u as u8; u >>= 8;
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn rotr32(x: u32, c: u32) -> u32 { x.rotate_right(c) }
fn rotr64(x: u64, c: u32) -> u64 { x.rotate_right(c) }

fn ch32(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (!x & z) }
fn maj32(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (x & z) ^ (y & z) }
fn sigma0_32(x: u32) -> u32 { rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22) }
fn sigma1_32(x: u32) -> u32 { rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25) }
fn lsigma0_32(x: u32) -> u32 { rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3) }
fn lsigma1_32(x: u32) -> u32 { rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10) }

fn ch64(x: u64, y: u64, z: u64) -> u64 { (x & y) ^ (!x & z) }
fn maj64(x: u64, y: u64, z: u64) -> u64 { (x & y) ^ (x & z) ^ (y & z) }
fn sigma0_64(x: u64) -> u64 { rotr64(x, 28) ^ rotr64(x, 34) ^ rotr64(x, 39) }
fn sigma1_64(x: u64) -> u64 { rotr64(x, 14) ^ rotr64(x, 18) ^ rotr64(x, 41) }
fn lsigma0_64(x: u64) -> u64 { rotr64(x, 1) ^ rotr64(x, 8) ^ (x >> 7) }
fn lsigma1_64(x: u64) -> u64 { rotr64(x, 19) ^ rotr64(x, 61) ^ (x >> 6) }

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
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

static IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85,
    0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c,
    0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

static IV_512: [u8; 64] = [
    0x6a, 0x09, 0xe6, 0x67, 0xf3, 0xbc, 0xc9, 0x08, 0xbb, 0x67, 0xae,
    0x85, 0x84, 0xca, 0xa7, 0x3b, 0x3c, 0x6e, 0xf3, 0x72, 0xfe, 0x94,
    0xf8, 0x2b, 0xa5, 0x4f, 0xf5, 0x3a, 0x5f, 0x1d, 0x36, 0xf1, 0x51,
    0x0e, 0x52, 0x7f, 0xad, 0xe6, 0x82, 0xd1, 0x9b, 0x05, 0x68, 0x8c,
    0x2b, 0x3e, 0x6c, 0x1f, 0x1f, 0x83, 0xd9, 0xab, 0xfb, 0x41, 0xbd,
    0x6b, 0x5b, 0xe0, 0xcd, 0x19, 0x13, 0x7e, 0x21, 0x79,
];

fn crypto_hashblocks_sha256(statebytes: &mut [u8], data: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[4 * i..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut pos = 0usize;
    while inlen >= 64 {
        let inp = &data[pos..];
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = load_bigendian_32(&inp[4 * i..]);
        }

        // 64 rounds
        for round in 0..64 {
            if round >= 16 {
                w[round & 15] = lsigma1_32(w[(round + 14) & 15])
                    .wrapping_add(w[(round + 9) & 15])
                    .wrapping_add(lsigma0_32(w[(round + 1) & 15]))
                    .wrapping_add(w[round & 15]);
            }
            let t1 = h
                .wrapping_add(sigma1_32(e))
                .wrapping_add(ch32(e, f, g))
                .wrapping_add(K256[round])
                .wrapping_add(w[round & 15]);
            let t2 = sigma0_32(a).wrapping_add(maj32(a, b, c));
            h = g; g = f; f = e;
            e = d.wrapping_add(t1);
            d = c; c = b; b = a;
            a = t1.wrapping_add(t2);
        }

        a = a.wrapping_add(state[0]); state[0] = a;
        b = b.wrapping_add(state[1]); state[1] = b;
        c = c.wrapping_add(state[2]); state[2] = c;
        d = d.wrapping_add(state[3]); state[3] = d;
        e = e.wrapping_add(state[4]); state[4] = e;
        f = f.wrapping_add(state[5]); state[5] = f;
        g = g.wrapping_add(state[6]); state[6] = g;
        h = h.wrapping_add(state[7]); state[7] = h;

        pos += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[4 * i..], state[i] as u64);
    }
    inlen
}

fn crypto_hashblocks_sha512(statebytes: &mut [u8], data: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u64; 8];
    for i in 0..8 {
        state[i] = load_bigendian_64(&statebytes[8 * i..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut pos = 0usize;
    while inlen >= 128 {
        let inp = &data[pos..];
        let mut w = [0u64; 16];
        for i in 0..16 {
            w[i] = load_bigendian_64(&inp[8 * i..]);
        }

        for round in 0..80 {
            if round >= 16 {
                w[round & 15] = lsigma1_64(w[(round + 14) & 15])
                    .wrapping_add(w[(round + 9) & 15])
                    .wrapping_add(lsigma0_64(w[(round + 1) & 15]))
                    .wrapping_add(w[round & 15]);
            }
            let t1 = h
                .wrapping_add(sigma1_64(e))
                .wrapping_add(ch64(e, f, g))
                .wrapping_add(K512[round])
                .wrapping_add(w[round & 15]);
            let t2 = sigma0_64(a).wrapping_add(maj64(a, b, c));
            h = g; g = f; f = e;
            e = d.wrapping_add(t1);
            d = c; c = b; b = a;
            a = t1.wrapping_add(t2);
        }

        a = a.wrapping_add(state[0]); state[0] = a;
        b = b.wrapping_add(state[1]); state[1] = b;
        c = c.wrapping_add(state[2]); state[2] = c;
        d = d.wrapping_add(state[3]); state[3] = d;
        e = e.wrapping_add(state[4]); state[4] = e;
        f = f.wrapping_add(state[5]); state[5] = f;
        g = g.wrapping_add(state[6]); state[6] = g;
        h = h.wrapping_add(state[7]); state[7] = h;

        pos += 128;
        inlen -= 128;
    }

    for i in 0..8 {
        store_bigendian_64(&mut statebytes[8 * i..], state[i]);
    }
    inlen
}

pub fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 { state[i] = 0; }
}

pub fn sha512_inc_init(state: &mut [u8]) {
    state[..64].copy_from_slice(&IV_512);
    for i in 64..72 { state[i] = 0; }
}

pub fn sha256_inc_blocks(state: &mut [u8], data: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, data, 64 * inblocks);
    bytes += (64 * inblocks) as u64;
    store_bigendian_64(&mut state[32..], bytes);
}

pub fn sha512_inc_blocks(state: &mut [u8], data: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[64..]);
    crypto_hashblocks_sha512(state, data, 128 * inblocks);
    bytes += (128 * inblocks) as u64;
    store_bigendian_64(&mut state[64..], bytes);
}

pub fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]).wrapping_add(inlen as u64);

    crypto_hashblocks_sha256(state, data, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;

    padded[..remaining].copy_from_slice(&data[start..start + remaining]);
    padded[remaining] = 0x80;

    if remaining < 56 {
        for i in remaining + 1..56 { padded[i] = 0; }
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
        for i in remaining + 1..120 { padded[i] = 0; }
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

pub fn sha512_inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) {
    let mut padded = [0u8; 256];
    let bytes = load_bigendian_64(&state[64..]).wrapping_add(inlen as u64);

    crypto_hashblocks_sha512(state, data, inlen);
    let remaining = inlen & 127;
    let start = inlen - remaining;

    padded[..remaining].copy_from_slice(&data[start..start + remaining]);
    padded[remaining] = 0x80;

    if remaining < 112 {
        for i in remaining + 1..119 { padded[i] = 0; }
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
        for i in remaining + 1..247 { padded[i] = 0; }
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

pub fn sha256(out: &mut [u8], data: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, data, inlen);
}

pub fn sha512(out: &mut [u8], data: &[u8], inlen: usize) {
    let mut state = [0u8; 72];
    sha512_inc_init(&mut state);
    sha512_inc_finalize(out, &mut state, data, inlen);
}

pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut i = 0u32;
    let mut off = 0usize;
    while (i as usize + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha256(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i as usize * SPX_SHA256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub fn mgf1_512(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut i = 0u32;
    let mut off = 0usize;
    while (i as usize + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha512(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_SHA512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha512(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i as usize * SPX_SHA512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest already zero

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}
