// SPHINCS+ SHA2 backend — translated from C (sha2.c, hash_sha2.c, thash_sha2_simple.c, thash_sha2_robust.c)
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::{u32_to_bytes, bytes_to_ull};

// ============ sha2.c helpers ============

fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64) | ((x[6] as u64) << 8) | ((x[5] as u64) << 16) | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32) | ((x[2] as u64) << 40) | ((x[1] as u64) << 48) | ((x[0] as u64) << 56)
}

fn store_bigendian_32(x: &mut [u8], mut u: u32) {
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

// ============ SHA-256 compression ============

#[inline(always)] fn rotr32(x: u32, c: u32) -> u32 { (x >> c) | (x << (32 - c)) }
#[inline(always)] fn ch32(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (!x & z) }
#[inline(always)] fn maj32(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (x & z) ^ (y & z) }
#[inline(always)] fn sigma0_32(x: u32) -> u32 { rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22) }
#[inline(always)] fn sigma1_32(x: u32) -> u32 { rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25) }
#[inline(always)] fn lsigma0_32(x: u32) -> u32 { rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3) }
#[inline(always)] fn lsigma1_32(x: u32) -> u32 { rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10) }

fn crypto_hashblocks_sha256(statebytes: &mut [u8], data: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[4 * i..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h);
    a = state[0]; b = state[1]; c = state[2]; d = state[3];
    e = state[4]; f = state[5]; g = state[6]; h = state[7];

    let mut off = 0usize;
    while inlen >= 64 {
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = load_bigendian_32(&data[off + 4 * i..]);
        }

        macro_rules! f32_round {
            ($w:expr, $k:expr) => {
                let t1 = h.wrapping_add(sigma1_32(e)).wrapping_add(ch32(e, f, g)).wrapping_add($k).wrapping_add($w);
                let t2 = sigma0_32(a).wrapping_add(maj32(a, b, c));
                h = g; g = f; f = e; e = d.wrapping_add(t1);
                d = c; c = b; b = a; a = t1.wrapping_add(t2);
            }
        }

        macro_rules! expand32 {
            () => {
                for i in 0..16 {
                    w[i] = w[i].wrapping_add(lsigma1_32(w[(i + 14) % 16]))
                        .wrapping_add(w[(i + 9) % 16])
                        .wrapping_add(lsigma0_32(w[(i + 1) % 16]));
                }
            }
        }

        static K256: [u32; 64] = [
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

        for i in 0..16 { f32_round!(w[i], K256[i]); }
        expand32!();
        for i in 0..16 { f32_round!(w[i], K256[16 + i]); }
        expand32!();
        for i in 0..16 { f32_round!(w[i], K256[32 + i]); }
        expand32!();
        for i in 0..16 { f32_round!(w[i], K256[48 + i]); }

        a = a.wrapping_add(state[0]); b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]); d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]); f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]); h = h.wrapping_add(state[7]);

        state[0] = a; state[1] = b; state[2] = c; state[3] = d;
        state[4] = e; state[5] = f; state[6] = g; state[7] = h;

        off += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[4 * i..], state[i]);
    }
    inlen
}

// ============ SHA-512 compression ============

#[inline(always)] fn rotr64(x: u64, c: u32) -> u64 { (x >> c) | (x << (64 - c)) }
#[inline(always)] fn ch64(x: u64, y: u64, z: u64) -> u64 { (x & y) ^ (!x & z) }
#[inline(always)] fn maj64(x: u64, y: u64, z: u64) -> u64 { (x & y) ^ (x & z) ^ (y & z) }
#[inline(always)] fn sigma0_64(x: u64) -> u64 { rotr64(x, 28) ^ rotr64(x, 34) ^ rotr64(x, 39) }
#[inline(always)] fn sigma1_64(x: u64) -> u64 { rotr64(x, 14) ^ rotr64(x, 18) ^ rotr64(x, 41) }
#[inline(always)] fn lsigma0_64(x: u64) -> u64 { rotr64(x, 1) ^ rotr64(x, 8) ^ (x >> 7) }
#[inline(always)] fn lsigma1_64(x: u64) -> u64 { rotr64(x, 19) ^ rotr64(x, 61) ^ (x >> 6) }

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

