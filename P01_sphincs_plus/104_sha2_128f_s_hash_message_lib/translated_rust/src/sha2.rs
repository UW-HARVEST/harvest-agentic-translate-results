use crate::params::*;

static IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85,
    0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c,
    0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

fn store_bigendian_32(x: &mut [u8], u: u32) {
    x[3] = u as u8;
    x[2] = (u >> 8) as u8;
    x[1] = (u >> 16) as u8;
    x[0] = (u >> 24) as u8;
}

fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64) | ((x[6] as u64) << 8) | ((x[5] as u64) << 16) | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32) | ((x[2] as u64) << 40) | ((x[1] as u64) << 48) | ((x[0] as u64) << 56)
}

fn store_bigendian_64(x: &mut [u8], u: u64) {
    x[7] = u as u8;
    x[6] = (u >> 8) as u8;
    x[5] = (u >> 16) as u8;
    x[4] = (u >> 24) as u8;
    x[3] = (u >> 32) as u8;
    x[2] = (u >> 40) as u8;
    x[1] = (u >> 48) as u8;
    x[0] = (u >> 56) as u8;
}

fn crypto_hashblocks_sha256(statebytes: &mut [u8], in_data: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[i * 4..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut pos = 0usize;
    while inlen >= 64 {
        let inp = &in_data[pos..];
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = load_bigendian_32(&inp[i * 4..]);
        }

        macro_rules! ch { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ (!$x & $z) } }
        macro_rules! maj { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ ($x & $z) ^ ($y & $z) } }
        macro_rules! sigma0 { ($x:expr) => { $x.rotate_right(2) ^ $x.rotate_right(13) ^ $x.rotate_right(22) } }
        macro_rules! sigma1 { ($x:expr) => { $x.rotate_right(6) ^ $x.rotate_right(11) ^ $x.rotate_right(25) } }
        macro_rules! lsigma0 { ($x:expr) => { $x.rotate_right(7) ^ $x.rotate_right(18) ^ ($x >> 3) } }
        macro_rules! lsigma1 { ($x:expr) => { $x.rotate_right(17) ^ $x.rotate_right(19) ^ ($x >> 10) } }

        macro_rules! f32_round {
            ($w:expr, $k:expr) => {
                let t1 = h.wrapping_add(sigma1!(e)).wrapping_add(ch!(e, f, g)).wrapping_add($k).wrapping_add($w);
                let t2 = sigma0!(a).wrapping_add(maj!(a, b, c));
                h = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
            }
        }

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

        for round in 0..4 {
            if round > 0 {
                // EXPAND
                for i in 0..16 {
                    w[i] = lsigma1!(w[(i + 14) % 16])
                        .wrapping_add(w[(i + 9) % 16])
                        .wrapping_add(lsigma0!(w[(i + 1) % 16]))
                        .wrapping_add(w[i]);
                }
            }
            for i in 0..16 {
                f32_round!(w[i], K256[round * 16 + i]);
            }
        }

        a = a.wrapping_add(state[0]);
        b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]);
        d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]);
        f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]);
        h = h.wrapping_add(state[7]);

        state = [a, b, c, d, e, f, g, h];

        pos += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[i * 4..], state[i]);
    }
    inlen
}

pub fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 {
        state[i] = 0;
    }
}

pub fn sha256_inc_blocks(state: &mut [u8], in_data: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, in_data, 64 * inblocks);
    bytes += (64 * inblocks) as u64;
    store_bigendian_64(&mut state[32..], bytes);
}

pub fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], in_data: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]) + inlen as u64;

    crypto_hashblocks_sha256(state, in_data, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;

    padded[..remaining].copy_from_slice(&in_data[start..start + remaining]);
    padded[remaining] = 0x80;

    if remaining < 56 {
        for i in remaining + 1..56 {
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
        for i in remaining + 1..120 {
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

pub fn sha256(out: &mut [u8], in_data: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, in_data, inlen);
}

pub fn mgf1_256(out: &mut [u8], outlen: usize, in_data: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&in_data[..inlen]);

    let mut i = 0u64;
    let mut offset = 0;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES as u64 <= outlen as u64 {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[offset..], &inbuf, inlen + 4);
        offset += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_SHA256_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i as usize * SPX_SHA256_OUTPUT_BYTES;
        out[offset..offset + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub fn seed_state(ctx: &mut crate::context::SpxCtx) {
    let mut block = [0u8; 128]; // SPX_SHA512_BLOCK_BYTES, but we only use 64
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest is already zero

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}
