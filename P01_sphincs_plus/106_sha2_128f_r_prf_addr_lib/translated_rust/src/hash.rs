use crate::params::*;
use crate::address::*;

// SHA-256 implementation

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
    (x[7] as u64)
        | ((x[6] as u64) << 8)
        | ((x[5] as u64) << 16)
        | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32)
        | ((x[2] as u64) << 40)
        | ((x[1] as u64) << 48)
        | ((x[0] as u64) << 56)
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

#[inline(always)] fn rotr32(x: u32, c: u32) -> u32 { x.rotate_right(c) }
#[inline(always)] fn ch(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (!x & z) }
#[inline(always)] fn maj(x: u32, y: u32, z: u32) -> u32 { (x & y) ^ (x & z) ^ (y & z) }
#[inline(always)] fn sigma0_32(x: u32) -> u32 { rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22) }
#[inline(always)] fn sigma1_32(x: u32) -> u32 { rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25) }
#[inline(always)] fn lsigma0_32(x: u32) -> u32 { rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3) }
#[inline(always)] fn lsigma1_32(x: u32) -> u32 { rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10) }

static SHA256_K: [u32; 64] = [
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

fn crypto_hashblocks_sha256(statebytes: &mut [u8], inp: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[i * 4..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut pos = 0usize;
    while inlen >= 64 {
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = load_bigendian_32(&inp[pos + i * 4..]);
        }

        for round in 0..4 {
            for i in 0..16 {
                let ki = round * 16 + i;
                let t1 = h.wrapping_add(sigma1_32(e))
                    .wrapping_add(ch(e, f, g))
                    .wrapping_add(SHA256_K[ki])
                    .wrapping_add(w[i]);
                let t2 = sigma0_32(a).wrapping_add(maj(a, b, c));
                h = g; g = f; f = e;
                e = d.wrapping_add(t1);
                d = c; c = b; b = a;
                a = t1.wrapping_add(t2);
            }
            if round < 3 {
                // EXPAND
                for i in 0..16 {
                    w[i] = w[i]
                        .wrapping_add(lsigma1_32(w[(i + 14) % 16]))
                        .wrapping_add(w[(i + 9) % 16])
                        .wrapping_add(lsigma0_32(w[(i + 1) % 16]));
                }
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

        state[0] = a; state[1] = b; state[2] = c; state[3] = d;
        state[4] = e; state[5] = f; state[6] = g; state[7] = h;

        pos += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[i * 4..], state[i]);
    }
    inlen
}

static IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85,
    0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c,
    0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

pub fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 {
        state[i] = 0;
    }
}

pub fn sha256_inc_blocks(state: &mut [u8], inp: &[u8], inblocks: usize) {
    let bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, inp, 64 * inblocks);
    store_bigendian_64(&mut state[32..], bytes + (64 * inblocks) as u64);
}

pub fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]) + inlen as u64;

    crypto_hashblocks_sha256(state, inp, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;

    padded[..remaining].copy_from_slice(&inp[start..start + remaining]);
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

pub fn sha256(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, inp, inlen);
}

pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut pos = 0usize;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[pos..], &inbuf, inlen + 4);
        pos += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        out[pos..pos + outlen - i * SPX_SHA256_OUTPUT_BYTES]
            .copy_from_slice(&outbuf[..outlen - i * SPX_SHA256_OUTPUT_BYTES]);
    }
}

pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA256_BLOCK_BYTES]; // 64 bytes is enough, SPX_SHA512=0
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest already zero

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N]; // 22 + 16 = 38
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes = unsafe {
        core::slice::from_raw_parts(addr as *const [u32; 8] as *const u8, 32)
    };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N]
        .copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: usize,
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES]; // 64+32=96
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES]; // 40

    // HMAC-SHA256
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    sha256_inc_init(&mut state);
    sha256_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let tmp = buf;
        sha256_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &tmp[..SPX_N + mlen],
            mlen + SPX_N,
        );
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha256_inc_blocks(&mut state, &buf, 1);

        let m_off = SPX_SHAX_BLOCK_BYTES - SPX_N;
        let mlen_rem = mlen - m_off;
        sha256_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &m[m_off..],
            mlen_rem,
        );
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    sha256(&mut buf.clone(), &buf, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r_val: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: usize,
    _ctx: &SpxCtx,
) {
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES]; // 32+32=64
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES]; // 64
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES]; // 40

    sha256_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    let total_header = SPX_N + SPX_PK_BYTES;
    let block_size = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES;

    if total_header + mlen < block_size {
        inbuf[total_header..total_header + mlen].copy_from_slice(&m[..mlen]);
        sha256_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            total_header + mlen,
        );
    } else {
        let fill = block_size - total_header;
        inbuf[total_header..block_size].copy_from_slice(&m[..fill]);
        sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        sha256_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &m[fill..],
            mlen - fill,
        );
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

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