fn crypto_hashblocks_sha512(statebytes: &mut [u8], data: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u64; 8];
    for i in 0..8 {
        state[i] = load_bigendian_64(&statebytes[8 * i..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h);
    a = state[0]; b = state[1]; c = state[2]; d = state[3];
    e = state[4]; f = state[5]; g = state[6]; h = state[7];

    let mut off = 0usize;
    while inlen >= 128 {
        let mut w = [0u64; 16];
        for i in 0..16 {
            w[i] = load_bigendian_64(&data[off + 8 * i..]);
        }

        // 80 rounds: first 16 use w directly, then expand every 16
        for round in 0..80 {
            if round > 0 && round % 16 == 0 {
                for i in 0..16 {
                    w[i] = w[i].wrapping_add(lsigma1_64(w[(i + 14) % 16]))
                        .wrapping_add(w[(i + 9) % 16])
                        .wrapping_add(lsigma0_64(w[(i + 1) % 16]));
                }
            }
            let t1 = h.wrapping_add(sigma1_64(e)).wrapping_add(ch64(e, f, g))
                .wrapping_add(K512[round]).wrapping_add(w[round % 16]);
            let t2 = sigma0_64(a).wrapping_add(maj64(a, b, c));
            h = g; g = f; f = e; e = d.wrapping_add(t1);
            d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }

        a = a.wrapping_add(state[0]); b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]); d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]); f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]); h = h.wrapping_add(state[7]);

        state[0] = a; state[1] = b; state[2] = c; state[3] = d;
        state[4] = e; state[5] = f; state[6] = g; state[7] = h;

        off += 128;
        inlen -= 128;
    }

    for i in 0..8 {
        store_bigendian_64(&mut statebytes[8 * i..], state[i]);
    }
    inlen
}

// ============ SHA-256/512 incremental API ============

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

fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 { state[i] = 0; }
}

fn sha512_inc_init(state: &mut [u8]) {
    state[..64].copy_from_slice(&IV_512);
    for i in 64..72 { state[i] = 0; }
}

fn sha256_inc_blocks(state: &mut [u8], data: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, data, 64 * inblocks);
    bytes += (64 * inblocks) as u64;
    store_bigendian_64(&mut state[32..], bytes);
}

fn sha512_inc_blocks(state: &mut [u8], data: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[64..]);
    crypto_hashblocks_sha512(state, data, 128 * inblocks);
    bytes += (128 * inblocks) as u64;
    store_bigendian_64(&mut state[64..], bytes);
}

fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]) + inlen as u64;

    crypto_hashblocks_sha256(state, data, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;

    padded[..remaining].copy_from_slice(&data[start..start + remaining]);
    padded[remaining] = 0x80;

    if remaining < 56 {
        for i in (remaining + 1)..56 { padded[i] = 0; }
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
        for i in (remaining + 1)..120 { padded[i] = 0; }
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

fn sha512_inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) {
    let mut padded = [0u8; 256];
    let bytes = load_bigendian_64(&state[64..]) + inlen as u64;

    crypto_hashblocks_sha512(state, data, inlen);
    let remaining = inlen & 127;
    let start = inlen - remaining;

    padded[..remaining].copy_from_slice(&data[start..start + remaining]);
    padded[remaining] = 0x80;

    if remaining < 112 {
        for i in (remaining + 1)..119 { padded[i] = 0; }
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
        for i in (remaining + 1)..247 { padded[i] = 0; }
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

fn sha256(out: &mut [u8], data: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, data, inlen);
}

fn sha512(out: &mut [u8], data: &[u8], inlen: usize) {
    let mut state = [0u8; 72];
    sha512_inc_init(&mut state);
    sha512_inc_finalize(out, &mut state, data, inlen);
}

// ============ MGF1 ============

fn mgf1_256(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut i = 0u32;
    let mut off = 0usize;
    while (i as usize + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha256(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_SHA256_OUTPUT_BYTES {
        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i as usize * SPX_SHA256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

fn mgf1_512(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut i = 0u32;
    let mut off = 0usize;
    while (i as usize + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha512(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_SHA512_OUTPUT_BYTES {
        let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha512(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i as usize * SPX_SHA512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============ seed_state ============

fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block[..64], 1);
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        sha512_inc_init(&mut ctx.state_seeded_512);
        sha512_inc_blocks(&mut ctx.state_seeded_512, &block, 1);
    }
}

// ============ hash_sha2.c ============

fn addr_bytes(addr: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) }
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    // Determine SHA variant sizes at compile time
    const SHAX_OUTPUT: usize = if SPX_SHA512 { SPX_SHA512_OUTPUT_BYTES } else { SPX_SHA256_OUTPUT_BYTES };
    const SHAX_BLOCK: usize = if SPX_SHA512 { SPX_SHA512_BLOCK_BYTES } else { SPX_SHA256_BLOCK_BYTES };
    const STATE_SIZE: usize = if SPX_SHA512 { 72 } else { 40 };

    let mlen = mlen as usize;
    let mut buf = vec![0u8; SHAX_BLOCK + SHAX_OUTPUT];
    let mut state = vec![0u8; STATE_SIZE];

    // HMAC inner: ipad key
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK {
        buf[i] = 0x36;
    }

    if SPX_SHA512 {
        sha512_inc_init(&mut state);
        sha512_inc_blocks(&mut state, &buf[..SHAX_BLOCK], 1);
    } else {
        sha256_inc_init(&mut state);
        sha256_inc_blocks(&mut state, &buf[..SHAX_BLOCK], 1);
    }

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < SHAX_BLOCK {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let tmp = buf[..SPX_N + mlen].to_vec();
        if SPX_SHA512 {
            sha512_inc_finalize(&mut buf[SHAX_BLOCK..], &mut state, &tmp, SPX_N + mlen);
        } else {
            sha256_inc_finalize(&mut buf[SHAX_BLOCK..], &mut state, &tmp, SPX_N + mlen);
        }
    } else {
        buf[SPX_N..SHAX_BLOCK].copy_from_slice(&m[..SHAX_BLOCK - SPX_N]);
        if SPX_SHA512 {
            sha512_inc_blocks(&mut state, &buf[..SHAX_BLOCK], 1);
            let m_rest = &m[SHAX_BLOCK - SPX_N..];
            let mlen_rest = mlen - (SHAX_BLOCK - SPX_N);
            sha512_inc_finalize(&mut buf[SHAX_BLOCK..], &mut state, m_rest, mlen_rest);
        } else {
            sha256_inc_blocks(&mut state, &buf[..SHAX_BLOCK], 1);
            let m_rest = &m[SHAX_BLOCK - SPX_N..];
            let mlen_rest = mlen - (SHAX_BLOCK - SPX_N);
            sha256_inc_finalize(&mut buf[SHAX_BLOCK..], &mut state, m_rest, mlen_rest);
        }
    }

    // HMAC outer: opad key
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK {
        buf[i] = 0x5c;
    }

    if SPX_SHA512 {
        let tmp = buf.clone();
        sha512(&mut buf, &tmp, SHAX_BLOCK + SHAX_OUTPUT);
    } else {
        let tmp = buf.clone();
        sha256(&mut buf, &tmp, SHAX_BLOCK + SHAX_OUTPUT);
    }
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    const SHAX_OUTPUT: usize = if SPX_SHA512 { SPX_SHA512_OUTPUT_BYTES } else { SPX_SHA256_OUTPUT_BYTES };
    const SHAX_BLOCK: usize = if SPX_SHA512 { SPX_SHA512_BLOCK_BYTES } else { SPX_SHA256_BLOCK_BYTES };
    const STATE_SIZE: usize = if SPX_SHA512 { 72 } else { 40 };
    const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SHAX_BLOCK - 1) & (0usize.wrapping_sub(SHAX_BLOCK))) / SHAX_BLOCK;

    let mlen = mlen as usize;
    let mut seed = vec![0u8; 2 * SPX_N + SHAX_OUTPUT];
    let mut inbuf = vec![0u8; SPX_INBLOCKS * SHAX_BLOCK];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = vec![0u8; STATE_SIZE];

    if SPX_SHA512 {
        sha512_inc_init(&mut state);
    } else {
        sha256_inc_init(&mut state);
    }

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SHAX_BLOCK {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        if SPX_SHA512 {
            sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
        } else {
            sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
        }
    } else {
        let fill = SPX_INBLOCKS * SHAX_BLOCK - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        if SPX_SHA512 {
            sha512_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
            sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[fill..], mlen - fill);
        } else {
            sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
            sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[fill..], mlen - fill);
        }
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    if SPX_SHA512 {
        mgf1_512(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SHAX_OUTPUT);
    } else {
        mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SHAX_OUTPUT);
    }

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ============ thash ============

#[cfg(all(feature = "simple", not(feature = "robust")))]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let ab = addr_bytes(addr);

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if SPX_SHA512 && inblocks > 1 {
            let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
            let mut sha2_state = [0u8; 72];
            sha2_state.copy_from_slice(&ctx.state_seeded_512);
            let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
            buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
            buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
                .copy_from_slice(&input[..inblocks * SPX_N]);
            sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
            out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
            return;
        }
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&input[..inblocks * SPX_N]);
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "robust", not(feature = "simple")))]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let ab = addr_bytes(addr);

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if SPX_SHA512 && inblocks > 1 {
            let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
            let mut bitmask = vec![0u8; inblocks * SPX_N];
            let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
            buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
            buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
            mgf1_512(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES], SPX_N + SPX_SHA256_ADDR_BYTES);

            let mut sha2_state = [0u8; 72];
            sha2_state.copy_from_slice(&ctx.state_seeded_512);
            for i in 0..inblocks * SPX_N {
                buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
            }
            sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..],
                                SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
            out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
            return;
        }
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES], SPX_N + SPX_SHA256_ADDR_BYTES);

    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..],
                        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// FFI wrappers
pub fn sha256_inc_init_ffi(state: &mut [u8]) { sha256_inc_init(state); }
pub fn sha256_inc_blocks_ffi(state: &mut [u8], data: &[u8], inblocks: usize) { sha256_inc_blocks(state, data, inblocks); }
pub fn sha256_inc_finalize_ffi(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) { sha256_inc_finalize(out, state, data, inlen); }
pub fn sha256_ffi(out: &mut [u8], data: &[u8], inlen: usize) { sha256(out, data, inlen); }
pub fn sha512_inc_init_ffi(state: &mut [u8]) { sha512_inc_init(state); }
pub fn sha512_inc_blocks_ffi(state: &mut [u8], data: &[u8], inblocks: usize) { sha512_inc_blocks(state, data, inblocks); }
pub fn sha512_inc_finalize_ffi(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) { sha512_inc_finalize(out, state, data, inlen); }
pub fn sha512_ffi(out: &mut [u8], data: &[u8], inlen: usize) { sha512(out, data, inlen); }
pub fn mgf1_256_ffi(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) { mgf1_256(out, outlen, input, inlen); }
pub fn mgf1_512_ffi(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) { mgf1_512(out, outlen, input, inlen); }
pub fn seed_state_ffi(ctx: &mut SpxCtx) { seed_state(ctx); }
